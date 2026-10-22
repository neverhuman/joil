//! Campaign configuration and the run manifest that binds a checkpoint to it.

use crate::layout::Envelope;
use joil_forge_judges::capacity::Limits;
use joil_forge_judges::cyclone::Resolution;
use joil_forge_judges::fluid::{FeedState, Fluids, Lognormal};
use joil_forge_judges::pressure::Terminals;
use joil_forge_judges::MODEL_VERSION;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const FORGE_VERSION: &str = "joil-forge/1.1.1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Seeding {
    /// Random genomes only (no agent-authored designs).
    DeNovo,
    /// Random genomes plus mutants of the archetypes in [`crate::random::archetypes`].
    Archetypes,
}

/// Weights of the scalar objective applied to feasible designs.
///
/// Oil separation leads: the main term is the oil removal achieved on every duty at the
/// design's rated flow. Capacity, energy, mass and part count trade against it. Nothing
/// here mentions stages or topology; series processing scores only through the oil it
/// removes.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Objective {
    /// On the weighted mean over duties of `ln(C_in / max(C_out, floor))` (natural-log
    /// removal), each duty run at the rated flow under its worst closure sample.
    pub separation: f64,
    /// On `ln(conservative treated capacity / 1 m³/h)`.
    pub capacity: f64,
    /// On `-ln(specific energy / 100 kJ/m³)` at the capacity point.
    pub energy: f64,
    /// On `-ln(printed shell mass / 1 kg)`.
    pub mass: f64,
    /// Per unit (Occam pressure on part count).
    pub per_unit: f64,
    /// Outlet concentration below which further removal earns nothing (mg/L): about the
    /// bottom of routine oil-in-water measurement.
    pub floor_mg_l: f64,
}

impl Default for Objective {
    fn default() -> Self {
        Self {
            separation: 1.0,
            capacity: 0.5,
            energy: 0.25,
            mass: 0.15,
            per_unit: 0.02,
            floor_mg_l: 1.0,
        }
    }
}

/// One operating duty: carrier water, dispersed oil and feed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Duty {
    pub name: String,
    pub fluids: Fluids,
    /// Concentration and drop sizes; the flow is set by the design's rating.
    pub feed: FeedState,
    /// Weight in the separation term.
    pub weight: f64,
}

impl Duty {
    /// The repository reference duty (`RS-joil-research-v1`): fresh-water properties at
    /// about 20 °C, oil at 850 kg/m³, 500 mg/L, oil-volume median 15 µm.
    pub fn produced_water() -> Self {
        Self {
            name: "produced water (reference)".into(),
            fluids: Fluids::reference(),
            feed: FeedState::reference(),
            weight: 1.0,
        }
    }

