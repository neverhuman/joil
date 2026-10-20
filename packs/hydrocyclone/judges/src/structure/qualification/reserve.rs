use super::*;

// ---------------------------------------------------------------- V5-19-05 reserve

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LimitKind {
    Yield,
    Buckling,
    Contact,
    Modal,
    Fatigue,
}

impl LimitKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Yield => "yield",
            Self::Buckling => "buckling",
            Self::Contact => "contact",
            Self::Modal => "modal",
            Self::Fatigue => "fatigue",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Response {
    Linear,
    Nonlinear,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Derivation {
    /// Scaled from one solve at multiplier 1; valid only for a linear response.
    ScaledSingleSolve,
    /// Re-solved at `resolved_multiplier`, which must reach the claimed allowable.
    ExplicitResolves {
        solve_sha256: Sha256Hex,
        resolved_multiplier: f64,
    },
}

/// An allowable load multiplier of the frozen demand for one family.
#[derive(Clone, Debug, PartialEq)]
pub struct Limit {
    pub family: LoadFamily,
    pub kind: LimitKind,
    pub response: Response,
    pub derivation: Derivation,
    pub allowable_multiplier: f64,
    pub load_basis_sha256: Sha256Hex,
    /// Required before a modal or fatigue number may be treated as a load multiplier.
    pub multiplier_definition: Option<Field<String>>,
}

/// A frequency ratio or fatigue safety factor judged against its own requirement.
#[derive(Clone, Debug, PartialEq)]
pub struct SeparateRequirement {
    pub kind: LimitKind,
    pub name: String,
    pub value: f64,
    pub required: Field<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReserveReport {
    /// Allowable multiplier over required factor, per family, from compatible limits only.
    pub family_reserve: BTreeMap<LoadFamily, f64>,
    pub governing: Option<(LoadFamily, LimitKind, f64)>,
    pub separate: Vec<(String, Verdict)>,
    pub verdict: Verdict,
}

pub fn compute_reserve(
    frozen: &FrozenLoads,
    limits: &[Limit],
    separate: &[SeparateRequirement],
) -> Result<ReserveReport, JudgeError> {
    if limits
        .iter()
        .any(|l| !l.allowable_multiplier.is_finite() || l.allowable_multiplier <= 0.0)
        || separate.iter().any(|s| !s.value.is_finite())
    {
        return Err(invalid(
            "allowable multipliers must be finite and positive; separate values finite",
        ));
    }
    let mut parts = Vec::new();
    let mut family_reserve = BTreeMap::new();
    let mut governing: Option<(LoadFamily, LimitKind, f64)> = None;
    for family in ALL_FAMILIES {
        let name = family.name();
        let required = frozen
            .set
            .families
            .get(&family)
            .and_then(|d| d.required_factor.admitted())
            .copied();
        if required.is_none() {
            parts.push(open(format!("family:{name}:required_factor:not_admitted")));
        }
        let mut usable: Vec<&Limit> = Vec::new();
        for l in limits.iter().filter(|l| l.family == family) {
            let kn = l.kind.name();
            if l.load_basis_sha256 != frozen.sha256 {
                parts.push(fail(format!(
                    "family:{name}:{kn}:load_basis_differs_from_frozen_set"
                )));
            } else if matches!(l.kind, LimitKind::Modal | LimitKind::Fatigue)
                && l.multiplier_definition
                    .as_ref()
                    .and_then(Field::admitted)
                    .is_none()
            {
                parts.push(open(format!(
                    "family:{name}:{kn}:not_a_load_multiplier_without_definition"
                )));
            } else if let (Response::Nonlinear, Derivation::ScaledSingleSolve) =
                (l.response, &l.derivation)
            {
                parts.push(open(format!(
                    "family:{name}:{kn}:nonlinear_limit_needs_explicit_resolves"
                )));
            } else if let Derivation::ExplicitResolves {
                resolved_multiplier,
                ..
            } = &l.derivation
            {
                if !resolved_multiplier.is_finite()
                    || *resolved_multiplier < l.allowable_multiplier * (1.0 - 1e-12)
                {
                    parts.push(open(format!(
                        "family:{name}:{kn}:resolve_does_not_reach_claimed_multiplier"
                    )));
                } else {
                    usable.push(l);
                }
            } else {
                usable.push(l);
            }
        }
        if family == LoadFamily::ExternalDifferentialUpset
            && !usable.iter().any(|l| l.kind == LimitKind::Buckling)
        {
            parts.push(open("external_pressure_buckling:untested"));
        }
        let Some(worst) = usable
            .iter()
            .min_by(|a, b| a.allowable_multiplier.total_cmp(&b.allowable_multiplier))
        else {
            parts.push(open(format!("family:{name}:no_usable_limit")));
            continue;
        };
        let Some(required) = required else { continue };
        let reserve = worst.allowable_multiplier / required;
        family_reserve.insert(family, reserve);
        if governing.is_none_or(|(_, _, g)| reserve < g) {
            governing = Some((family, worst.kind, reserve));
        }
        if reserve < 1.0 {
            parts.push(fail(format!(
                "family:{name}:reserve_below_1:{}",
                worst.kind.name()
            )));
        }
    }
    let mut separate_out = Vec::new();
    for s in separate {
        let v = match s.required.admitted() {
            None => open(format!("{}:required_value:not_admitted", s.name)),
            Some(r) if s.value >= *r => Verdict::Pass,
            Some(_) => fail(format!(
                "{}:{}_below_own_requirement",
                s.name,
                s.kind.name()
            )),
        };
        parts.push(v.clone());
        separate_out.push((s.name.clone(), v));
    }
    Ok(ReserveReport {
        family_reserve,
        governing,
        separate: separate_out,
        verdict: Verdict::combine(parts),
    })
}
