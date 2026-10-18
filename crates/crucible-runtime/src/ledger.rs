//! Integer accounting: cumulative time/I/O and concurrent occupancy are separate.
use crate::{Result, RuntimeError};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resources {
    pub cpu_ms: u64,
    pub gpu_ms: u64,
    pub ram_bytes: u64,
    pub vram_bytes: u64,
    pub scratch_bytes: u64,
    pub io_bytes: u64,
}
impl Resources {
    pub fn check(&self) -> Result<()> {
        if self.values().iter().any(|v| *v > 2_u64.pow(53)) {
            return Err(RuntimeError::Invalid("resource_integer_range"));
        }
        Ok(())
    }
    pub fn values(self) -> [u64; 6] {
        [
            self.cpu_ms,
            self.gpu_ms,
            self.ram_bytes,
            self.vram_bytes,
            self.scratch_bytes,
            self.io_bytes,
        ]
    }
    pub fn from_values(v: [u64; 6]) -> Self {
        Self {
            cpu_ms: v[0],
            gpu_ms: v[1],
            ram_bytes: v[2],
            vram_bytes: v[3],
            scratch_bytes: v[4],
            io_bytes: v[5],
        }
    }
    pub fn checked_add(self, b: Self) -> Result<Self> {
        let mut a = self.values();
        for (a, b) in a.iter_mut().zip(b.values()) {
            *a = a
                .checked_add(b)
                .ok_or(RuntimeError::Invalid("resource_overflow"))?;
        }
        let result = Self::from_values(a);
        result.check()?;
        Ok(result)
    }
    pub fn fits(self, b: Self) -> bool {
        self.values().iter().zip(b.values()).all(|(a, b)| *a <= b)
    }
    pub fn cumulative(self) -> Self {
        Self {
            cpu_ms: self.cpu_ms,
            gpu_ms: self.gpu_ms,
            io_bytes: self.io_bytes,
            ..Self::default()
        }
    }
    pub fn occupancy(self) -> Self {
        Self {
            ram_bytes: self.ram_bytes,
            vram_bytes: self.vram_bytes,
            scratch_bytes: self.scratch_bytes,
            ..Self::default()
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkClass {
    Evaluation,
    Retry,
    CacheLookup,
    Audit,
    SurrogateFit,
    Checkpoint,
    Admission,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Charge {
    pub measured: Resources,
    pub wall_ns: u64,
    pub accounting_basis: String,
}
impl Charge {
    pub fn check(&self) -> Result<()> {
        self.measured.check()?;
        if self.wall_ns > 2_u64.pow(53) || self.accounting_basis.trim().is_empty() {
            return Err(RuntimeError::Invalid("charge_basis_or_range"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Accounting {
    pub authorized: Resources,
    pub spent: Resources,
    pub reserved: Resources,
    pub charges: u64,
    pub exceeded: bool,
}
impl Accounting {
    pub fn may_reserve(&self, request: Resources) -> Result<()> {
        request.check()?;
        if self.exceeded
            || !self
                .spent
                .cumulative()
                .checked_add(self.reserved)?
                .checked_add(request)?
                .fits(self.authorized)
        {
            return Err(RuntimeError::Budget("remaining_authorized_vector"));
        }
        Ok(())
    }
}

/// Decimal alpha bookkeeping has a fixed trillionth grid. Unsupported precision
/// is rejected; binary rounding never increases an admitted error allowance.
pub fn alpha_units(value: joil_evidence::observation::Finite) -> Result<u64> {
    let value = value.get();
    if !(0.0 < value && value < 1.0) {
        return Err(RuntimeError::Invalid("alpha_range"));
    }
    let text = value.to_string();
    let (mantissa, exponent) = match text.split_once(['e', 'E']) {
        Some((m, e)) => (
            m,
            e.parse::<i32>()
                .map_err(|_| RuntimeError::Invalid("alpha_decimal"))?,
        ),
        None => (text.as_str(), 0),
    };
    let fractional = mantissa.split_once('.').map_or(0, |(_, f)| f.len()) as i32;
    let digits = mantissa
        .replace('.', "")
        .parse::<u64>()
        .map_err(|_| RuntimeError::Invalid("alpha_decimal"))?;
    let power = 12 + exponent - fractional;
    if !(0..=12).contains(&power) {
        return Err(RuntimeError::Unsupported(
            "alpha_precision_finer_than_trillionth",
        ));
    }
    digits
        .checked_mul(10_u64.pow(power as u32))
        .filter(|n| *n > 0 && *n < 1_000_000_000_000)
        .ok_or(RuntimeError::Invalid("alpha_decimal_range"))
}
use crate::{
    db::{conflict, event, RuntimeDb},
    decode, encode, SemanticId,
};
use joil_evidence::{
    claims::Registry,
    decision::{DecisionProfile, FamilyPolicy, RequirementSet},
    identity::ContentRef,
};
use rusqlite::{params, OptionalExtension, TransactionBehavior};
/// Reviewed policy identity, not a physical/statistical qualification token.
pub struct ApprovedFamily {
    reference: ContentRef,
    requirements: ContentRef,
    policy: FamilyPolicy,
    alpha: u64,
}
impl ApprovedFamily {
    pub fn from_registry(
        registry: &Registry,
        reference: ContentRef,
        requirements: &ContentRef,
    ) -> Result<Self> {
        let bytes = registry
            .checked_artifact(&reference, std::slice::from_ref(requirements))
            .map_err(RuntimeError::Wire)?;
        let policy: FamilyPolicy =
            joil_evidence::identity::strict_decode(bytes).map_err(RuntimeError::Wire)?;
        joil_evidence::identity::check_id(&policy.experiment_family_id)
            .map_err(RuntimeError::Wire)?;
        if policy.requirements != *requirements || policy.max_attempts == 0 {
            return Err(RuntimeError::Invalid("family_scope_or_attempt_limit"));
        }
        let alpha = alpha_units(policy.family_alpha)?;
        if alpha < u64::from(policy.max_attempts) {
            return Err(RuntimeError::Unsupported("family_attempt_grid_too_fine"));
        }
        Ok(Self {
            reference,
            requirements: requirements.clone(),
            policy,
            alpha,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FamilySpend {
    pub family: String,
    pub attempts: u32,
    pub spent_alpha_units: u64,
    pub maximum_alpha_units: u64,
    pub policy: ContentRef,
}
impl RuntimeDb {
    /// Reserve the full frozen per-attempt allotment before qualification work.
    /// Reusing the same ordinal/decision is idempotent; changing its decision
    /// cannot erase spend. Forks use this same coordinator database.
    pub fn reserve_family_attempt(
        &mut self,
        family: &ApprovedFamily,
        registry: &Registry,
        ordinal: u32,
        decision: &SemanticId,
    ) -> Result<FamilySpend> {
        let bytes = registry
            .checked_artifact(
                &family.reference,
                std::slice::from_ref(&family.requirements),
            )
            .map_err(RuntimeError::Wire)?;
        if joil_evidence::identity::strict_decode::<FamilyPolicy>(bytes)
            .map_err(RuntimeError::Wire)?
            != family.policy
        {
            return Err(RuntimeError::Conflict("family_policy_changed"));
        }
        let tx = self
            .connection()
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing: Option<(String, String, String)> = tx
            .query_row(
                "SELECT source,requirements,policy FROM family_policies WHERE id=?1",
                [&family.policy.experiment_family_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;
        if let Some((source, requirements, policy)) = existing {
            if decode::<ContentRef>(&source)? != family.reference
                || decode::<ContentRef>(&requirements)? != family.requirements
                || decode::<FamilyPolicy>(&policy)? != family.policy
            {
                conflict(
                    &tx,
                    &family.policy.experiment_family_id,
                    "family_reset_refused",
                    &family.reference,
                )?;
                tx.commit()?;
                return Err(RuntimeError::Conflict("family_reset_refused"));
            }
        } else {
            tx.execute("INSERT INTO family_policies(id,source,requirements,policy,alpha_units,max_attempts) VALUES(?1,?2,?3,?4,?5,?6)",params![family.policy.experiment_family_id,encode(&family.reference)?,encode(&family.requirements)?,encode(&family.policy)?,family.alpha,family.policy.max_attempts])?;
        }
        let old: Option<String> = tx
            .query_row(
                "SELECT decision FROM family_decisions WHERE family=?1 AND ordinal=?2",
                params![family.policy.experiment_family_id, ordinal],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(old) = old {
            if old != decision.as_str() {
                conflict(
                    &tx,
                    &family.policy.experiment_family_id,
                    "family_ordinal_reused",
                    decision,
                )?;
                tx.commit()?;
                return Err(RuntimeError::Conflict("family_ordinal_reused"));
            }
        } else {
            let count: u32 = tx.query_row(
                "SELECT count(*) FROM family_decisions WHERE family=?1",
                [&family.policy.experiment_family_id],
                |r| r.get(0),
            )?;
            if count.checked_add(1) != Some(ordinal) || ordinal > family.policy.max_attempts {
                return Err(RuntimeError::Budget("family_attempt_limit_or_sequence"));
            }
            let amount = family.alpha / u64::from(family.policy.max_attempts);
            tx.execute("INSERT INTO family_decisions(family,ordinal,alpha_units,decision) VALUES(?1,?2,?3,?4)",params![family.policy.experiment_family_id,ordinal,amount,decision.as_str()])?;
            event(
                &tx,
                "family_attempt_reserved",
                &family.policy.experiment_family_id,
                &(ordinal, amount, decision),
            )?;
        }
        let (attempts, spent): (u32, u64) = tx.query_row(
            "SELECT count(*),coalesce(sum(alpha_units),0) FROM family_decisions WHERE family=?1",
            [&family.policy.experiment_family_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        if spent > family.alpha {
            return Err(RuntimeError::Budget("family_alpha_exhausted"));
        }
        let result = FamilySpend {
            family: family.policy.experiment_family_id.clone(),
            attempts,
            spent_alpha_units: spent,
            maximum_alpha_units: family.alpha,
            policy: family.reference.clone(),
        };
        tx.commit()?;
        Ok(result)
    }
    /// Profile validation includes allocation/IUT/sampling semantics; this method
    /// additionally persists attempt spending. Neither operation supplies data.
    pub fn reserve_profile_attempt(
        &mut self,
        profile: &DecisionProfile,
        requirements: &RequirementSet,
        registry: &Registry,
    ) -> Result<FamilySpend> {
        profile
            .validate(requirements, registry)
            .map_err(|e| RuntimeError::Wire(e.to_string()))?;
        let family = ApprovedFamily::from_registry(
            registry,
            profile.statistical_method.family_policy.clone(),
            &profile.requirements,
        )?;
        let per_attempt = family.alpha / u64::from(family.policy.max_attempts);
        let allocations = profile
            .statistical_method
            .allocations
            .iter()
            .map(|a| alpha_units(a.alpha))
            .collect::<Result<Vec<_>>>()?;
        let exact = match profile.statistical_method.procedure {
            joil_evidence::decision::Procedure::IntersectionUnion => allocations
                .into_iter()
                .max()
                .ok_or(RuntimeError::Invalid("empty_alpha_family"))?,
            joil_evidence::decision::Procedure::Bonferroni => {
                allocations.into_iter().try_fold(0_u64, |sum, x| {
                    sum.checked_add(x)
                        .ok_or(RuntimeError::Invalid("alpha_sum_overflow"))
                })?
            }
        };
        if exact > per_attempt {
            return Err(RuntimeError::Budget("exact_decimal_attempt_alpha"));
        }
        let decision = SemanticId::parse(
            &profile
                .identity()
                .map_err(|e| RuntimeError::Wire(e.to_string()))?,
        )
        .map_err(|e| RuntimeError::Wire(e.to_string()))?;
        self.reserve_family_attempt(&family, registry, profile.attempt, &decision)
    }
}