    /// Oil-spill recovery water. Seawater at 15 °C and 35 g/kg (ITTC 7.5-02-01-03:
    /// 1025.89 kg/m³, 1.220 mPa·s). The oil is an authored screen assumption for a
    /// medium, lightly weathered crude (900 kg/m³, 20 mPa·s, 0.020 N/m), at 1000 mg/L
    /// with a 10 µm oil-volume median for pump-sheared skimmer water.
    pub fn seawater_spill() -> Self {
        Self {
            name: "seawater oil spill".into(),
            fluids: Fluids {
                rho_water_kg_m3: 1025.89,
                mu_water_pa_s: 1.220e-3,
                rho_oil_kg_m3: 900.0,
                mu_oil_pa_s: 0.020,
                interfacial_tension_n_m: 0.020,
            },
            feed: FeedState {
                flow_m3_h: 4.0,
                cin_mg_l: 1000.0,
                dsd: Lognormal {
                    volume_median_um: 10.0,
                    geometric_sd: 1.6,
                },
            },
            weight: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Campaign {
    pub name: String,
    pub seed: u64,
    /// Seed of the Latin-hypercube closure samples; `seed` when absent. Replicate runs set
    /// it so they share one objective while their search randomness differs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closure_seed: Option<u64>,
    pub population: usize,
    pub islands: usize,
    pub migrate_every: usize,
    pub migrants: usize,
    pub tournament: usize,
    pub crossover_rate: f64,
    pub structural_rate: f64,
    pub archive_parent_rate: f64,
    pub seeding: Seeding,
    /// Latin-hypercube closure samples in addition to the nominal closure.
    pub closure_samples: usize,
    pub allow_pumps: bool,
    pub objective: Objective,
    pub limits: Limits,
    /// Duty 0 rates the design: capacity and compliance are found on it under every
    /// closure sample. Every duty, the first included, enters the separation term.
    pub duties: Vec<Duty>,
    /// Printer build volume and printability rules.
    pub envelope: Envelope,
    pub terminals: Terminals,
    pub resolution: Resolution,
}

impl Default for Campaign {
    fn default() -> Self {
        Self {
            name: "forge-v2".into(),
            seed: 20_261_021,
            closure_seed: None,
            population: 48,
            islands: 2,
            migrate_every: 5,
            migrants: 2,
            tournament: 3,
            crossover_rate: 0.3,
            structural_rate: 0.45,
            archive_parent_rate: 0.3,
            seeding: Seeding::DeNovo,
            closure_samples: 3,
            allow_pumps: true,
            objective: Objective::default(),
            limits: Limits::default(),
            duties: vec![Duty::produced_water(), Duty::seawater_spill()],
            envelope: Envelope::default(),
            terminals: Terminals::default(),
            resolution: Resolution::default(),
        }
    }
}

impl Campaign {
    /// Seed of the closure samples every candidate is scored under.
    pub fn closure_seed(&self) -> u64 {
        self.closure_seed.unwrap_or(self.seed)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.population < 4 || self.islands == 0 || self.tournament == 0 {
            return Err("population >= 4, islands >= 1 and tournament >= 1 are required".into());
        }
        if !(0.0..=1.0).contains(&self.crossover_rate)
            || !(0.0..=1.0).contains(&self.structural_rate)
            || !(0.0..=1.0).contains(&self.archive_parent_rate)
        {
            return Err("rates must lie in [0, 1]".into());
        }
        if self.duties.is_empty() || self.duties.iter().all(|d| d.weight <= 0.0) {
            return Err("at least one duty with positive weight is required".into());
        }
        for d in &self.duties {
            if !d.weight.is_finite() || d.weight < 0.0 {
                return Err(format!(
                    "duty '{}': weight must be finite and nonnegative",
                    d.name
                ));
            }
            if ![
                d.fluids.rho_water_kg_m3,
                d.fluids.mu_water_pa_s,
                d.fluids.rho_oil_kg_m3,
                d.fluids.mu_oil_pa_s,
                d.fluids.interfacial_tension_n_m,
                d.feed.dsd.volume_median_um,
                d.feed.dsd.geometric_sd,
            ]
            .iter()
            .all(|x| x.is_finite())
            {
                return Err(format!(
                    "duty '{}': properties and drop sizes must be finite",
                    d.name
                ));
            }
            d.fluids
                .validate()
                .map_err(|e| format!("duty '{}': {e}", d.name))?;
            d.feed
                .validate()
                .map_err(|e| format!("duty '{}': {e}", d.name))?;
        }
        let weight_sum: f64 = self.duties.iter().map(|d| d.weight).sum();
        if !weight_sum.is_finite() || weight_sum <= 0.0 {
            return Err("total duty weight must be finite and positive".into());
        }
        self.envelope.validate()?;
        if !self.objective.floor_mg_l.is_finite() || self.objective.floor_mg_l <= 0.0 {
            return Err("separation floor must be finite and positive".into());
        }
        if ![
            self.objective.separation,
            self.objective.capacity,
            self.objective.energy,
            self.objective.mass,
            self.objective.per_unit,
        ]
        .iter()
        .all(|x| x.is_finite() && *x >= 0.0)
        {
            return Err("objective weights must be finite and nonnegative".into());
        }
        let l = &self.limits;
        if ![
            l.pressure_budget_pa,
            l.quality_limit_mg_l,
            l.q_min_m3_h,
            l.q_max_m3_h,
            l.max_amplification,
        ]
        .iter()
        .all(|x| x.is_finite() && *x > 0.0)
            || l.q_max_m3_h < l.q_min_m3_h
            || l.max_amplification < 1.0
            || !(0.0..1.0).contains(&l.reject_budget)
            || !(0.0..1.0).contains(&l.quality_probe_ratio)
            || l.quality_probe_ratio == 0.0
            || l.quality_probes == 0
        {
            return Err(
                "invalid pressure, quality, reject, flow, amplification or probe limits".into(),
            );
        }
        if ![
            self.terminals.product_pa,
            self.terminals.reject_pa,
            self.terminals.pump_suction_min_pa,
        ]
        .iter()
        .all(|x| x.is_finite())
            || !self.terminals.pump_efficiency.is_finite()
            || !(0.0..=1.0).contains(&self.terminals.pump_efficiency)
            || self.terminals.pump_efficiency == 0.0
        {
            return Err("terminal pressures must be finite and pump efficiency in (0, 1]".into());
        }
        if self.resolution.nz == 0 || self.resolution.ns == 0 {
            return Err("resolution dimensions must be positive".into());
        }
        Ok(())
    }

    pub fn sha256(&self) -> String {
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(self).expect("campaign serialises"))
        )
    }

    /// The duty that rates the design (capacity and compliance).
    pub fn rating(&self) -> &Duty {
        &self.duties[0]
    }
}

/// Identity of a run: campaign content plus code/model versions. A checkpoint resumes
/// only under the same manifest unless the caller explicitly forks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub campaign_sha256: String,
    pub model_version: String,
    pub forge_version: String,
    pub evidence_class: String,
    pub parent_manifest_sha256: Option<String>,
    pub seeded_from: Option<String>,
}

impl Manifest {
    pub fn new(c: &Campaign) -> Self {
        Self {
            campaign_sha256: c.sha256(),
            model_version: MODEL_VERSION.into(),
            forge_version: FORGE_VERSION.into(),
            evidence_class: joil_forge_judges::EVIDENCE_CLASS.into(),
            parent_manifest_sha256: None,
            seeded_from: None,
        }
    }

    /// Identity used for resume checks (lineage fields excluded).
    pub fn identity(&self) -> String {
        let core = (
            &self.campaign_sha256,
            &self.model_version,
            &self.forge_version,
        );
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&core).expect("serialises"))
        )
    }

    pub fn sha256(&self) -> String {
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(self).expect("serialises"))
        )
    }
}
