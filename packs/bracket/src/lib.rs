//! Independent synthetic structural bracket pack.
//!
//! This is a small domain pack proof, not a validated FEA implementation.
//! Loads and allowables are admitted fixture inputs and never hydrocyclone data.
use crucible_contract::keys::{Calibration, EvaluationKey};
use crucible_contract::subject::SemanticId;
use crucible_contract::subject::{
    ArtifactAcceptance, ArtifactFormat, ArtifactRef, ArtifactRole, BlobDigest, FrozenRegion,
    LengthUnit, MaterialState, Port, PortPurpose, RoughnessState, SubjectBundle,
};
use crucible_units::{CoordinateFrame, FrameTransform, Metres};
use joil_evidence::observation::Finite;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BracketProgram {
    pub schema_version: u32,
    pub interface: BoltInterface,
    pub support: Support,
    pub load: FixtureLoad,
    pub material: Material,
    pub geometry: Geometry,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoltInterface {
    pub bolt_centres_mm: [[f64; 2]; 4],
    pub bolt_diameter_mm: f64,
    pub plate_thickness_mm: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Support {
    pub span_mm: f64,
    pub width_mm: f64,
    pub thickness_mm: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureLoad {
    pub force_n: [f64; 3],
    pub moment_nm: [f64; 3],
    pub source: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Material {
    pub name: String,
    pub fixture_yield_mpa: f64,
    pub fixture_density_kg_m3: f64,
    pub evidence: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Geometry {
    pub envelope_mm: [f64; 3],
    pub minimum_web_mm: f64,
    pub maximum_span_mm: f64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuralObservation {
    pub schema_version: u32,
    pub subject: SemanticId,
    pub max_bending_stress_mpa: f64,
    pub fixture_reserve_factor: f64,
    pub estimated_mass_g: f64,
    pub structural_state: String,
    pub physical_qualification: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PackError {
    Invalid(&'static str),
    Wire(String),
}
impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for PackError {}

impl BracketProgram {
    pub fn check(&self) -> Result<(), PackError> {
        if self.schema_version != 1 {
            return Err(PackError::Invalid("schema_version"));
        }
        let positive = [
            self.interface.bolt_diameter_mm,
            self.interface.plate_thickness_mm,
            self.support.span_mm,
            self.support.width_mm,
            self.support.thickness_mm,
            self.material.fixture_yield_mpa,
            self.material.fixture_density_kg_m3,
            self.geometry.minimum_web_mm,
            self.geometry.maximum_span_mm,
        ];
        if positive.iter().any(|x| !x.is_finite() || *x <= 0.0) {
            return Err(PackError::Invalid("positive_finite_geometry_material"));
        }
        if self
            .interface
            .bolt_centres_mm
            .iter()
            .flatten()
            .any(|x| !x.is_finite())
            || self
                .load
                .force_n
                .iter()
                .chain(self.load.moment_nm.iter())
                .any(|x| !x.is_finite())
            || self
                .geometry
                .envelope_mm
                .iter()
                .any(|x| !x.is_finite() || *x <= 0.0)
            || self.load.source.trim().is_empty()
            || self.material.name.trim().is_empty()
            || self.material.evidence.trim().is_empty()
        {
            return Err(PackError::Invalid("finite_admitted_inputs"));
        }
        if positive.iter().any(|x| *x > 1e6)
            || self.load.force_n.iter().any(|x| x.abs() > 1e9)
            || self.load.moment_nm.iter().any(|x| x.abs() > 1e9)
        {
            return Err(PackError::Invalid("bounded_admitted_numeric_range"));
        }
        if self.support.span_mm > self.geometry.maximum_span_mm
            || self.support.span_mm > self.geometry.envelope_mm[0]
            || self.support.width_mm > self.geometry.envelope_mm[1]
            || self.support.thickness_mm > self.geometry.envelope_mm[2]
            || self.support.thickness_mm < self.geometry.minimum_web_mm
        {
            return Err(PackError::Invalid("geometric_bounds"));
        }
        let mut unique = std::collections::BTreeSet::new();
        if self
            .interface
            .bolt_centres_mm
            .iter()
            .any(|p| !unique.insert((p[0].to_bits(), p[1].to_bits())))
        {
            return Err(PackError::Invalid("duplicate_bolt_location"));
        }
        let radius = self.interface.bolt_diameter_mm / 2.0;
        if self.interface.bolt_centres_mm.iter().any(|p| {
            p[0].abs() + radius > self.geometry.envelope_mm[0] / 2.0
                || p[1].abs() + radius > self.geometry.envelope_mm[1] / 2.0
        }) {
            return Err(PackError::Invalid("bolt_outside_interface_envelope"));
        }
        Ok(())
    }

    pub fn identity(&self) -> Result<SemanticId, PackError> {
        self.check()?;
        SemanticId::of(&("pack-bracket/1", self)).map_err(|e| PackError::Wire(e.to_string()))
    }

    /// Elementary cantilever screen: rectangular section, tip load plus applied
    /// bending moment. Fixture calculations only; no stress concentration or FEA.
    pub fn observe(&self) -> Result<StructuralObservation, PackError> {
        self.check()?;
        let f = self.load.force_n;
        let applied = self.load.moment_nm[1] * 1000.0;
        let moment_n_mm = (f[1] * self.support.span_mm + applied).abs();
        let section_modulus_mm3 = self.support.width_mm * self.support.thickness_mm.powi(2) / 6.0;
        let stress = moment_n_mm / section_modulus_mm3;
        let ratio = |capacity: f64, demand: f64| {
            if demand == 0.0 {
                f64::MAX
            } else {
                capacity / demand
            }
        };
        let support_reserve = ratio(self.material.fixture_yield_mpa, stress);
        // Four-bolt elastic load share; bolt group geometry directly affects
        // the extreme fastener demand and is part of the evaluated subject.
        let cx = self
            .interface
            .bolt_centres_mm
            .iter()
            .map(|p| p[0])
            .sum::<f64>()
            / 4.0;
        let cy = self
            .interface
            .bolt_centres_mm
            .iter()
            .map(|p| p[1])
            .sum::<f64>()
            / 4.0;
        let lever = self
            .interface
            .bolt_centres_mm
            .iter()
            .map(|p| ((p[0] - cx).powi(2) + (p[1] - cy).powi(2)).sqrt())
            .fold(0.0_f64, f64::max);
        if lever <= f64::EPSILON {
            return Err(PackError::Invalid("bolt_group_lever_arm"));
        }
        let tensile_per_bolt_n = f[2].abs() / 4.0
            + self.load.moment_nm[0].abs() * 1000.0 / (4.0 * lever)
            + self.load.moment_nm[1].abs() * 1000.0 / (4.0 * lever);
        let bolt_area_mm2 = std::f64::consts::PI * self.interface.bolt_diameter_mm.powi(2) / 4.0;
        let bolt_stress = tensile_per_bolt_n / bolt_area_mm2;
        let reserve = support_reserve.min(ratio(self.material.fixture_yield_mpa, bolt_stress));
        let volume_mm3 = self.support.span_mm * self.support.width_mm * self.support.thickness_mm
            + self.interface.plate_thickness_mm
                * self.geometry.envelope_mm[0]
                * self.geometry.envelope_mm[1];
        let mass_g = volume_mm3 * 1e-6 * self.material.fixture_density_kg_m3;
        if [stress, reserve, mass_g]
            .iter()
            .any(|v| !v.is_finite() || *v < 0.0)
        {
            return Err(PackError::Invalid("unrepresentable_elementary_observation"));
        }
        Ok(StructuralObservation {
            schema_version: 1,
            subject: self.identity()?,
            max_bending_stress_mpa: stress,
            fixture_reserve_factor: reserve,
            estimated_mass_g: mass_g,
            structural_state: if reserve >= 1.0 {
                "fixture_screen_pass"
            } else {
                "fixture_screen_fail"
            }
            .into(),
            physical_qualification: "blocked_missing_as_built_material_process_and_validated_fea"
                .into(),
        })
    }
}

/// Builds the shared observation key using exact pack-owned synthetic artifact
/// bytes. The adapter build/calibration labels make this fixture nonphysical.
pub fn evaluation_key(program: &BracketProgram) -> Result<EvaluationKey, PackError> {
    program.check()?;
    let geometry = serde_json::to_vec(&("synthetic-bracket-geometry/1", program))
        .map_err(|e| PackError::Wire(e.to_string()))?;
    let surface = serde_json::to_vec(&(
        "synthetic-bracket-surface/1",
        &program.interface,
        &program.support,
    ))
    .map_err(|e| PackError::Wire(e.to_string()))?;
    let interface_bytes = serde_json::to_vec(&("pack-bracket-interface/1", &program.interface))
        .map_err(|e| PackError::Wire(e.to_string()))?;
    let material_state = serde_json::to_vec(&("pack-bracket-material-state/1", &program.material))
        .map_err(|e| PackError::Wire(e.to_string()))?;
    let load_scenario = serde_json::to_vec(&("pack-bracket-load-scenario/1", &program.load))
        .map_err(|e| PackError::Wire(e.to_string()))?;
    let interface = BlobDigest::of_bytes(&interface_bytes);
    let bound = Metres::new(1e-6).map_err(|e| PackError::Wire(e.to_string()))?;
    let make_artifact = |role, format, length_unit, bytes: &[u8]| {
        let blob = BlobDigest::of_bytes(bytes);
        let acceptance_bytes = serde_json::to_vec(&ArtifactAcceptance {
            artifact: blob.clone(),
            format,
            length_unit,
            approximation_bound_m: bound,
            interface_contract: interface.clone(),
        })
        .map_err(|e| PackError::Wire(e.to_string()))?;
        Ok::<_, PackError>(ArtifactRef {
            role,
            format,
            blob,
            bytes: bytes.len() as u64,
            length_unit,
            approximation_bound_m: bound,
            acceptance: BlobDigest::of_bytes(&acceptance_bytes),
        })
    };
    let artifacts = vec![
        make_artifact(
            ArtifactRole::Geometry,
            ArtifactFormat::Step,
            LengthUnit::Millimetre,
            &geometry,
        )?,
        make_artifact(
            ArtifactRole::SurfaceMesh,
            ArtifactFormat::TriangleSurfaceJson,
            LengthUnit::Metre,
            &surface,
        )?,
    ];
    let frame = CoordinateFrame::RightHandedSiZUp;
    let finite = |v| Finite::new(v).map_err(PackError::Wire);
    let ports = vec![Port {
        tag: 1,
        purpose: PortPurpose::Support,
        frame,
        centre_m: [
            Metres::new(0.0).unwrap(),
            Metres::new(0.0).unwrap(),
            Metres::new(0.0).unwrap(),
        ],
        area_m2: finite(1e-4)?,
        outward_normal: [finite(0.0)?, finite(0.0)?, finite(1.0)?],
    }];
    let frozen = vec![FrozenRegion {
        tag: 1,
        boundary: BlobDigest::of_bytes(&interface_bytes),
    }];
    let subject = SubjectBundle {
        schema_version: 1,
        artifacts,
        transform: FrameTransform {
            from: frame,
            to: frame,
            translation_m: [Metres::new(0.0).unwrap(); 3],
            rotation: [
                [finite(1.0)?, finite(0.0)?, finite(0.0)?],
                [finite(0.0)?, finite(1.0)?, finite(0.0)?],
                [finite(0.0)?, finite(0.0)?, finite(1.0)?],
            ],
        },
        material: MaterialState::Unknown,
        roughness: RoughnessState::Unknown,
        interface_contract: interface,
        ports,
        frozen_regions: frozen,
    };
    let digest = |label: &str| BlobDigest::of_bytes(label.as_bytes());
    Ok(EvaluationKey {
        schema_version: 1,
        subject,
        manufacturing_state: BlobDigest::of_bytes(&material_state),
        scenario: BlobDigest::of_bytes(&load_scenario),
        solver_build: digest("elementary-bracket-screen-v1"),
        container: digest("software-fixture"),
        numerics: digest("closed-form-rectangular-beam"),
        mesh_policy: digest("no-mesh-elementary-screen"),
        sampling_design: digest("deterministic"),
        randomness: digest("none"),
        extractor: digest("pack-bracket-extractor-v1"),
        calibration: Calibration::Uncalibrated,
    })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttemptRecord {
    pub ordinal: u32,
    pub subject: SemanticId,
    pub outcome: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignResult {
    pub synthetic_only: bool,
    pub attempted: u32,
    pub committed: u32,
    pub attempts: Vec<AttemptRecord>,
    pub observation: StructuralObservation,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignCheckpoint {
    pub schema_version: u32,
    pub campaign: SemanticId,
    pub next_ordinal: u32,
    pub attempts: Vec<AttemptRecord>,
}

pub fn start_campaign(campaign: &Campaign) -> Result<CampaignCheckpoint, PackError> {
    campaign.check()?;
    Ok(CampaignCheckpoint {
        schema_version: 1,
        campaign: SemanticId::of(&("bracket-campaign/1", campaign))
            .map_err(|e| PackError::Wire(e.to_string()))?,
        next_ordinal: 0,
        attempts: Vec::new(),
    })
}

/// Advances from a serialized checkpoint without resetting or hiding attempts.
pub fn advance_campaign(
    campaign: &Campaign,
    checkpoint: &mut CampaignCheckpoint,
    max_new: u32,
) -> Result<(), PackError> {
    campaign.check()?;
    let expected = SemanticId::of(&("bracket-campaign/1", campaign))
        .map_err(|e| PackError::Wire(e.to_string()))?;
    let subject = campaign.fixture.identity()?;
    if checkpoint.schema_version != 1
        || checkpoint.campaign != expected
        || checkpoint.next_ordinal as usize != checkpoint.attempts.len()
        || checkpoint.next_ordinal > campaign.attempts
        || checkpoint.attempts.iter().enumerate().any(|(i, row)| {
            row.ordinal as usize != i
                || row.subject != subject
                || row.outcome != "completed_software_fixture"
        })
    {
        return Err(PackError::Invalid("campaign_checkpoint_contract"));
    }
    for _ in 0..max_new.min(campaign.attempts - checkpoint.next_ordinal) {
        checkpoint.attempts.push(AttemptRecord {
            ordinal: checkpoint.next_ordinal,
            subject: subject.clone(),
            outcome: "completed_software_fixture".into(),
        });
        checkpoint.next_ordinal += 1;
    }
    Ok(())
}

pub fn run_campaign(campaign: &Campaign) -> Result<CampaignResult, PackError> {
    campaign.check()?;
    let observation = campaign.fixture.observe()?;
    let mut checkpoint = start_campaign(campaign)?;
    advance_campaign(campaign, &mut checkpoint, campaign.attempts)?;
    let attempts = checkpoint.attempts;
    Ok(CampaignResult {
        synthetic_only: true,
        attempted: campaign.attempts,
        committed: campaign.attempts,
        attempts,
        observation,
    })
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Campaign {
    pub schema_version: u32,
    pub domain: String,
    pub attempts: u32,
    pub objectives: Vec<String>,
    pub fixture: BracketProgram,
    pub disable_judges: Vec<String>,
    pub inapplicability_rules: Vec<InapplicabilityRule>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InapplicabilityRule {
    pub judge: String,
    pub admitted_domain: String,
    pub reason: String,
}
impl Campaign {
    pub fn reservation(&self) -> crucible_runtime::ledger::Resources {
        crucible_runtime::ledger::Resources {
            cpu_ms: u64::from(self.attempts) * 25,
            gpu_ms: 0,
            ram_bytes: 32 * 1024 * 1024,
            vram_bytes: 0,
            scratch_bytes: 8 * 1024 * 1024,
            io_bytes: 2 * 1024 * 1024,
        }
    }
    pub fn check(&self) -> Result<(), PackError> {
        if self.schema_version != 1
            || self.domain != "structural_bracket_fixture"
            || !(1..=4).contains(&self.attempts)
            || self.objectives
                != [
                    "minimize_estimated_mass_g",
                    "maximize_fixture_reserve_factor",
                ]
        {
            return Err(PackError::Invalid("bounded_bracket_campaign"));
        }
        self.fixture.check()?;
        self.reservation()
            .check()
            .map_err(|e| PackError::Wire(e.to_string()))?;
        for judge in ["oil_grade", "fluid_capacity", "hydraulic_energy"] {
            let expected = match judge {
                "oil_grade" => "Bracket fixture has no fluid domain or oil transport subject.",
                "fluid_capacity" => "Bracket fixture defines no flow interfaces or capacity duty.",
                _ => "Bracket fixture defines no hydraulic system.",
            };
            if self.disable_judges.iter().any(|x| x == judge)
                && !self.inapplicability_rules.iter().any(|r| {
                    r.judge == judge && r.admitted_domain == self.domain && r.reason == expected
                })
            {
                return Err(PackError::Invalid("unregistered_not_applicable_rule"));
            }
        }
        if self
            .disable_judges
            .iter()
            .any(|x| !["oil_grade", "fluid_capacity", "hydraulic_energy"].contains(&x.as_str()))
        {
            return Err(PackError::Invalid("unknown_judge"));
        }
        if self.inapplicability_rules.iter().any(|r| {
            !["oil_grade", "fluid_capacity", "hydraulic_energy"].contains(&r.judge.as_str())
                || r.admitted_domain != self.domain
                || !self.disable_judges.contains(&r.judge)
        }) {
            return Err(PackError::Invalid("unregistered_inapplicability_rule"));
        }
        Ok(())
    }
}

/// Pack-owned data card recipe. It intentionally accepts only bracket fields.
pub fn data_card(
    program: &BracketProgram,
    observation: &StructuralObservation,
) -> Result<String, PackError> {
    program.check()?;
    if observation.subject != program.identity()? {
        return Err(PackError::Invalid("data_card_subject_mismatch"));
    }
    let escape = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    };
    Ok(format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"720\" height=\"300\" viewBox=\"0 0 720 300\"><title>Bracket fixture data card</title><rect width=\"720\" height=\"300\" fill=\"white\"/><text x=\"20\" y=\"32\">Bracket fixture | source: {}</text><text x=\"20\" y=\"64\">Subject SHA-256: {}</text><text x=\"20\" y=\"96\">Force [N]: {:.3}, {:.3}, {:.3}</text><text x=\"20\" y=\"128\">Moment [N m]: {:.3}, {:.3}, {:.3}</text><text x=\"20\" y=\"160\">Material: {} | fixture yield [MPa]: {:.3}</text><text x=\"20\" y=\"192\">Material evidence: {}</text><text x=\"20\" y=\"224\">Bending screen [MPa]: {:.6} | reserve factor [-]: {:.6}</text><text x=\"20\" y=\"256\">Mass estimate [g]: {:.6} | {} | qualification blocked</text></svg>", escape(&program.load.source), observation.subject.as_str(), program.load.force_n[0], program.load.force_n[1], program.load.force_n[2], program.load.moment_nm[0], program.load.moment_nm[1], program.load.moment_nm[2], escape(&program.material.name), program.material.fixture_yield_mpa, escape(&program.material.evidence), observation.max_bending_stress_mpa, observation.fixture_reserve_factor, observation.estimated_mass_g, observation.structural_state))
}
