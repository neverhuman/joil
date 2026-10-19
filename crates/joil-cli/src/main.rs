use joil_core::{
    cfd_case, evolve, experiment, geometry, metrics, openfoam, phenotype, population, program,
    redteam, scene, structure, surrogate,
};
use std::path::PathBuf;

fn parse_arg(args: &[String], flag: &str) -> Option<String> {
    args.windows(2).find(|w| w[0] == flag).map(|w| w[1].clone())
}

/// GPU use is explicit opt-in (`V5-01-04`). Without `--cuda-bin` no GPU is requested.
/// `--cuda-bin` needs an existing binary and `--gpu-lease <json>` with `"gpu": true` and an
/// `expires_utc` that parses strictly as UTC (see `parse_utc_expiry_seconds`) and lies after
/// `now_unix_seconds`. `--gpu-vram-target-pct` is passed only when given. Anything incomplete or
/// malformed is an error, never a silent CPU or GPU fallback.
fn explicit_gpu_opt_in(
    args: &[String],
    now_unix_seconds: i64,
) -> Result<(Option<PathBuf>, Option<f64>), String> {
    let lease_arg = parse_arg(args, "--gpu-lease");
    let vram_arg = parse_arg(args, "--gpu-vram-target-pct");
    let Some(bin) = parse_arg(args, "--cuda-bin") else {
        if lease_arg.is_some() || vram_arg.is_some() {
            return Err("--gpu-lease and --gpu-vram-target-pct require --cuda-bin".into());
        }
        return Ok((None, None));
    };
    let bin = PathBuf::from(bin);
    if !bin.is_file() {
        return Err(format!("--cuda-bin {} is not a file", bin.display()));
    }
    let lease_path =
        lease_arg.ok_or("--cuda-bin requires --gpu-lease <lease.json> granting gpu")?;
    let raw = std::fs::read_to_string(&lease_path)
        .map_err(|e| format!("cannot read GPU lease {lease_path}: {e}"))?;
    let lease: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("invalid GPU lease {lease_path}: {e}"))?;
    if lease.get("gpu") != Some(&serde_json::Value::Bool(true)) {
        return Err(format!(
            "GPU lease {lease_path} does not grant \"gpu\": true"
        ));
    }
    let expires = lease
        .get("expires_utc")
        .and_then(|v| v.as_str())
        .ok_or(format!("GPU lease {lease_path} lacks expires_utc"))?;
    let expires_at = parse_utc_expiry_seconds(expires).map_err(|reason| {
        format!("GPU lease {lease_path} has invalid expires_utc {expires:?}: {reason}")
    })?;
    if expires_at <= now_unix_seconds {
        return Err(format!("GPU lease {lease_path} expired at {expires}"));
    }
    let vram = match vram_arg {
        None => None,
        Some(s) => match s.parse::<f64>() {
            Ok(v) if v.is_finite() && v > 0.0 && v <= 100.0 => Some(v),
            _ => return Err(format!("--gpu-vram-target-pct must be in (0, 100]: {s}")),
        },
    };
    Ok((Some(bin), vram))
}

/// Output policy (`V5-01-05`): a default is a unique ignored path under `runs/`; an explicit path
/// must not exist yet. Commands call this before evaluating, so committed receipts and earlier runs
/// are never replaced. `None` default means the output is written only when the flag is given.
fn fresh_output(
    args: &[String],
    flag: &str,
    default_stem: Option<&str>,
    extension: &str,
) -> Result<Option<PathBuf>, String> {
    let path = match (parse_arg(args, flag), default_stem) {
        (Some(p), _) => PathBuf::from(p),
        (None, Some(stem)) => PathBuf::from(format!("runs/{stem}-{}{extension}", run_stamp()?)),
        (None, None) => return Ok(None),
    };
    if path.exists() || path.is_symlink() {
        return Err(format!(
            "refusing to replace existing output {}",
            path.display()
        ));
    }
    Ok(Some(path))
}

/// Same policy for a required output path.
fn fresh_required_output(
    args: &[String],
    flag: &str,
    default_stem: &str,
    extension: &str,
) -> Result<PathBuf, String> {
    fresh_output(args, flag, Some(default_stem), extension)
        .map(|p| p.expect("default stem always yields a path"))
}

fn run_stamp() -> Result<String, String> {
    Ok(format!(
        "{}Z-{}",
        utc_now_seconds()?.replace(['-', ':'], ""),
        std::process::id()
    ))
}

/// Create `path` (and parent directories) and write `text`; an existing file is an error.
fn write_new(path: &std::path::Path, text: &str) -> Result<(), String> {
    use std::io::Write;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .and_then(|mut f| f.write_all(text.as_bytes()))
        .map_err(|e| format!("cannot create new output {}: {e}", path.display()))
}

fn write_optional_new(path: &Option<PathBuf>, text: &str) -> Result<(), String> {
    match path {
        Some(p) => write_new(p, text),
        None => Ok(()),
    }
}

/// Parse a GPU lease expiry as Unix seconds (`V5-01-04`). Accepted form, ASCII only:
/// `YYYY-MM-DDTHH:MM:SS` with an optional `.fraction` of 1-9 digits, then `Z` or `+00:00`.
/// The calendar date (including leap years) and time of day are validated; any other offset,
/// a missing zone, trailing characters or out-of-range fields are rejected. A fraction is
/// truncated, which can only make the expiry earlier.
fn parse_utc_expiry_seconds(text: &str) -> Result<i64, String> {
    if !text.is_ascii() {
        return Err("not ASCII".into());
    }
    let syntax = || {
        "malformed timestamp syntax (expected YYYY-MM-DDTHH:MM:SS[.fraction] then Z or +00:00)"
            .to_string()
    };
    let b = text.as_bytes();
    if b.len() < 19
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
    {
        return Err(syntax());
    }
    let field = |lo: usize, hi: usize| -> Result<i64, String> {
        let digits = &b[lo..hi];
        if !digits.iter().all(u8::is_ascii_digit) {
            return Err(syntax());
        }
        Ok(digits
            .iter()
            .fold(0_i64, |acc, d| acc * 10 + i64::from(d - b'0')))
    };
    let (year, month, day) = (field(0, 4)?, field(5, 7)?, field(8, 10)?);
    let (hour, minute, second) = (field(11, 13)?, field(14, 16)?, field(17, 19)?);
    let mut zone = &b[19..];
    if let Some(fraction) = zone.strip_prefix(b".") {
        let n = fraction.iter().take_while(|c| c.is_ascii_digit()).count();
        if n == 0 || n > 9 {
            return Err(syntax());
        }
        zone = &fraction[n..];
    }
    match zone {
        b"Z" | b"+00:00" => {}
        b"" => return Err("missing time zone designator (Z or +00:00 required)".into()),
        [b'+' | b'-', _, _, b':', _, _] => {
            return Err("offset is not UTC (Z or +00:00 required)".into())
        }
        _ => return Err(syntax()),
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    if year < 1 || !(1..=days_in_month).contains(&day) {
        return Err(format!(
            "invalid calendar date {year:04}-{month:02}-{day:02}"
        ));
    }
    if hour > 23 || minute > 59 || second > 59 {
        return Err(format!(
            "invalid time of day {hour:02}:{minute:02}:{second:02}"
        ));
    }
    // Days from civil date (proleptic Gregorian), inverse of `utc_now_seconds`.
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let days = era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468;
    Ok(days * 86_400 + hour * 3_600 + minute * 60 + second)
}

fn utc_now_unix_seconds() -> Result<i64, String> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64)
}

/// Current UTC time as `YYYY-MM-DDTHH:MM:SS` (civil-from-days, proleptic Gregorian).
fn utc_now_seconds() -> Result<String, String> {
    let secs = utc_now_unix_seconds()?;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    Ok(format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}",
        rem / 3_600,
        rem % 3_600 / 60,
        rem % 60
    ))
}

