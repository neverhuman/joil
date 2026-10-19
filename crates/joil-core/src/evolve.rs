//! Multi-island Constrained NSGA-II + 4D MAP-Elites Quality-Diversity evolutionary engine
//! for radical 3D hydrocyclone discovery, with Monte Carlo robustness and optional CUDA batch verification.

use crate::experiment::ProposalBudget;
use crate::metrics;
use crate::openfoam::STANDARD_BINS_UM;
use crate::phenotype::{Phenotype3D, SolidMesh};
use crate::program::{DesignProgram, MechanismDescriptors, Rng, VariationError, VariationOutcome};
use crate::redteam;
use crate::structure::{self, AnalyticStructuralScreen};
use joil_evidence::{identity::ContentRef, observation::EvidenceTier, subject::Lineage};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub(crate) use std::fs;
pub(crate) use std::io::{BufWriter, Write};
pub(crate) use std::path::{Path, PathBuf};
pub(crate) use std::process::Command;

#[path = "evolve/campaign.rs"]
mod campaign;
#[path = "evolve/raw_capture.rs"]
mod raw_capture;
#[path = "evolve/types.rs"]
mod types;
#[path = "evolve/wave2.rs"]
mod wave2;
#[path = "evolve/wave3_arm.rs"]
mod wave3_arm;
#[path = "evolve/wave3_proof.rs"]
mod wave3_proof;

pub use self::campaign::*;
pub use self::raw_capture::*;
pub use self::types::*;
pub use self::wave2::*;
pub use self::wave3_arm::*;
pub use self::wave3_proof::*;

