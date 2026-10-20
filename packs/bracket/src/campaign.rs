use super::*;

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