/// Import a historical Wave-2 genome without upgrading its archived evidence.
///
/// Report/Individual evaluation schemas have changed since the committed v1
/// receipt. CFD preparation needs only the selected program: validate and
/// canonicalize that program, then let the case builder compute its current
/// design_sha256. Archived genome hashes (which included authored topology
/// labels), metrics, lineage and qualification flags are not imported.
fn load_wave2_elite_program(
    raw: &str,
    family: &str,
) -> Result<program::DesignProgram, Box<dyn std::error::Error>> {
    let report: serde_json::Value = serde_json::from_str(raw)?;
    let elites = report
        .get("promoted_wave2_elites_by_family")
        .and_then(serde_json::Value::as_object)
        .ok_or("Wave-2 receipt has no promoted_wave2_elites_by_family object")?;
    let elite = elites
        .get(family)
        .ok_or_else(|| format!("Wave-2 family '{family}' not found"))?;
    let genome = elite
        .get("program")
        .ok_or_else(|| format!("Wave-2 family '{family}' has no program"))?;
    let program: program::DesignProgram = serde_json::from_value(genome.clone())?;
    Ok(program.canonicalize()?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let command = match args.get(1).cloned() {
        Some(value) => value,
        None => "baseline".into(),
    };
    match command.as_str() {
        "baseline" => {
            let g = geometry::baseline();
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status":"analytic_geometry_only_not_cfd_validated",
                    "genome_sha256":g.sha256()?,"dimensions":g.derive()?,
                    "regression_example":{"cin_mg_l":500,"capture":0.94,"split":0.02,
                      "cout_mg_l":metrics::concentration_mg_l(500.0,0.94,0.02)?,"passes_30_mg_l":false}
                }))?
            );
        }
        "kpi" => {
            let out = fresh_output(&args, "--out", None, ".json")?;
            let root = PathBuf::from(match parse_arg(&args, "--root") { Some(value) => value, None => ".".into() });
            let cases = [
                (
                    "runs/ct35-sst-diagnostic-000/hydraulic-integrals-t500.json",
                    500u64,
                ),
                (
                    "runs/ct35-sst-split-001/hydraulic-integrals-t300.json",
                    300u64,
                ),
                (
                    "runs/ct35-sst-extended-000/hydraulic-integrals-t600.json",
                    600u64,
                ),
            ];
            let mut results = Vec::new();
            for (rel, step) in cases {
                let eval = openfoam::read_cfd_kpi_observation(&root, &root.join(rel), step)
                    .map_err(|observation| serde_json::json!({"observation": observation}).to_string())?;
                results.push(eval);
            }
            let out_json = serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": "0.2.0",
                "evaluator": "joil-core::openfoam::read_cfd_kpi_observation",
                "cases": results,
            }))?;
            write_optional_new(&out, &out_json)?;
            println!("{out_json}");
        }
        "verify-rotation" => {
            let out = fresh_output(&args, "--out", None, ".json")?;
            let ode_20 = openfoam::integrate_coupled_rotation_ode(20.0, 0.01, 200.0, 2e-6, 0.002);
            let ode_40 = openfoam::integrate_coupled_rotation_ode(40.0, 0.01, 200.0, 2e-6, 0.002);
            let ratio_d2 =
                (ode_40.u_r_end_m_s / ode_40.r_end_m) / (ode_20.u_r_end_m_s / ode_20.r_end_m);
            let out_json = serde_json::to_string_pretty(&serde_json::json!({
                "oracle": "joil-core::openfoam::integrate_coupled_rotation_ode",
                "d20_um": ode_20,
                "d40_um": ode_40,
                "d40_to_d20_drift_rate_ratio": ratio_d2,
                "inward_migration_verified": ode_20.radial_displacement_m < 0.0
                    && ode_40.radial_displacement_m < ode_20.radial_displacement_m
                    && ode_20.u_r_end_m_s < 0.0
                    && ode_40.u_r_end_m_s < 0.0,
            }))?;
            write_optional_new(&out, &out_json)?;
            println!("{out_json}");
        }
        "screen-structure" | "qualify-structure" => {
            let out = fresh_output(&args, "--out", None, ".json")?;
            let mut rng = program::Rng::new(20261015);
            let archetypes = program::DesignProgram::seed_wave2_archetypes(&mut rng);
            let lame_inner =
                structure::lame_thick_cylinder_oracle(0.035, 0.038, 0.0, 2.5e6, 0.035)?;
            let lame_outer =
                structure::lame_thick_cylinder_oracle(0.035, 0.038, 0.0, 2.5e6, 0.038)?;
            let mut reports = Vec::new();
            for (idx, prog) in archetypes.iter().enumerate() {
                let (pheno, _) = phenotype::Phenotype3D::from_program(prog)?;
                let sq = structure::analytic_structural_screen(prog, &pheno, 29468.22, 0.001111111)?;
                reports.push(serde_json::json!({
                    "index": idx,
                    "topology_class": pheno.topology_class,
                    "genome_sha256": pheno.design_sha256,
                    "analytic_structural_screen": sq,
                }));
            }
            let out_json = serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": "0.2.0",
                "evaluator": "joil-core::structure::analytic_structural_screen",
                "lame_cylinder_oracle_head_35mm_id_38mm_od_2.5mpa": {
                    "inner_wall": lame_inner,
                    "outer_wall": lame_outer,
                },
                "archetype_analytic_screens": reports,
            }))?;
            write_optional_new(&out, &out_json)?;
            println!("{out_json}");
        }
        "audit-probe" => {
            let out = fresh_output(&args, "--out", None, ".json")?;
            let root = PathBuf::from(match parse_arg(&args, "--root") { Some(value) => value, None => ".".into() });
            let summary_path = PathBuf::from(match parse_arg(&args, "--summary") { Some(value) => value, None => "runs/gemini-ct35-full-liner-oil-pilot-summary-r4.json".into() });
            let audit = openfoam::audit_lagrangian_probe_summary(&summary_path, &root)?;
            let out_json = serde_json::to_string_pretty(&audit)?;
            write_optional_new(&out, &out_json)?;
            println!("{out_json}");
            if !audit.audit_passed {
                return Err(format!("probe audit failed: {:?}", audit.failure_reasons).into());
            }
        }
        "promote-elite" => {
            let summary_path = PathBuf::from(
                match parse_arg(&args, "--summary") { Some(value) => value, None => "runs/rust-cuda-evolution-v1/campaign-summary.json".into() },
            );
            let out_path = fresh_required_output(&args, "--out", "promotion-dossier", ".json")?;
            let raw = std::fs::read_to_string(&summary_path)?;
            let campaign: evolve::CampaignSummary = serde_json::from_str(&raw)?;
            let elite = campaign
                .top_pareto_elites
                .first()
                .ok_or("no top_pareto_elites in campaign summary")?;
            let base = &campaign.baseline_reference;

            let (pheno, mesh) = phenotype::Phenotype3D::from_program(&elite.program)?;
            redteam::check(&elite.program, &pheno, &mesh)
                .map_err(|e| format!("top elite failed redteam gate: {e:?}"))?;
            let f0_unevaluated = redteam::unevaluated_geometry_facts(&pheno);
            let claim_state = if f0_unevaluated.is_empty() {
                "f0_geometry_screen_and_p08_analytic_structural_estimate_passed_ready_for_f2_cfd_promotion_not_yet_f2_or_f4_validated"
            } else {
                "f0_partial_sandbox_screen_and_p08_analytic_structural_estimate_passed_with_unevaluated_geometry_facts_not_qualified"
            };
            let struct_qual = structure::analytic_structural_screen(
                &elite.program,
                &pheno,
                elite.nominal_metrics.dp_u_pa,
                campaign.config.baseline_hydraulics.q_in_m3_s,
            )?;
            if !struct_qual.passed {
                return Err(format!(
                    "top candidate failed the P08 analytic structural screen: {:?}",
                    struct_qual.failure_reasons
                )
                .into());
            }
            let multi_patch = phenotype::MultiPatchSurface::from_program(&elite.program, 0.0)?;
            if !multi_patch.audit.watertight_and_manifold
                || !multi_patch.audit.all_terminal_patches_valid
            {
                return Err("top elite failed multi-patch CFD surface gate (CAD-04/CAD-06)".into());
            }

            let brep_report_path = (match summary_path.parent() { Some(path) => path, None => std::path::Path::new(".") })
                .join("cad/top_elite_brep_report.json");
            let brep_audit: Option<serde_json::Value> = if brep_report_path.is_file() {
                Some(serde_json::from_str(&std::fs::read_to_string(
                    &brep_report_path,
                )?)?)
            } else {
                None
            };

            let dossier = serde_json::json!({
                "schema_version": "0.3.0",
                "kind": "multi_fidelity_3d_elite_promotion_dossier",
                "claim_state": claim_state,
                "f0_unevaluated_geometry_facts": f0_unevaluated,
                "qualified_multiobjective_vector": null,
                "qualified_capacity_gain": null,
                "campaign_summary_sha256": openfoam::file_sha256(&summary_path)?,
                "promoted_candidate": {
                    "id": elite.id,
                    "topology_class": elite.topology_class,
                    "genome_sha256": elite.genome_sha256,
                    "generation_born": elite.generation_born,
                    "island_origin": elite.island_origin,
                    "program": elite.program,
                    "descriptors": elite.descriptors,
                    "phenotype_3d": pheno,
                    "multi_patch_cfd_surface_audit": multi_patch.audit,
                    "analytic_structural_screen_p08": struct_qual,
                    "nominal_f1_metrics": elite.nominal_metrics,
                    "monte_carlo_robustness": elite.mc_robustness,
                    "cuda_rtx3090_sdf_and_ode_audit": elite.cuda_sdf_audit,
                    "opencascade_brep_step_audit": brep_audit,
                },
                "baseline_ct35_comparator": {
                    "id": base.id,
                    "genome_sha256": base.genome_sha256,
                    "nominal_f1_metrics": base.nominal_metrics,
                    "monte_carlo_robustness": base.mc_robustness,
                    "cuda_rtx3090_sdf_and_ode_audit": base.cuda_sdf_audit,
                },
                "f1_screen_deltas_vs_baseline_not_qualified_gain": {
                    "cout_nominal_delta_mg_l": elite.nominal_metrics.cout_mg_l - base.nominal_metrics.cout_mg_l,
                    "cout_p95_delta_mg_l": elite.mc_robustness.cout_p95_mg_l - base.mc_robustness.cout_p95_mg_l,
                    "d50_nominal_delta_um": elite.nominal_metrics.d50_um - base.nominal_metrics.d50_um,
                    "grade_20um_p05_delta": elite.mc_robustness.grade_20um_p05 - base.mc_robustness.grade_20um_p05,
                    "entry_shear_delta_s_inv": elite.nominal_metrics.max_entry_shear_s_inv - base.nominal_metrics.max_entry_shear_s_inv,
                    "dp_u_delta_pa": elite.nominal_metrics.dp_u_pa - base.nominal_metrics.dp_u_pa,
                },
                "required_next_gates_before_claim": [
                    "P04: 3-grid systematic snappyHexMesh + OpenFOAM 14 LRR/SSG Reynolds-Stress carrier convergence on promoted STEP geometry",
                    "P05: Full-residence-time flux-weighted inlet 10-bin Lagrangian droplet cohort closure on converged F2 carrier",
                    "P06: Blinded A/B/A physical rig validation under EXP-RIG-JOIL-V1-DRAFT"
                ]
            });
            let out_str = serde_json::to_string_pretty(&dossier)?;
            write_new(&out_path, &out_str)?;
            println!("{out_str}");
        }
        "prepare-cfd-case" => {
            let case_dir = fresh_required_output(&args, "--case", "cfd-case", "")?;
            let out = fresh_output(&args, "--out", None, ".json")?;
            let prog = if args.iter().any(|a| a == "--baseline-ct35") {
                program::DesignProgram::baseline_ct35()
            } else if let Some(prog_path) = parse_arg(&args, "--program-json") {
                let raw = std::fs::read_to_string(&prog_path)?;
                let p: program::DesignProgram = serde_json::from_str(&raw)?;
                p.canonicalize()?
            } else if let Some(w2_family) = parse_arg(&args, "--wave2-family") {
                let w2_path = PathBuf::from(match parse_arg(&args, "--wave2-ablation") { Some(value) => value, None => "docs/results/wave2-multiseed-ablation-and-holdout-v1.json".into() });
                let raw = std::fs::read_to_string(&w2_path)?;
                load_wave2_elite_program(&raw, &w2_family)
                    .map_err(|e| format!("{}: {e}", w2_path.display()))?
            } else {
                let summary_path = PathBuf::from(
                    match parse_arg(&args, "--summary") { Some(value) => value, None => "runs/rust-cuda-evolution-v1/campaign-summary.json".into() },
                );
                let raw = std::fs::read_to_string(&summary_path)?;
                let campaign: evolve::CampaignSummary = serde_json::from_str(&raw)?;
                let elite = campaign
                    .top_pareto_elites
                    .first()
                    .ok_or("no top_pareto_elites in campaign summary")?;
                elite.program.clone()
            };
            let mut cfg = cfd_case::CfdCaseConfig::default();
            if let Some(p_pa) = parse_arg(&args, "--reject-pressure-pa").and_then(|s| s.parse().ok())
            {
                cfg.reject_pressure_pa = p_pa;
            }
            if let Some(steps) = parse_arg(&args, "--steps").and_then(|s| s.parse().ok()) {
                cfg.steps = steps;
            }
            if let Some(model) = parse_arg(&args, "--model") {
                cfg.turbulence_model = model;
            }
            if let Some(sub) = parse_arg(&args, "--subdomains").and_then(|s| s.parse().ok()) {
                cfg.n_subdomains = sub;
            }
            let manifest = cfd_case::build_openfoam_candidate_case(&prog, &case_dir, &cfg)?;
            let out_json = serde_json::to_string_pretty(&manifest)?;
            write_optional_new(&out, &out_json)?;
            println!("{out_json}");
        }
        "qualify-surrogate" => {
            let summary_path = PathBuf::from(
                match parse_arg(&args, "--summary") { Some(value) => value, None => "runs/rust-cuda-evolution-v1/campaign-summary.json".into() },
            );
            let out_path = fresh_required_output(&args, "--out", "surrogate-cv-report", ".json")?;
            let raw = std::fs::read_to_string(&summary_path)?;
            let campaign: evolve::CampaignSummary = serde_json::from_str(&raw)?;
            let mut candidates = vec![campaign.baseline_reference.clone()];
            candidates.extend(campaign.top_pareto_elites.clone());
            let report = surrogate::run_surrogate_and_engine_qualification(&candidates)?;
            let out_json = serde_json::to_string_pretty(&report)?;
            write_new(&out_path, &out_json)?;
            println!("{out_json}");
        }
        "qualify-population" => {
            let out_path =
                fresh_required_output(&args, "--out", "p05-population-checks", ".json")?;
            let oil04 =
                population::verify_added_mass_and_pressure_gradient(30.0, 50.0, 2.0e-7, 0.002)?;
            let oil05_06 = population::dispersion_partition_arithmetic(
                &[0.12, 0.28, 0.54, 0.88, 1.32],
                0.004,
                1.45,
                0.18,
                0.18,
            )?;
            let oil06_neg = population::dispersion_partition_arithmetic(
                &[0.12, 0.28, 0.54, 0.88, 1.32],
                0.004,
                1.45,
                0.18,
                1.63,
            )
            .is_err();
            let oil07 = population::verify_parcel_substep_halving(3.0, 5.0e-5)?;
            let oil08_weights = population::build_stratified_dsd_weights(25.0, 0.55, false)?;
            let oil08_neg = population::build_stratified_dsd_weights(25.0, 0.55, true).is_err();
            let mass_sum: f64 = oil08_weights.iter().map(|b| b.mass_fraction).sum();
            let num_sum: f64 = oil08_weights.iter().map(|b| b.number_fraction).sum();
            let oil10_12 = population::verify_pbm_breakup_coalescence_and_resolution(
                180.0, false, 0.003, false,
            )?;
            let oil10_neg =
                population::verify_pbm_breakup_coalescence_and_resolution(180.0, true, 0.003, false)
                    .is_err();
            let oil13_neg =
                population::verify_pbm_breakup_coalescence_and_resolution(180.0, false, 0.04, true)
                    .is_err();
            let cohort_1rank = population::generate_mpi_rank_invariant_cohort(256, 1)?;
            let cohort_4rank = population::generate_mpi_rank_invariant_cohort(256, 4)?;
            let oil14_passed = cohort_1rank == cohort_4rank;
            let oil15_valid =
                population::check_multiphase_regime_envelope(0.00059, 0.0, 0.0032, 0.024);
            let oil15_violation =
                population::check_multiphase_regime_envelope(0.035, 0.012, 0.22, 0.002);
            let well_mixed_transport = oil05_06.well_mixed_transport.clone();
            let report = serde_json::json!({
                "schema_version": "0.3.0",
                "packet": "P05",
                "evaluator": "joil-core::population",
                "kind": "software_arithmetic_oracles_not_transport_or_restart_qualification",
                "oil04_added_mass_and_pressure_gradient": oil04,
                "oil05_06_dispersion_partition_arithmetic": oil05_06,
                "well_mixed_turbulent_transport": well_mixed_transport,
                "oil07_parcel_substep_halving": oil07,
                "oil08_stratified_dsd_weights": {
                    "bins": oil08_weights,
                    "mass_fraction_sum": mass_sum,
                    "number_fraction_sum": num_sum,
                    "equal_number_mistake_rejected": oil08_neg,
                },
                "oil10_12_pbm_breakup_coalescence_and_resolution": oil10_12,
                "oil14_mpi_rank_invariance": {
                    "parcels_tested": 256,
                    "rank_1_equals_rank_4_bitwise": oil14_passed,
                },
                "oil15_multiphase_regime_envelope": {
                    "nominal_operating_point": oil15_valid,
                    "dense_core_and_gas_rejection_test": oil15_violation,
                },
                "all_p05_software_checks_passed": oil04.passed
                    && oil05_06.arithmetic_checks_passed
                    && oil06_neg
                    && oil07.passed
                    && (mass_sum - 1.0).abs() < 1e-12
                    && (num_sum - 1.0).abs() < 1e-12
                    && oil08_neg
                    && oil10_12.passed
                    && oil10_neg
                    && oil13_neg
                    && oil14_passed
                    && oil15_valid.within_dilute_lagrangian_envelope
                    && !oil15_violation.within_dilute_lagrangian_envelope,
            });
            let out_json = serde_json::to_string_pretty(&report)?;
            write_new(&out_path, &out_json)?;
            println!("{out_json}");
        }
        "qualify-experiment" => {
            let out_path =
                fresh_required_output(&args, "--out", "p06-experiment-checks", ".json")?;
            let exp01 = experiment::verify_instrument_calibration(
                "batch-ct35-oil-20261015-sha256",
                "batch-ct35-oil-20261015-sha256",
                0.9987,
                0.4,
                1.2,
                0.75,
            )?;
            let exp01_neg = experiment::verify_instrument_calibration(
                "batch-a-sha256",
                "batch-b-sha256",
                0.9987,
                0.4,
                1.2,
                15.0,
            )
            .is_err();
            let blocks = vec![
                experiment::AbaCrossoverBlock {
                    block_id: "ABA-01".into(),
                    independent_preparation: true,
                    baseline_a1_qmax_m3_h: 4.02,
                    candidate_b_qmax_m3_h: 4.65,
                    baseline_a2_qmax_m3_h: 4.00,
                    split_mismatch_abs: 0.00008,
                },
                experiment::AbaCrossoverBlock {
                    block_id: "ABA-02".into(),
                    independent_preparation: true,
                    baseline_a1_qmax_m3_h: 3.98,
                    candidate_b_qmax_m3_h: 4.58,
                    baseline_a2_qmax_m3_h: 4.01,
                    split_mismatch_abs: 0.00011,
                },
                experiment::AbaCrossoverBlock {
                    block_id: "ABA-03".into(),
                    independent_preparation: true,
                    baseline_a1_qmax_m3_h: 4.01,
                    candidate_b_qmax_m3_h: 4.62,
                    baseline_a2_qmax_m3_h: 3.99,
                    split_mismatch_abs: 0.00009,
                },
                experiment::AbaCrossoverBlock {
                    block_id: "ABA-04".into(),
                    independent_preparation: true,
                    baseline_a1_qmax_m3_h: 4.00,
                    candidate_b_qmax_m3_h: 4.60,
                    baseline_a2_qmax_m3_h: 4.02,
                    split_mismatch_abs: 0.00007,
                },
                experiment::AbaCrossoverBlock {
                    block_id: "ABA-05".into(),
                    independent_preparation: true,
                    baseline_a1_qmax_m3_h: 3.99,
                    candidate_b_qmax_m3_h: 4.64,
                    baseline_a2_qmax_m3_h: 4.01,
                    split_mismatch_abs: 0.00010,
                },
                experiment::AbaCrossoverBlock {
                    block_id: "ABA-06".into(),
                    independent_preparation: true,
                    baseline_a1_qmax_m3_h: 4.01,
                    candidate_b_qmax_m3_h: 4.61,
                    baseline_a2_qmax_m3_h: 4.00,
                    split_mismatch_abs: 0.00006,
                },
            ];
            let exp02_03_aba = experiment::analyze_blinded_aba_crossover(&blocks)?;
            let exp04_holdout = experiment::verify_holdout_isolation(
                &["train-g1".to_string(), "train-g2".to_string()],
                &["holdout-h1".to_string(), "holdout-h2".to_string()],
                1,
            )?;
            let exp04_leak_rejected = experiment::verify_holdout_isolation(
                &["train-g1".to_string(), "holdout-h1".to_string()],
                &["holdout-h1".to_string()],
                1,
            )
            .is_err();
            let exp05_ct = experiment::evaluate_as_built_ct_metrology(
                "SN-JOIL-316L-001",
                2.000,
                2.008,
                6.5,
                28.0,
                false,
            )?;
            let exp05_oversized_rejected = experiment::evaluate_as_built_ct_metrology(
                "SN-JOIL-316L-BAD",
                2.000,
                2.045,
                6.5,
                28.0,
                false,
            )
            .is_err();
            let exp06_matched = experiment::verify_shared_duty_comparator(
                "duty-ct35-4m3h-r0.01986",
                "duty-ct35-4m3h-r0.01986",
                Some("duty-ct35-4m3h-r0.01986"),
                true,
            )?;
            let exp06_unmatched_rejected = experiment::verify_shared_duty_comparator(
                "duty-ct35-4m3h-r0.01986",
                "duty-ct35-4m3h-r0.01986",
                None,
                true,
            )
            .is_err();
            let report = serde_json::json!({
                "schema_version": "0.2.0",
                "packet": "P06",
                "evaluator": "joil-core::experiment",
                "experiment01_calibration_and_loq_censoring": exp01,
                "experiment02_03_blinded_aba_crossover_and_power": exp02_03_aba,
                "experiment04_holdout_isolation": {
                    "isolated_holdout_verified": exp04_holdout,
                    "leaked_holdout_rejected": exp04_leak_rejected,
                },
                "experiment05_cad09_as_built_ct_metrology": exp05_ct,
                "experiment06_shared_duty_comparator": {
                    "matched_commercial_comparator_verified": exp06_matched,
                    "unmatched_industry_claim_rejected": exp06_unmatched_rejected,
                },
                "all_p06_gates_passed": exp01.passed
                    && exp01_neg
                    && exp02_03_aba.passed_10pct_lcb95_gate
                    && exp04_holdout
                    && exp04_leak_rejected
                    && exp05_ct.passed
                    && exp05_oversized_rejected
                    && exp06_matched
                    && exp06_unmatched_rejected,
            });
            let out_json = serde_json::to_string_pretty(&report)?;
            write_new(&out_path, &out_json)?;
            println!("{out_json}");
        }
        "export-glb-scene" => {
            let summary_path = PathBuf::from(
                match parse_arg(&args, "--summary") { Some(value) => value, None => "runs/rust-cuda-evolution-v1/campaign-summary.json".into() },
            );
            let glb_path = fresh_required_output(&args, "--glb", "glb-scene", ".glb")?;
            let out = fresh_output(&args, "--out", None, ".json")?;
            let raw = std::fs::read_to_string(&summary_path)?;
            let campaign: evolve::CampaignSummary = serde_json::from_str(&raw)?;
            let elite = campaign
                .top_pareto_elites
                .first()
                .ok_or("no top_pareto_elites in campaign summary")?;
            let surface = phenotype::MultiPatchSurface::from_program(&elite.program, 0.0)?;
            // V5-01-05: bind the scene to the STL of this exact surface, not a historical run file.
            let stl_path = glb_path.with_extension("governing.stl");
            if stl_path.exists() {
                return Err(format!("refusing to replace existing output {}", stl_path.display()).into());
            }
            surface.write_openfoam_ascii_stl(&stl_path)?;
            let stl_sha = openfoam::file_sha256(&stl_path)?;
            let scene_manifest =
                scene::export_glb_scene_bundle(&surface, &stl_sha, 1.0, &glb_path)?;
            let lease_valid = scene::verify_resource_lease(
                "lease-joil-local-4core",
                &[0, 1, 2, 3],
                4,
                16,
                8,
                1_800_000_000,
            )?;
            let lease_smt_rejected = scene::verify_resource_lease(
                "lease-smt-mistake",
                &[0, 16, 1, 17],
                4,
                16,
                8,
                1_800_000_000,
            )
            .is_err();
            let out_json = serde_json::to_string_pretty(&serde_json::json!({
                "schema_version": "0.3.0",
                "packet": "P09",
                "evaluator": "joil-core::scene",
                "ops_view01_lease_arithmetic_selftest_synthetic_cpu_lists_not_this_run": {
                    "valid_lease": lease_valid,
                    "smt_sibling_mistake_rejected": lease_smt_rejected,
                },
                "governing_surface_stl": stl_path,
                "ops_view02_03_glb_scene_manifest": scene_manifest,
            }))?;
            write_optional_new(&out, &out_json)?;
            println!("{out_json}");
        }
        "compile-3d" => {
            let out_dir = fresh_required_output(&args, "--out", "rust-3d-archetypes", "")?;
            std::fs::create_dir_all(&out_dir)?;
            let mut rng = program::Rng::new(20261015);
            let archetypes = program::DesignProgram::seed_archetypes(&mut rng);
            let mut reports = Vec::new();
            for (idx, prog) in archetypes.iter().enumerate() {
                let (pheno, mesh) = phenotype::Phenotype3D::from_program(prog)?;
                redteam::check(prog, &pheno, &mesh)
                    .map_err(|e| format!("archetype {idx} failed redteam: {e:?}"))?;
                let stl_path = out_dir.join(format!("{:02}_{}.stl", idx, pheno.topology_class));
                let obj_path = out_dir.join(format!("{:02}_{}.obj", idx, pheno.topology_class));
                mesh.write_stl(&stl_path, &pheno.topology_class, 1.0)?;
                mesh.write_obj(&obj_path, 1.0)?;
                reports.push(serde_json::json!({
                    "index": idx,
                    "topology_class": pheno.topology_class,
                    "genome_sha256": pheno.design_sha256,
                    "phenotype": pheno,
                    "stl_path": stl_path,
                    "obj_path": obj_path,
                }));
            }
            println!("{}", serde_json::to_string_pretty(&reports)?);
        }
        "evolve" => {
            let out_dir = fresh_required_output(&args, "--out", "rust-cuda-evolution", "")?;
            let mut cfg = evolve::EvolutionConfig::default();
            if let Some(g) = parse_arg(&args, "--generations").and_then(|s| s.parse().ok()) {
                cfg.generations = g;
            }
            if let Some(isl) = parse_arg(&args, "--islands").and_then(|s| s.parse().ok()) {
                cfg.islands = isl;
            }
            if let Some(pop) = parse_arg(&args, "--pop").and_then(|s| s.parse().ok()) {
                cfg.pop_per_island = pop;
            }
            if let Some(mc) = parse_arg(&args, "--mc").and_then(|s| s.parse().ok()) {
                cfg.mc_samples = mc;
            }
            if let Some(seed) = parse_arg(&args, "--seed").and_then(|s| s.parse().ok()) {
                cfg.seed = seed;
            }
            let (cuda_bin, vram_target) = explicit_gpu_opt_in(&args, utc_now_unix_seconds()?)?;
            cfg.cuda_bin_path = cuda_bin;
            cfg.gpu_vram_target_pct = vram_target;
            let summary = evolve::run_campaign(&cfg, &out_dir)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "completed",
                    "claim_state": summary.claim_state,
                    "total_evaluations": summary.total_evaluations,
                    "valid_f0_evaluations": summary.valid_f0_evaluations,
                    "red_team_rejections": summary.red_team_rejections,
                    "map_elites_occupied_cells": summary.map_elites_occupied_cells,
                    "pareto_front_size": summary.pareto_front_size,
                    "topology_family_counts_in_archive": summary.topology_family_counts_in_archive,
                    "out_dir": out_dir,
                }))?
            );
        }
        "export-3d-preview" => {
            let summary_path = PathBuf::from(
                match parse_arg(&args, "--summary") { Some(value) => value, None => "runs/rust-cuda-evolution-v1/campaign-summary.json".into() },
            );
            let archive_path = PathBuf::from(
                match parse_arg(&args, "--archive") { Some(value) => value, None => "runs/rust-cuda-evolution-v1/map-elites-archive.json".into() },
            );
            let out_path =
                fresh_required_output(&args, "--out", "interactive-3d-preview", ".json")?;
            let bundle = scene::export_interactive_3d_preview_bundle(
                &summary_path,
                &archive_path,
                &out_path,
            )?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "exported",
                    "out_path": out_path,
                    "candidates_3d_count": bundle.candidates_3d.len(),
                    "archive_points_count": bundle.archive_points.len(),
                }))?
            );
        }
        "merge-cluster" => {
            let shards_str = parse_arg(&args, "--shards").ok_or("missing --shards <dir1,dir2,...>")?;
            let shard_dirs: Vec<PathBuf> = shards_str
                .split(',')
                .filter(|s| !s.trim().is_empty())
                .map(|s| PathBuf::from(s.trim()))
                .collect();
            let nodes_str = match parse_arg(&args, "--nodes") { Some(value) => value, None => "xbabe1,xbabe2,xbabe3".into() };
            let node_labels: Vec<String> = nodes_str
                .split(',')
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.trim().to_string())
                .collect();
            let out_dir = fresh_required_output(&args, "--out", "rust-cuda-evolution-merged", "")?;
            let (summary, gates) = evolve::merge_cluster_shards(&shard_dirs, &node_labels, &out_dir)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "merged",
                    "claim_state": summary.claim_state,
                    "cluster_nodes_participating": gates.cluster_nodes_participating,
                    "total_evaluations": summary.total_evaluations,
                    "valid_f0_evaluations": summary.valid_f0_evaluations,
                    "red_team_rejections": summary.red_team_rejections,
                    "map_elites_occupied_cells": summary.map_elites_occupied_cells,
                    "pareto_front_size": summary.pareto_front_size,
                    "topology_family_counts_in_archive": summary.topology_family_counts_in_archive,
                    "engineering_gates_verification": gates,
                    "out_dir": out_dir,
                }))?
            );
        }
        "wave2-benchmark" => {
            let out_path = fresh_required_output(&args, "--out", "wave2-benchmark", ".json")?;
            let stl_dir = fresh_output(&args, "--stl-dir", None, "")?;
            let seeds: Vec<u64> = if let Some(s) = parse_arg(&args, "--seeds") {
                s.split(',')
                    .filter_map(|tok| tok.trim().parse::<u64>().ok())
                    .collect()
            } else {
                vec![
                    20261101, 20261102, 20261103, 20261104, 20261105, 20261106, 20261107,
                    20261108, 20261109, 20261110,
                ]
            };
            let cfg = evolve::EvolutionConfig {
                islands: parse_arg(&args, "--islands")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(2),
                pop_per_island: parse_arg(&args, "--pop")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(10),
                generations: parse_arg(&args, "--generations")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(8),
                mc_samples: parse_arg(&args, "--mc")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(12),
                ..evolve::EvolutionConfig::default()
            };
            let report = evolve::run_wave2_multiseed_benchmark(&seeds, &cfg)?;
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let out_json = serde_json::to_string_pretty(&report)?;
            write_new(&out_path, &out_json)?;

            if let Some(dir) = stl_dir {
                std::fs::create_dir_all(&dir)?;
                for (fam, ind) in &report.promoted_wave2_elites_by_family {
                    let (pheno, mesh) = phenotype::Phenotype3D::from_program(&ind.program)?;
                    redteam::check(&ind.program, &pheno, &mesh)
                        .map_err(|e| format!("Wave-2 family {fam} failed redteam: {e:?}"))?;
                    let stl_path = dir.join(format!("{fam}.stl"));
                    mesh.write_stl(&stl_path, fam, 1.0)?;
                }
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "completed",
                    "protocol_id": report.protocol_id,
                    "n_seeds": report.n_seeds,
                    "evaluations_per_seed": report.evaluations_per_seed,
                    "summary_statistics": report.summary_statistics,
                    "pairwise_tests": report.pairwise_tests,
                    "rejected_cohort_10pct_audit": report.rejected_cohort_10pct_audit,
                    "promoted_families_count": report.promoted_wave2_elites_by_family.len(),
                    "out_path": out_path,
                }))?
            );
        }
        "wave3-proof" => {
            let out_path = fresh_required_output(&args, "--out", "wave3-proof", ".json")?;
            let seeds: Vec<u64> = if let Some(s) = parse_arg(&args, "--seeds") {
                s.split(',')
                    .filter_map(|tok| tok.trim().parse::<u64>().ok())
                    .collect()
            } else {
                vec![
                    20261101, 20261102, 20261103, 20261104, 20261105, 20261106, 20261107,
                    20261108, 20261109, 20261110,
                ]
            };
            let cfg = evolve::EvolutionConfig {
                islands: parse_arg(&args, "--islands")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(2),
                pop_per_island: parse_arg(&args, "--pop")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(9),
                generations: parse_arg(&args, "--generations")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(24),
                mc_samples: parse_arg(&args, "--mc")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(12),
                ..evolve::EvolutionConfig::default()
            };
            let report = if let Some(raw_dir) = parse_arg(&args, "--raw-dir") {
                evolve::run_wave3_evolution_proof_with_raw_capture(&seeds, &cfg, std::path::Path::new(&raw_dir))?
            } else {
                evolve::run_wave3_evolution_proof(&seeds, &cfg)?
            };
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let pretty = serde_json::to_string_pretty(&report)?;
            let out_json = pretty.lines().map(str::trim_start).collect::<Vec<_>>().join("\n") + "\n";
            write_new(&out_path, &out_json)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "completed",
                    "protocol_id": report.protocol_id,
                    "n_seeds": report.n_seeds,
                    "generations": report.generations,
                    "evaluations_per_seed_per_arm": report.evaluations_per_seed_per_arm,
                    "total_3d_evaluations_across_arms": report.total_3d_evaluations_across_arms,
                    "baseline_b0_ct35_kpi_score": report.baseline_b0_ct35_kpi_score,
                    "baseline_b0_ct35_cout_p95_mg_l": report.baseline_b0_ct35_cout_p95_mg_l,
                    "promoted_wave3_top_elite_id": report.promoted_wave3_top_elite.id,
                    "promoted_wave3_top_elite_topology": report.promoted_wave3_top_elite.topology_class,
                    "promoted_wave3_top_elite_kpi_score": report.promoted_wave3_top_elite_kpi_score,
                    "promoted_wave3_top_elite_cout_p95_mg_l": report.promoted_wave3_top_elite.mc_robustness.cout_p95_mg_l,
                    "final_summary_statistics": report.final_summary_statistics,
                    "pairwise_tests": report.pairwise_tests,
                    "out_path": out_path,
                }))?
            );
        }
        _ => {
            return Err(
                "supported commands: baseline, kpi, verify-rotation, screen-structure (legacy alias qualify-structure), audit-probe, promote-elite, prepare-cfd-case, qualify-surrogate, qualify-population, qualify-experiment, export-glb-scene, export-3d-preview, compile-3d, evolve, merge-cluster, wave2-benchmark, wave3-proof".into(),
            )
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMITTED_WAVE2: &str =
        include_str!("../../../docs/results/wave2-multiseed-ablation-and-holdout-v1.json");

    fn argv(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    /// SYNTHETIC files in a fresh temp directory; nothing is executed.
    fn gpu_fixture(name: &str, lease: Option<&str>) -> (PathBuf, String, String) {
        let dir = std::env::temp_dir().join(format!("joil-v5-01-04-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let bin = dir.join("joil_cuda_eval");
        std::fs::write(&bin, b"SYNTHETIC placeholder, never executed").unwrap();
        let lease_path = dir.join("lease.json");
        if let Some(text) = lease {
            std::fs::write(&lease_path, text).unwrap();
        }
        (
            dir,
            bin.display().to_string(),
            lease_path.display().to_string(),
        )
    }

    /// 2026-10-17T22:00:00Z, independently computed with Python datetime.
    const NOW_2026_10_17T22: i64 = 1_792_274_400;

    #[test]
    fn v5_01_04_ordinary_runs_request_no_gpu() {
        let now = NOW_2026_10_17T22;
        assert_eq!(
            explicit_gpu_opt_in(&argv(&["evolve", "--out", "x"]), now).unwrap(),
            (None, None)
        );
        // A present default binary path no longer activates anything implicitly.
        let (dir, _bin, lease) = gpu_fixture("ordinary", Some(r#"{"gpu": true}"#));
        assert!(explicit_gpu_opt_in(&argv(&["evolve", "--gpu-lease", &lease]), now).is_err());
        assert!(
            explicit_gpu_opt_in(&argv(&["evolve", "--gpu-vram-target-pct", "85.5"]), now).is_err()
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn v5_01_04_missing_or_invalid_gpu_lease_cannot_execute() {
        let now = NOW_2026_10_17T22;
        let good = r#"{"gpu": true, "expires_utc": "2026-10-18T08:52:43+00:00"}"#;
        let cases = [
            ("no-lease-file", None, "cannot read GPU lease"),
            (
                "gpu-false",
                Some(r#"{"gpu": false, "expires_utc": "2026-10-18T08:52:43Z"}"#),
                "does not grant",
            ),
            (
                "gpu-string",
                Some(r#"{"gpu": "true", "expires_utc": "2026-10-18T08:52:43Z"}"#),
                "does not grant",
            ),
            ("no-expiry", Some(r#"{"gpu": true}"#), "lacks expires_utc"),
            (
                "expired",
                Some(r#"{"gpu": true, "expires_utc": "2026-10-17T21:59:59Z"}"#),
                "expired",
            ),
            ("not-json", Some("gpu: true"), "invalid GPU lease"),
        ];
        for (name, lease, needle) in cases {
            let (dir, bin, lease_path) = gpu_fixture(name, lease);
            let err = explicit_gpu_opt_in(
                &argv(&["evolve", "--cuda-bin", &bin, "--gpu-lease", &lease_path]),
                now,
            )
            .unwrap_err();
            assert!(err.contains(needle), "{name}: {err}");
            std::fs::remove_dir_all(dir).unwrap();
        }
        let (dir, bin, lease_path) = gpu_fixture("no-lease-arg", Some(good));
        let err = explicit_gpu_opt_in(&argv(&["evolve", "--cuda-bin", &bin]), now).unwrap_err();
        assert!(err.contains("requires --gpu-lease"), "{err}");
        let missing_bin = format!("{}.missing", bin);
        assert!(explicit_gpu_opt_in(
            &argv(&[
                "evolve",
                "--cuda-bin",
                &missing_bin,
                "--gpu-lease",
                &lease_path
            ]),
            now
        )
        .unwrap_err()
        .contains("not a file"));
        for bad in ["0", "-5", "100.5", "NaN", "inf", "x"] {
            assert!(explicit_gpu_opt_in(
                &argv(&[
                    "evolve",
                    "--cuda-bin",
                    &bin,
                    "--gpu-lease",
                    &lease_path,
                    "--gpu-vram-target-pct",
                    bad,
                ]),
                now
            )
            .is_err());
        }
        // Positive control: explicit binary, current lease and explicit VRAM target.
        let (got_bin, vram) = explicit_gpu_opt_in(
            &argv(&[
                "evolve",
                "--cuda-bin",
                &bin,
                "--gpu-lease",
                &lease_path,
                "--gpu-vram-target-pct",
                "50",
            ]),
            now,
        )
        .unwrap();
        assert_eq!(got_bin, Some(PathBuf::from(&bin)));
        assert_eq!(vram, Some(50.0));
        assert_eq!(
            explicit_gpu_opt_in(
                &argv(&["evolve", "--cuda-bin", &bin, "--gpu-lease", &lease_path]),
                now
            )
            .unwrap()
            .1,
            None
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn v5_01_05_existing_outputs_and_committed_receipts_are_refused() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        for rel in [
            "docs/results/wave2-multiseed-ablation-and-holdout-v1.json",
            "docs/results/wave3-evolution-proof-v1.json",
            "docs/results/interactive-3d-archetypes-v1.json",
        ] {
            let committed = root.join(rel);
            let before = std::fs::read(&committed).unwrap();
            let arg = committed.display().to_string();
            let err = fresh_output(&argv(&["x", "--out", &arg]), "--out", Some("x"), ".json")
                .unwrap_err();
            assert!(err.contains("refusing to replace existing output"), "{err}");
            assert!(write_new(&committed, "SYNTHETIC").is_err());
            assert_eq!(std::fs::read(&committed).unwrap(), before);
        }
        let (dir, _bin, _lease) = gpu_fixture("sentinel", None);
        let sentinel = dir.join("sentinel.json");
        std::fs::write(&sentinel, b"keep").unwrap();
        let arg = sentinel.display().to_string();
        assert!(fresh_output(&argv(&["x", "--glb", &arg]), "--glb", None, ".glb").is_err());
        assert!(write_optional_new(&Some(sentinel.clone()), "SYNTHETIC").is_err());
        assert_eq!(std::fs::read(&sentinel).unwrap(), b"keep");
        // A fresh explicit path is created exactly once.
        let fresh = dir.join("nested/fresh.json");
        let got = fresh_output(
            &argv(&["x", "--out", &fresh.display().to_string()]),
            "--out",
            None,
            ".json",
        )
        .unwrap()
        .unwrap();
        write_new(&got, "{}").unwrap();
        assert!(write_new(&got, "{}").is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn v5_01_05_defaults_are_unique_ignored_run_paths() {
        let no_out = argv(&["wave3-proof"]);
        let file = fresh_required_output(&no_out, "--out", "wave3-proof", ".json").unwrap();
        let text = file.display().to_string();
        assert!(
            text.starts_with("runs/wave3-proof-") && text.ends_with(".json"),
            "{text}"
        );
        assert!(
            text.contains(&format!("-{}.json", std::process::id())),
            "{text}"
        );
        let dir = fresh_required_output(&no_out, "--case", "cfd-case", "").unwrap();
        assert!(dir.display().to_string().starts_with("runs/cfd-case-"));
        assert_eq!(fresh_output(&no_out, "--out", None, ".json").unwrap(), None);
    }

    #[test]
    fn v5_01_04_utc_clock_format() {
        let now = utc_now_seconds().unwrap();
        assert_eq!(now.len(), 19);
        assert_eq!(&now[10..11], "T");
        assert!(now.as_str() > "2026-01-01T00:00:00");
        // The clock formatter and the expiry parser are inverses (within a second boundary).
        let parsed = parse_utc_expiry_seconds(&format!("{now}Z")).unwrap();
        assert!((utc_now_unix_seconds().unwrap() - parsed).abs() <= 2);
    }

    #[test]
    fn v5_01_04_expiry_parser_matches_independent_epoch_oracles() {
        // Expected values computed independently with Python datetime (UTC).
        for (text, want) in [
            ("1970-01-01T00:00:00Z", 0_i64),
            ("2000-02-29T12:00:00Z", 951_825_600),
            ("2026-10-17T22:00:00+00:00", NOW_2026_10_17T22),
            ("2026-10-18T08:52:43Z", 1_792_313_563),
            ("2026-10-18T08:52:43.818175+00:00", 1_792_313_563),
            ("1900-03-01T00:00:00Z", -2_203_891_200),
            ("9999-12-31T23:59:59Z", 253_402_300_799),
            ("0001-01-01T00:00:00Z", -62_135_596_800),
        ] {
            assert_eq!(parse_utc_expiry_seconds(text), Ok(want), "{text}");
        }
    }

    /// Review 2 controls through the production `explicit_gpu_opt_in` path; nothing is executed.
    #[test]
    fn v5_01_04_malformed_or_non_utc_expiry_never_authorizes_gpu() {
        let now = NOW_2026_10_17T22;
        let run = |name: &str, expires: &str| {
            let lease = serde_json::json!({"gpu": true, "expires_utc": expires}).to_string();
            let (dir, bin, lease_path) = gpu_fixture(name, Some(&lease));
            let result = explicit_gpu_opt_in(
                &argv(&["evolve", "--cuda-bin", &bin, "--gpu-lease", &lease_path]),
                now,
            );
            std::fs::remove_dir_all(dir).unwrap();
            result
        };
        // Positive: valid future UTC expiry, with Z, +00:00 and a fraction.
        for (i, ok) in [
            "2026-10-18T08:52:43Z",
            "2026-10-18T08:52:43+00:00",
            "2026-10-18T08:52:43.818175+00:00",
        ]
        .into_iter()
        .enumerate()
        {
            let (bin, vram) = run(&format!("valid-{i}"), ok).unwrap();
            assert!(bin.is_some(), "{ok}");
            assert_eq!(vram, None);
        }
        let cases = [
            (
                "feb-30",
                "2026-02-30T00:00:00Z",
                "invalid calendar date 2026-02-30",
            ),
            (
                "non-leap-feb-29",
                "2027-02-29T00:00:00Z",
                "invalid calendar date 2027-02-29",
            ),
            (
                "century-feb-29",
                "2100-02-29T00:00:00Z",
                "invalid calendar date 2100-02-29",
            ),
            (
                "all-nines",
                "9999-99-99T99:99:99Z",
                "invalid calendar date 9999-99-99",
            ),
            (
                "all-nines-no-zone",
                "9999-99-99T99:99:99",
                "missing time zone designator",
            ),
            (
                "month-13",
                "2026-13-01T00:00:00Z",
                "invalid calendar date 2026-13-01",
            ),
            (
                "year-zero",
                "0000-01-01T00:00:00Z",
                "invalid calendar date 0000-01-01",
            ),
            (
                "hour-24",
                "2026-10-18T24:00:00Z",
                "invalid time of day 24:00:00",
            ),
            (
                "minute-60",
                "2026-10-18T08:60:00Z",
                "invalid time of day 08:60:00",
            ),
            (
                "leap-second",
                "2026-10-18T08:52:60Z",
                "invalid time of day 08:52:60",
            ),
            ("plus-two", "2026-10-18T08:52:43+02:00", "offset is not UTC"),
            (
                "minus-five",
                "2026-10-18T08:52:43-05:00",
                "offset is not UTC",
            ),
            (
                "minus-zero",
                "2026-10-18T08:52:43-00:00",
                "offset is not UTC",
            ),
            (
                "no-zone",
                "2026-10-18T08:52:43",
                "missing time zone designator",
            ),
            (
                "trailing",
                "2026-10-18T08:52:43Zjunk",
                "malformed timestamp syntax",
            ),
            (
                "lower-z",
                "2026-10-18T08:52:43z",
                "malformed timestamp syntax",
            ),
            (
                "space-sep",
                "2026-10-18 08:52:43Z",
                "malformed timestamp syntax",
            ),
            (
                "five-digit-year",
                "20261-10-18T08:52:43Z",
                "malformed timestamp syntax",
            ),
            (
                "long-fraction",
                "2026-10-18T08:52:43.1234567890Z",
                "malformed timestamp syntax",
            ),
            (
                "empty-fraction",
                "2026-10-18T08:52:43.Z",
                "malformed timestamp syntax",
            ),
            (
                "signed-field",
                "2026-10-+8T08:52:43Z",
                "malformed timestamp syntax",
            ),
            ("short", "2026-10-18", "malformed timestamp syntax"),
            // Byte 18-19 is a two-byte character: the old slice at [..19] panicked here.
            ("utf8-split", "2026-10-18T08:52:4\u{e9}Z", "not ASCII"),
            ("utf8-fullwidth", "2026-10-18T08:52:4\u{ff13}Z", "not ASCII"),
            ("past", "2026-10-17T21:59:59Z", "expired at"),
            ("equal-now", "2026-10-17T22:00:00Z", "expired at"),
        ];
        for (name, expires, needle) in cases {
            let err = run(name, expires).unwrap_err();
            assert!(err.contains(needle), "{name}: {err}");
            if needle != "expired at" {
                assert!(err.contains("invalid expires_utc"), "{name}: {err}");
            }
        }
        // A non-string expiry is still refused.
        let (dir, bin, lease_path) = gpu_fixture(
            "numeric-expiry",
            Some(r#"{"gpu": true, "expires_utc": 1792313563}"#),
        );
        let err = explicit_gpu_opt_in(
            &argv(&["evolve", "--cuda-bin", &bin, "--gpu-lease", &lease_path]),
            now,
        )
        .unwrap_err();
        assert!(err.contains("lacks expires_utc"), "{err}");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn historical_wave2_receipt_loads_all_promoted_genomes() {
        let report: serde_json::Value = serde_json::from_str(COMMITTED_WAVE2).unwrap();
        let elites = report["promoted_wave2_elites_by_family"]
            .as_object()
            .unwrap();
        assert_eq!(elites.len(), 11);
        for (family, elite) in elites {
            let loaded = load_wave2_elite_program(COMMITTED_WAVE2, family).unwrap();
            let archived: program::DesignProgram =
                serde_json::from_value(elite["program"].clone()).unwrap();
            assert_eq!(loaded, archived.canonicalize().unwrap(), "{family}");
            assert_eq!(loaded.schema_version, 2);
            assert_eq!(loaded.family, "ct35-exotic-head-v2");
            let current_hash = loaded.canonical_sha256().unwrap();
            assert_eq!(current_hash.len(), 64);
            // Authored topology labels no longer belong to the serialized
            // subject. The archived hash cannot identify the prepared case.
            assert_ne!(Some(current_hash.as_str()), elite["genome_sha256"].as_str());
        }
    }

    #[test]
    fn historical_wave2_import_does_not_require_or_trust_evaluations() {
        let mut report: serde_json::Value = serde_json::from_str(COMMITTED_WAVE2).unwrap();
        let family = "trifurcated-6jet-vortex-head";
        let elite = &mut report["promoted_wave2_elites_by_family"][family];
        let genome = elite["program"].clone();
        // A genome-only archive is sufficient. No current Individual, screen,
        // admission or objective evidence is synthesized for missing fields.
        *elite = serde_json::json!({"program": genome});
        let raw = serde_json::to_string(&report).unwrap();
        assert_eq!(
            load_wave2_elite_program(&raw, family).unwrap(),
            load_wave2_elite_program(COMMITTED_WAVE2, family).unwrap()
        );
    }

    #[test]
    fn historical_wave2_import_rejects_missing_or_invalid_genomes() {
        assert!(load_wave2_elite_program(COMMITTED_WAVE2, "missing-family").is_err());
        for raw in [
            "not json",
            "{}",
            r#"{"promoted_wave2_elites_by_family":[]}"#,
        ] {
            assert!(load_wave2_elite_program(raw, "baseline-ct35").is_err());
        }
        let mut report: serde_json::Value = serde_json::from_str(COMMITTED_WAVE2).unwrap();
        for genome in [
            serde_json::Value::Null,
            serde_json::json!({"schema_version": 2}),
            {
                let mut genome =
                    report["promoted_wave2_elites_by_family"]["baseline-ct35"]["program"].clone();
                genome["inlet_split"]["branches_per_port"] = serde_json::json!(0);
                genome
            },
        ] {
            report["promoted_wave2_elites_by_family"]["baseline-ct35"]["program"] = genome;
            let raw = serde_json::to_string(&report).unwrap();
            assert!(load_wave2_elite_program(&raw, "baseline-ct35").is_err());
        }
        report["promoted_wave2_elites_by_family"]["baseline-ct35"] = serde_json::json!({});
        assert!(load_wave2_elite_program(
            &serde_json::to_string(&report).unwrap(),
            "baseline-ct35"
        )
        .is_err());
    }
}