pub(crate) fn evaluate_proposal(
    outcome: Result<VariationOutcome, VariationError>,
    id: String,
    generation: usize,
    island: usize,
    cfg: &EvolutionConfig,
    budget: &mut ProposalBudget,
) -> Result<Option<Individual>, String> {
    evaluate_proposal_recorded(outcome, id, generation, island, cfg, budget, None)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn evaluate_proposal_recorded(
    outcome: Result<VariationOutcome, VariationError>,
    id: String,
    generation: usize,
    island: usize,
    cfg: &EvolutionConfig,
    budget: &mut ProposalBudget,
    capture: Option<&mut RawSeedCapture>,
) -> Result<Option<Individual>, String> {
    let mut outcome = outcome?;
    outcome.candidate_id = Some(id.clone());
    let mut mc = Vec::new();
    let result = match outcome.child.clone() {
        Some(child) if outcome.failure.is_none() => {
            match evaluate_individual_recorded(
                id,
                child,
                generation,
                island,
                cfg,
                capture.as_ref().map(|_| &mut mc),
            ) {
                Ok((mut individual, _)) => {
                    individual.lineage = Some(proposal_lineage(&outcome, cfg.seed)?);
                    Some(individual)
                }
                Err(reason) => {
                    outcome.child = None;
                    outcome.failure = Some(format!("compilation/evaluation failed: {reason}"));
                    None
                }
            }
        }
        _ => None,
    };
    if let Some(capture) = capture {
        capture.proposal(&outcome, result.as_ref(), mc)?;
    }
    budget.record(outcome);
    Ok(result)
}

pub(crate) fn proposal_lineage(outcome: &VariationOutcome, seed: u64) -> Result<Lineage, String> {
    // These identify the exact implementation source/configuration that produced
    // the legacy screening mesh, not an independently validated compiler binary.
    let compiler_source = [
        include_bytes!("phenotype.rs").as_slice(),
        include_bytes!("geometry.rs").as_slice(),
        include_bytes!("program.rs").as_slice(),
        include_bytes!("../../../Cargo.lock").as_slice(),
        include_bytes!("../../joil-ir/src/lib.rs").as_slice(),
    ]
    .concat();
    let operator_source = [
        outcome.operator_id.as_bytes(),
        outcome.operator_version.as_bytes(),
        include_bytes!("program.rs").as_slice(),
    ]
    .concat();
    let lineage = Lineage {
        parents: if outcome.parent_ids.is_empty() {
            outcome
                .parent_hashes
                .iter()
                .map(|hash| format!("genome:{hash}"))
                .collect::<Vec<_>>()
        } else {
            outcome.parent_ids.clone()
        }
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect(),
        operator: ContentRef::from_bytes(
            &outcome.operator_id,
            &outcome.operator_version,
            &operator_source,
        ),
        operator_version: outcome
            .operator_version
            .parse::<u32>()
            .map_err(|e| e.to_string())?,
        seed,
        rng_span: [outcome.rng_span.before, outcome.rng_span.after],
        rng_draws: Rng::draws_between(outcome.rng_span.before, outcome.rng_span.after),
        repairs: outcome
            .repairs
            .iter()
            .map(|r| joil_evidence::subject::RepairRecord {
                rule: r.rule.clone(),
                before_hash: r.before_hash.clone(),
                after_hash: r.after_hash.clone(),
            })
            .collect(),
        compiler: ContentRef::from_bytes(
            "joil-core/legacy-mesh-compiler-source",
            env!("CARGO_PKG_VERSION"),
            &compiler_source,
        ),
    };
    lineage.check()?;
    Ok(lineage)
}

pub(crate) fn lognormal_bin_weights(d50_feed_um: f64, sigma_ln: f64) -> [f64; 10] {
    let mut weights = [0.0_f64; 10];
    let mu = d50_feed_um.max(2.0).ln();
    let mut sum = 0.0_f64;
    for (i, &d) in STANDARD_BINS_UM.iter().enumerate() {
        let z = (d.ln() - mu) / (sigma_ln.max(0.15) * std::f64::consts::SQRT_2);
        let pdf = (-z * z).exp() / (d * sigma_ln.max(0.15) * (2.0 * std::f64::consts::PI).sqrt());
        let width = if i == 0 {
            3.0
        } else {
            STANDARD_BINS_UM[i] - STANDARD_BINS_UM[i - 1]
        };
        weights[i] = (pdf * width).max(1e-9);
        sum += weights[i];
    }
    for w in &mut weights {
        *w /= sum;
    }
    weights
}

/// Evaluates a single realization (nominal or perturbed Monte Carlo sample) of the coupled
/// 3D swirl-shear-breakup-migration physics model anchored to the OpenFOAM 14 baseline.
pub fn evaluate_realization(
    prog: &DesignProgram,
    desc: &MechanismDescriptors,
    base_hyd: &BaselineHydraulics,
    cin_mg_l: f64,
    d50_feed_um: f64,
    split_delta: f64,
    wall_tol_mm: f64,
) -> CandidateMetrics {
    let q_in_m3_s = base_hyd.q_in_m3_s;
    let q_in_m3_h = q_in_m3_s * 3600.0;

    // Effective split ratio under skimmer / hydraulic resistance and MC perturbation
    let skim_split_bias = 1.0 + 0.015 * desc.core_shield_ratio - 0.008 * (wall_tol_mm / 0.05);
    let split_nom = (base_hyd.split_ratio * skim_split_bias + split_delta).clamp(0.015, 0.032);

    // Entry shear rate G_entry (s^-1) and Kolmogorov-Hinze breakup diameter d_max (um)
    let base_shear_s_inv = 4850.0_f64;
    let max_entry_shear_s_inv =
        (base_shear_s_inv * desc.shear_mitigation_factor * (1.0 - 0.04 * wall_tol_mm))
            .clamp(2400.0, 9500.0);
    let shear_ratio = (max_entry_shear_s_inv / base_shear_s_inv).max(0.40);
    let hinze_dmax_um = (64.0_f64 * shear_ratio.powf(-0.62)).clamp(18.0, 130.0);

    // Hydraulic pressure drops dp_u and dp_o
    let dp_u_pa = (base_hyd.dp_u_pa * desc.hydraulic_resistance_factor).max(20_000.0);
    let dp_o_pa = (base_hyd.dp_o_pa
        * (0.94 + 0.06 * desc.swirl_flux_index.powi(2))
        * (1.0 + 0.03 * desc.core_shield_ratio))
        .max(dp_u_pa * 2.5);
    let pdr = dp_o_pa / dp_u_pa.max(1.0);
    let specific_energy_j_m3 = (1.0 - split_nom) * dp_u_pa + split_nom * dp_o_pa;

    // Swirl preservation & reverse-core coherence
    let sym_gain = 1.0 + 0.038 * (desc.azimuthal_symmetry_order - 2.0).max(0.0);
    let pitch_gain = 1.0 + 0.28 * desc.axial_pitch_sin;
    let rifle_gain = 1.0 + 0.095 * desc.wall_rifling_stabilization;
    let reverse_core_coherence = (0.76
        * desc.swirl_flux_index
        * sym_gain
        * pitch_gain
        * rifle_gain
        * desc.cone_accel_persistence)
        .clamp(0.45, 1.45);

    // Lip short-circuit fraction around the vortex finder
    let shield_relief = (1.0 - 0.36 * desc.core_shield_ratio).clamp(0.45, 1.0);
    let short_circuit_fraction = (0.048 * shield_relief / sym_gain).clamp(0.010, 0.14);

    // Effective droplet feed distribution after entry shear breakup
    let eff_d50_feed = d50_feed_um.min(0.68 * hinze_dmax_um);
    let weights = lognormal_bin_weights(eff_d50_feed, 0.42);

    // Separation number by droplet bin (calibrated to CT-35 baseline: capture ~ 0.9400, Cout ~ 30.61 mg/L)
    let mut grade_efficiency_bins = BTreeMap::new();
    let mut overall_oil_capture = 0.0_f64;
    let mut prev_d = 0.0_f64;
    let mut prev_eta = 0.0_f64;
    let mut d50_um = 100.0_f64;
    let mut d90_um = 100.0_f64;

    let is_base = prog.is_exact_baseline().unwrap_or(false);
    let sep_factor = if is_base && wall_tol_mm.abs() < 1e-9 && split_delta.abs() < 1e-9 {
        1.0
    } else {
        (reverse_core_coherence / 0.76).powf(1.55)
            * (split_nom / base_hyd.split_ratio).powf(0.36)
            * (1.0 - short_circuit_fraction)
            / (1.0 - 0.048)
    };

    for (i, &d_um) in STANDARD_BINS_UM.iter().enumerate() {
        let d_eff = if d_um > hinze_dmax_um {
            hinze_dmax_um + 0.25 * (d_um - hinze_dmax_um)
        } else {
            d_um
        };
        let stokes_num = (d_eff / 11.45).powi(2) * sep_factor;
        let eta_raw = 1.0 - (-std::f64::consts::LN_2 * stokes_num).exp();
        let eta = (split_nom + (1.0 - split_nom) * eta_raw * (1.0 - 0.35 * short_circuit_fraction))
            .clamp(split_nom, 0.9995);

        if prev_eta < 0.50 && eta >= 0.50 {
            let frac = (0.50 - prev_eta) / (eta - prev_eta).max(1e-9);
            d50_um = prev_d + frac * (d_um - prev_d);
        }
        if prev_eta < 0.90 && eta >= 0.90 {
            let frac = (0.90 - prev_eta) / (eta - prev_eta).max(1e-9);
            d90_um = prev_d + frac * (d_um - prev_d);
        }
        prev_d = d_um;
        prev_eta = eta;

        grade_efficiency_bins.insert(format!("{:.0}um", d_um), (eta * 1e6).round() / 1e6);
        overall_oil_capture += weights[i] * eta;
    }

    overall_oil_capture = overall_oil_capture.clamp(split_nom, 0.999);
    let cout_mg_l =
        metrics::concentration_mg_l(cin_mg_l, overall_oil_capture, split_nom).unwrap_or(cin_mg_l);

    CandidateMetrics {
        q_in_m3_h,
        split_ratio: split_nom,
        dp_u_pa,
        dp_o_pa,
        pdr,
        specific_energy_j_m3,
        max_entry_shear_s_inv,
        hinze_dmax_um,
        short_circuit_fraction,
        reverse_core_coherence,
        grade_efficiency_bins,
        d50_um,
        d90_um,
        overall_oil_capture,
        cout_mg_l,
        passes_30_mg_l: cout_mg_l
            <= crate::experiment::research_requirements()
                .expect("compiled requirements must validate")
                .thresholds
                .quality_limit_mg_l
                .get(),
    }
}

pub(crate) fn percentile_sorted(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let pos = q.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    let frac = pos - lo as f64;
    sorted[lo] * (1.0 - frac) + sorted[hi] * frac
}

/// Evaluates an `Individual` across nominal + `mc_samples` Monte Carlo realizations
/// and runs the F0 red-team and 3D phenotype checks.
pub fn evaluate_individual(
    id: String,
    raw_prog: DesignProgram,
    generation_born: usize,
    island_origin: usize,
    cfg: &EvolutionConfig,
) -> Result<(Individual, Option<SolidMesh>), String> {
    evaluate_individual_recorded(id, raw_prog, generation_born, island_origin, cfg, None)
}

pub(crate) fn evaluate_individual_recorded(
    id: String,
    raw_prog: DesignProgram,
    generation_born: usize,
    island_origin: usize,
    cfg: &EvolutionConfig,
    mut raw_mc: Option<&mut Vec<serde_json::Value>>,
) -> Result<(Individual, Option<SolidMesh>), String> {
    let prog = raw_prog.normalize()?;
    let genome_sha256 = prog.canonical_sha256()?;
    let descriptors = prog.descriptors()?;
    let (pheno, mesh) = Phenotype3D::from_program(&prog)?;

    let (red_team_passed, red_team_rejects) = match redteam::check(&prog, &pheno, &mesh) {
        Ok(()) => (true, Vec::new()),
        Err(rejs) => (false, rejs.into_iter().map(|r| format!("{r:?}")).collect()),
    };
    let red_team_unevaluated_facts = redteam::unevaluated_geometry_facts(&pheno);

    let nominal_metrics = evaluate_realization(
        &prog,
        &descriptors,
        &cfg.baseline_hydraulics,
        500.0,
        38.0,
        0.0,
        0.0,
    );

    let mut mc_rng = Rng::new(
        cfg.seed ^ u64::from_str_radix(&genome_sha256[0..16], 16).unwrap_or(0x1234_5678_9abc_def0),
    );
    let n_mc = cfg.mc_samples;
    if n_mc == 0 {
        return Err("positive declared Monte Carlo sample count required".into());
    }
    let mut couts = Vec::with_capacity(n_mc);
    let mut captures = Vec::with_capacity(n_mc);
    let mut d50s = Vec::with_capacity(n_mc);
    let mut eta20s = Vec::with_capacity(n_mc);
    let mut pass_count = 0usize;

    for draw_index in 0..n_mc {
        let cin = mc_rng.range(475.0, 525.0);
        let d50_feed = mc_rng.range(35.0, 41.0);
        let split_delta = mc_rng.range(-0.0012, 0.0012);
        let wall_tol = mc_rng.range(-0.05, 0.05);
        let m = evaluate_realization(
            &prog,
            &descriptors,
            &cfg.baseline_hydraulics,
            cin,
            d50_feed,
            split_delta,
            wall_tol,
        );
        if let Some(rows) = raw_mc.as_mut() {
            rows.push(serde_json::json!({
                "draw_index": draw_index,
                "cin_mg_l": cin, "feed_d50_um": d50_feed,
                "split_delta": split_delta, "wall_tolerance_fraction": wall_tol,
                "metrics": m,
            }));
        }
        if m.passes_30_mg_l {
            pass_count += 1;
        }
        couts.push(m.cout_mg_l);
        captures.push(m.overall_oil_capture);
        d50s.push(m.d50_um);
        eta20s.push(*m.grade_efficiency_bins.get("20um").unwrap_or(&0.0));
    }

    couts.sort_by(|a, b| a.total_cmp(b));
    captures.sort_by(|a, b| a.total_cmp(b));
    d50s.sort_by(|a, b| a.total_cmp(b));
    eta20s.sort_by(|a, b| a.total_cmp(b));

    let requirements = crate::experiment::research_requirements()?;
    let policy = &requirements.quantile_policy;
    let mut tail_quantiles = BTreeMap::new();
    for (name, samples) in [
        ("cout", &couts),
        ("capture", &captures),
        ("d50", &d50s),
        ("grade_20um", &eta20s),
    ] {
        for (suffix, quantile, minimum_n) in [
            ("p05", 0.05, policy.min_n_p05_p95),
            ("p95", 0.95, policy.min_n_p05_p95),
            ("p01", 0.01, policy.min_n_p01_p99),
            ("p99", 0.99, policy.min_n_p01_p99),
        ] {
            tail_quantiles.insert(
                format!("{name}_{suffix}"),
                metrics::quantile_estimate(samples, quantile, policy.confidence.get(), minimum_n)?,
            );
        }
    }
    // Small cohorts supply only explicitly named sample-envelope screens. They
    // never stand in for missing tail estimates in the objective vector.
    let lower = |samples: &[f64]| {
        if n_mc < policy.min_n_p05_p95 {
            samples[0]
        } else {
            percentile_sorted(samples, 0.05)
        }
    };
    let upper = |samples: &[f64]| {
        if n_mc < policy.min_n_p05_p95 {
            samples[n_mc - 1]
        } else {
            percentile_sorted(samples, 0.95)
        }
    };
    let mc_robustness = MonteCarloSummary {
        samples: n_mc,
        pass_count,
        sampled_pass_fraction_ci95: metrics::sampled_fraction_ci95(pass_count, n_mc)?,
        sampling_interpretation:
            "conditional_on_declared_sandbox_draws_not_empirical_physical_risk".into(),
        tail_quantiles,
        cout_p05_mg_l: lower(&couts),
        cout_p50_mg_l: percentile_sorted(&couts, 0.50),
        cout_p95_mg_l: upper(&couts),
        capture_p05: lower(&captures),
        capture_p50: percentile_sorted(&captures, 0.50),
        capture_p95: upper(&captures),
        d50_p05_um: lower(&d50s),
        d50_p50_um: percentile_sorted(&d50s, 0.50),
        d50_p95_um: upper(&d50s),
        grade_20um_p05: lower(&eta20s),
        probability_passes_30_mg_l: pass_count as f64 / n_mc as f64,
    };

    let requirements = crate::experiment::research_requirements()?;
    let observations = BTreeMap::from([
        (
            "cout_p95_mg_l".into(),
            joil_evidence::observation::Finite::new(mc_robustness.cout_p95_mg_l)?,
        ),
        (
            "specific_energy_j_m3".into(),
            joil_evidence::observation::Finite::new(nominal_metrics.specific_energy_j_m3)?,
        ),
        (
            "grade_20um_p05".into(),
            joil_evidence::observation::Finite::new(mc_robustness.grade_20um_p05)?,
        ),
        (
            "d50_p95_um".into(),
            joil_evidence::observation::Finite::new(mc_robustness.d50_p95_um)?,
        ),
        (
            "dp_u_pa".into(),
            joil_evidence::observation::Finite::new(nominal_metrics.dp_u_pa)?,
        ),
    ]);
    let missing_quantiles: Vec<String> = ["cout_p95", "grade_20um_p05", "d50_p95"]
        .into_iter()
        .filter(|id| mc_robustness.tail_quantiles[*id].value().is_none())
        .map(str::to_string)
        .collect();
    let objectives = if missing_quantiles.is_empty() {
        ObjectiveOutcome::SandboxEstimate {
            values: requirements
                .evaluate_objectives(&observations)?
                .into_iter()
                .map(|v| v.get())
                .collect::<Vec<_>>()
                .try_into()
                .map_err(|_| "expected frozen five-objective contract")?,
        }
    } else {
        ObjectiveOutcome::Insufficient { missing_quantiles }
    };

    let mut violation = 0.0_f64;
    if !red_team_passed {
        violation += 1000.0 + 100.0 * red_team_rejects.len() as f64;
    }
    if nominal_metrics.cout_mg_l > requirements.thresholds.quality_limit_mg_l.get() {
        violation += nominal_metrics.cout_mg_l - requirements.thresholds.quality_limit_mg_l.get();
    }
    let max_dp_u =
        cfg.baseline_hydraulics.dp_u_pa * requirements.thresholds.sandbox_pressure_ratio_max.get();
    if nominal_metrics.dp_u_pa > max_dp_u {
        violation += (nominal_metrics.dp_u_pa - max_dp_u) / 1000.0;
    }
    if nominal_metrics.pdr < requirements.thresholds.sandbox_pdr_min.get() {
        violation += (requirements.thresholds.sandbox_pdr_min.get() - nominal_metrics.pdr) * 10.0;
    }

    let (structural_qualification, structural_screen_state) =
        match structure::analytic_structural_screen(
            &prog,
            &pheno,
            nominal_metrics.dp_u_pa,
            cfg.baseline_hydraulics.q_in_m3_s,
        ) {
            Ok(screen) => {
                if !screen.passed {
                    violation += 1.0 + screen.failure_reasons.len() as f64;
                }
                (Some(screen), StructuralScreenState::Screened)
            }
            Err(reason) => {
                violation += 1.0;
                (None, StructuralScreenState::Failed { reason })
            }
        };

    let ind = Individual {
        lineage: Some(proposal_lineage(
            &VariationOutcome::introduce(
                prog.clone(),
                "evaluate_external_program",
                &mut Rng::new(cfg.seed),
            )?,
            cfg.seed,
        )?),
        id,
        topology_class: prog.topology_class.clone(),
        program: prog,
        genome_sha256,
        generation_born,
        island_origin,
        red_team_passed,
        red_team_rejects,
        red_team_unevaluated_facts,
        descriptors,
        phenotype: Some(pheno),
        structural_qualification,
        structural_screen_state,
        nominal_metrics,
        mc_robustness,
        cuda_sdf_audit: None,
        objectives,
        constraint_violation: violation,
        rank: usize::MAX,
        crowding_distance: 0.0,
        evidence_tier: EvidenceTier::SoftwareContract,
        qualified_multiobjective_vector: None,
        qualified_capacity_gain: None,
    };

    Ok((ind, Some(mesh)))
}
