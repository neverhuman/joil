//! Restartable parcel identity, first-passage terminal inventory and adverse bounds.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum TerminalState {
    Active,
    RejectFirstPass,
    TreatedFirstPass,
    WallRetained,
    DomainRetained,
    Removed(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParcelRecord {
    pub parcel_id: u64,
    pub cohort_id: u64,
    /// Immutable oil mass assigned at injection; zero for daughters.
    pub injected_mass_kg: f64,
    /// Current physical oil mass represented by this live parcel.
    pub represented_mass_kg: f64,
    /// DSD injection-size stratum, inherited unchanged by daughter parcels.
    pub injection_size_bin: usize,
    pub initial_diameter_m: f64,
    pub current_diameter_m: f64,
    pub injection_stratum: String,
    pub injection_time_s: f64,
    pub parent_id: Option<u64>,
    pub daughter_ids: Vec<u64>,
    pub random_stream: u64,
    pub terminal: TerminalState,
    /// Display-only size, deliberately excluded from physical mass/diameter.
    pub marker_diameter_px: f64,
}

impl ParcelRecord {
    pub fn validate(&self) -> Result<(), String> {
        if self.injection_stratum.is_empty()
            || !self.injected_mass_kg.is_finite()
            || self.injected_mass_kg < 0.0
            || !self.represented_mass_kg.is_finite()
            || self.represented_mass_kg < 0.0
            || (self.parent_id.is_none() && self.injected_mass_kg <= 0.0)
            || (self.parent_id.is_some() && self.injected_mass_kg != 0.0)
            || !self.initial_diameter_m.is_finite()
            || self.initial_diameter_m <= 0.0
            || !self.current_diameter_m.is_finite()
            || self.current_diameter_m <= 0.0
            || !self.injection_time_s.is_finite()
            || self.injection_time_s < 0.0
        {
            return Err("invalid parcel identity or conserved properties".into());
        }
        Ok(())
    }
    pub fn is_independent_injection(&self) -> bool {
        self.parent_id.is_none()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Inventory {
    pub parcels: Vec<ParcelRecord>,
    pub unresolved_mass_kg: f64,
    pub synchronized_window_s: [f64; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Closure {
    pub injected_kg: f64,
    pub rejected_kg: f64,
    pub treated_kg: f64,
    pub wall_kg: f64,
    pub domain_kg: f64,
    pub discrepancy_kg: f64,
    pub concentration_lower_mg_l: f64,
    pub concentration_upper_mg_l: f64,
}

impl Inventory {
    pub fn validate_restart(&self) -> Result<(), String> {
        if !self.unresolved_mass_kg.is_finite()
            || self.unresolved_mass_kg < 0.0
            || !self.synchronized_window_s.iter().all(|x| x.is_finite())
            || self.synchronized_window_s[0] < 0.0
            || self.synchronized_window_s[1] <= self.synchronized_window_s[0]
        {
            return Err("invalid inventory window or unresolved mass".into());
        }
        let mut ids = std::collections::HashSet::new();
        for p in &self.parcels {
            p.validate()?;
            if !ids.insert(p.parcel_id) {
                return Err("duplicate parcel ID".into());
            }
        }
        for p in &self.parcels {
            if p.parent_id.is_some_and(|id| !ids.contains(&id)) {
                return Err("daughter parent is absent from restart inventory".into());
            }
            if p.daughter_ids.iter().any(|id| !ids.contains(id)) {
                return Err("parcel references an absent daughter".into());
            }
            if let Some(parent) = p
                .parent_id
                .and_then(|id| self.parcels.iter().find(|x| x.parcel_id == id))
            {
                if !parent.daughter_ids.contains(&p.parcel_id)
                    || parent.cohort_id != p.cohort_id
                    || parent.injection_size_bin != p.injection_size_bin
                    || parent.initial_diameter_m != p.initial_diameter_m
                    || parent.injection_stratum != p.injection_stratum
                    || parent.injection_time_s != p.injection_time_s
                {
                    return Err("daughter lineage or cohort identity is inconsistent".into());
                }
            }
            let mut daughters = std::collections::HashSet::new();
            for id in &p.daughter_ids {
                if !daughters.insert(*id)
                    || self
                        .parcels
                        .iter()
                        .find(|d| d.parcel_id == *id)
                        .is_none_or(|d| d.parent_id != Some(p.parcel_id))
                {
                    return Err("duplicate or nonreciprocal daughter link".into());
                }
            }
            let mut ancestors = std::collections::HashSet::new();
            let mut current = Some(p.parcel_id);
            while let Some(id) = current {
                if !ancestors.insert(id) {
                    return Err("cyclic parcel lineage".into());
                }
                current = self
                    .parcels
                    .iter()
                    .find(|p| p.parcel_id == id)
                    .and_then(|p| p.parent_id);
            }
        }
        Ok(())
    }

    /// Atomically transfer a parent's current represented mass to daughters.
    /// The parent remains as a zero-current-mass lineage record; its immutable
    /// injected mass continues to count exactly once.
    pub fn split(
        &mut self,
        parent_id: u64,
        mut daughters: Vec<ParcelRecord>,
    ) -> Result<(), String> {
        self.validate_restart()?;
        if daughters.is_empty() {
            return Err("breakup requires at least one daughter".into());
        }
        let parent_index = self
            .parcels
            .iter()
            .position(|p| p.parcel_id == parent_id)
            .ok_or("unknown breakup parent")?;
        let parent = &self.parcels[parent_index];
        if parent.terminal != TerminalState::Active || !parent.daughter_ids.is_empty() {
            return Err("only an active unsplit parent can break up".into());
        }
        let mass = parent.represented_mass_kg;
        let daughter_mass: f64 = daughters.iter().map(|p| p.represented_mass_kg).sum();
        let roundoff = 64.0 * f64::EPSILON * mass.abs().max(daughter_mass.abs());
        if mass <= 0.0
            || !daughter_mass.is_finite()
            || daughter_mass <= 0.0
            || (daughter_mass - mass).abs() > roundoff
        {
            return Err("breakup daughter mass does not conserve parent represented mass".into());
        }
        let parent_cohort = parent.cohort_id;
        let parent_size_bin = parent.injection_size_bin;
        let mut ids: std::collections::HashSet<u64> =
            self.parcels.iter().map(|p| p.parcel_id).collect();
        if daughters.iter().any(|d| {
            d.parent_id != Some(parent_id)
                || d.cohort_id != parent_cohort
                || d.injection_size_bin != parent_size_bin
                || d.initial_diameter_m != parent.initial_diameter_m
                || d.injection_stratum != parent.injection_stratum
                || d.injection_time_s != parent.injection_time_s
                || !ids.insert(d.parcel_id)
        }) {
            return Err("daughter identity, cohort, or parent link is invalid".into());
        }
        for d in &daughters {
            d.validate()?;
        }
        self.parcels[parent_index].represented_mass_kg = 0.0;
        self.parcels[parent_index].daughter_ids = daughters.iter().map(|d| d.parcel_id).collect();
        self.parcels[parent_index].terminal =
            TerminalState::Removed("mass_transferred_to_daughters".into());
        self.parcels.append(&mut daughters);
        Ok(())
    }

    /// A terminal's first crossing is counted once; recrossing cannot double count.
    pub fn mark_first_pass(&mut self, id: u64, terminal: TerminalState) -> Result<(), String> {
        if !matches!(
            terminal,
            TerminalState::RejectFirstPass
                | TerminalState::TreatedFirstPass
                | TerminalState::WallRetained
                | TerminalState::Removed(_)
        ) {
            return Err("not a terminal event".into());
        }
        let p = self
            .parcels
            .iter_mut()
            .find(|p| p.parcel_id == id)
            .ok_or("unknown parcel")?;
        if p.terminal != TerminalState::Active {
            return Err("terminal recrossing or duplicate removal detected".into());
        }
        p.terminal = terminal;
        Ok(())
    }

    pub fn closure(
        &self,
        inlet_oil_kg: f64,
        treated_water_m3: f64,
        uncertainty_oil_kg: f64,
    ) -> Result<Closure, String> {
        self.validate_restart()?;
        if !inlet_oil_kg.is_finite()
            || inlet_oil_kg < 0.0
            || !treated_water_m3.is_finite()
            || treated_water_m3 <= 0.0
            || !uncertainty_oil_kg.is_finite()
            || uncertainty_oil_kg < 0.0
        {
            return Err("invalid closure denominator or uncertainty".into());
        }
        let mut rejected = 0.0;
        let mut treated = 0.0;
        let mut wall = 0.0;
        let mut domain = 0.0;
        let mut injected = 0.0;
        for p in &self.parcels {
            injected += p.injected_mass_kg;
            match p.terminal {
                TerminalState::RejectFirstPass => rejected += p.represented_mass_kg,
                TerminalState::TreatedFirstPass => treated += p.represented_mass_kg,
                TerminalState::WallRetained => wall += p.represented_mass_kg,
                TerminalState::Active | TerminalState::DomainRetained => {
                    domain += p.represented_mass_kg
                }
                TerminalState::Removed(_) => {}
            }
        }
        if ![injected, rejected, treated, wall, domain]
            .iter()
            .all(|v| v.is_finite())
        {
            return Err("unrepresentable inventory mass sum".into());
        }
        let injection_roundoff = 64.0 * f64::EPSILON * injected.abs().max(inlet_oil_kg.abs());
        if (injected - inlet_oil_kg).abs() > uncertainty_oil_kg + injection_roundoff {
            return Err("inlet mass differs from immutable injection ledger".into());
        }
        let discrepancy = inlet_oil_kg - (rejected + treated + wall + domain);
        let unresolved = discrepancy.abs() + self.unresolved_mass_kg + uncertainty_oil_kg;
        let base_mg_l = treated / treated_water_m3 * 1e6;
        let adverse = (domain + wall + unresolved) / treated_water_m3 * 1e6;
        if ![discrepancy, base_mg_l, adverse, base_mg_l + adverse]
            .iter()
            .all(|v| v.is_finite())
        {
            return Err("unrepresentable inventory concentration bounds".into());
        }
        Ok(Closure {
            injected_kg: injected,
            rejected_kg: rejected,
            treated_kg: treated,
            wall_kg: wall,
            domain_kg: domain,
            discrepancy_kg: discrepancy,
            concentration_lower_mg_l: base_mg_l,
            concentration_upper_mg_l: base_mg_l + adverse,
        })
    }

    /// Reconcile inventory by immutable injection-size class over aligned windows.
    /// Unresolved and uncertainty mass remain adverse in their respective class.
    pub fn closure_by_size(
        &self,
        inlet_by_bin_kg: &[f64],
        treated_water_by_bin_m3: &[f64],
        unresolved_by_bin_kg: &[f64],
        uncertainty_by_bin_kg: &[f64],
    ) -> Result<Vec<Closure>, String> {
        self.validate_restart()?;
        let n = inlet_by_bin_kg.len();
        let allocated_unresolved: f64 = unresolved_by_bin_kg.iter().sum();
        let unresolved_roundoff =
            64.0 * f64::EPSILON * allocated_unresolved.abs().max(self.unresolved_mass_kg);
        if n == 0
            || treated_water_by_bin_m3.len() != n
            || unresolved_by_bin_kg.len() != n
            || uncertainty_by_bin_kg.len() != n
            || self.parcels.iter().any(|p| p.injection_size_bin >= n)
            || !inlet_by_bin_kg
                .iter()
                .chain(treated_water_by_bin_m3)
                .chain(unresolved_by_bin_kg)
                .chain(uncertainty_by_bin_kg)
                .all(|x| x.is_finite() && *x >= 0.0)
            || treated_water_by_bin_m3.contains(&0.0)
            || !allocated_unresolved.is_finite()
            || (allocated_unresolved - self.unresolved_mass_kg).abs() > unresolved_roundoff
        {
            return Err("size-class closure inputs are not aligned to finite, positive synchronized windows".into());
        }
        (0..n)
            .map(|bin| {
                Self {
                    parcels: self
                        .parcels
                        .iter()
                        .filter(|p| p.injection_size_bin == bin)
                        .cloned()
                        .collect(),
                    unresolved_mass_kg: unresolved_by_bin_kg[bin],
                    synchronized_window_s: self.synchronized_window_s,
                }
                .closure(
                    inlet_by_bin_kg[bin],
                    treated_water_by_bin_m3[bin],
                    uncertainty_by_bin_kg[bin],
                )
            })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SamplingPlan {
    pub duration_s: f64,
    pub independent_replicates: usize,
    pub parcel_budget: usize,
    pub residence_tail_s: f64,
}

/// Derive sampling effort from a declared decision margin and measured residence tail.
/// This deliberately refuses the legacy 32-parcel/0.01-second defaults as sufficient evidence.
pub fn plan_sampling(
    decision_margin_mg_l: f64,
    concentration_mg_l: f64,
    residence_tail_s: f64,
    tail_quantile_complete: bool,
    base_parcels: usize,
) -> Result<SamplingPlan, String> {
    if !decision_margin_mg_l.is_finite()
        || decision_margin_mg_l <= 0.0
        || !concentration_mg_l.is_finite()
        || concentration_mg_l < 0.0
        || !residence_tail_s.is_finite()
        || residence_tail_s <= 0.0
        || !tail_quantile_complete
        || base_parcels == 0
    {
        return Err("decision precision and completed residence-tail evidence are required".into());
    }
    let required =
        ((1.96 * concentration_mg_l.max(1.0) / (decision_margin_mg_l / 3.0)).powi(2)).ceil();
    let duration = 3.0 * residence_tail_s;
    if !required.is_finite() || required >= usize::MAX as f64 || !duration.is_finite() {
        return Err("unrepresentable sampling effort".into());
    }
    let n = required as usize;
    let parcels = base_parcels.max(n).max(33);
    Ok(SamplingPlan {
        duration_s: duration,
        independent_replicates: 3,
        parcel_budget: parcels,
        residence_tail_s,
    })
}

pub fn grade_qualified(completed: bool, closure: &Closure, threshold_mg_l: f64) -> bool {
    completed
        && threshold_mg_l.is_finite()
        && threshold_mg_l >= 0.0
        && [
            closure.injected_kg,
            closure.rejected_kg,
            closure.treated_kg,
            closure.wall_kg,
            closure.domain_kg,
            closure.concentration_lower_mg_l,
            closure.concentration_upper_mg_l,
        ]
        .iter()
        .all(|v| v.is_finite() && *v >= 0.0)
        && closure.discrepancy_kg.is_finite()
        && closure.concentration_lower_mg_l <= closure.concentration_upper_mg_l
        && closure.concentration_upper_mg_l <= threshold_mg_l
}
