//! Interactive 3D preview bundle construction and unit tests for `scene`.

use super::*;

fn round2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

fn build_3d_candidate_preview(
    ind: &crate::evolve::Individual,
) -> Result<Interactive3DCandidate, String> {
    use crate::openfoam::STANDARD_BINS_UM;
    use crate::phenotype::Phenotype3D;
    use std::f64::consts::PI;

    let prog = &ind.program;
    let d = crate::geometry::baseline().derive()?;
    let (pheno, _) = Phenotype3D::from_program(prog)?;
    let sq = crate::structure::analytic_structural_screen(
        prog,
        &pheno,
        ind.nominal_metrics.dp_u_pa,
        0.001111111,
    )?;

    // 1. Outer wall 3D rings (22 axial stations in z in [0, 186.75] mm x 28 azimuthal points)
    let n_z = 22usize;
    let n_theta = 28usize;
    let mut outer_wall_rings_mm = Vec::with_capacity(n_z);
    for iz in 0..n_z {
        let z_mm = 186.747 * (iz as f64) / ((n_z - 1) as f64);
        let mut ring = Vec::with_capacity(n_theta);
        for it in 0..n_theta {
            let theta = 2.0 * PI * (it as f64) / (n_theta as f64);
            let r = Phenotype3D::inner_radius_mm(prog, &d, theta, z_mm);
            ring.push([
                round2(r * theta.cos()),
                round2(r * theta.sin()),
                round2(z_mm),
            ]);
        }
        outer_wall_rings_mm.push(ring);
    }

    // 2. Overflow vortex finder & contoured/fluted skimmer rings (z in [-22.8, skimmer.length_mm] mm)
    let skim_end = prog.skimmer.length_mm.max(0.5);
    let n_sz = 7usize;
    let n_st = 20usize;
    let mut skimmer_rings_mm = Vec::with_capacity(n_sz);
    for iz in 0..n_sz {
        let s = (iz as f64) / ((n_sz - 1) as f64);
        let z_mm = -22.8 + (22.8 + skim_end) * s;
        let base_r = if z_mm < 0.0 {
            1.4
        } else {
            prog.skimmer.outer_radius_mm - 0.35 * (z_mm / skim_end)
        };
        let mut ring = Vec::with_capacity(n_st);
        for it in 0..n_st {
            let theta = 2.0 * PI * (it as f64) / (n_st as f64);
            let flute_mod = if z_mm >= 0.0 && prog.skimmer.capture_flutes > 0 {
                0.18 * ((prog.skimmer.capture_flutes as f64) * theta).cos()
            } else {
                0.0
            };
            let r = base_r + flute_mod;
            ring.push([
                round2(r * theta.cos()),
                round2(r * theta.sin()),
                round2(z_mm),
            ]);
        }
        skimmer_rings_mm.push(ring);
    }

    // 3. Multi-branch 3D tangential entry jet centerlines
    let branches = if prog.inlet_split.enabled {
        prog.inlet_split.branches_per_port.max(2)
    } else {
        1
    };
    let mut inlet_jets_3d_mm = Vec::new();
    for &port_sign in &[-1.0_f64, 1.0_f64] {
        for b in 0..branches {
            let b_frac = if branches == 1 {
                0.0
            } else {
                ((b as f64) / ((branches - 1) as f64)) - 0.5
            };
            let mut jet = Vec::with_capacity(16);
            for step in 0..16 {
                let t = (step as f64) / 15.0;
                if t <= 0.45 {
                    let u = t / 0.45;
                    let x = port_sign * (60.0 - 52.0 * u);
                    let y = -port_sign
                        * (31.28 * prog.inlet_ramp.radial_slot_factor + 1.8 * b_frac * u);
                    let z = 6.45 + prog.inlet_ramp.axial_drop_mm * u + 3.2 * b_frac * u;
                    jet.push([round2(x), round2(y), round2(z)]);
                } else {
                    let u = (t - 0.45) / 0.55;
                    let base_theta = if port_sign < 0.0 { -0.5 * PI } else { 0.5 * PI }
                        + (prog.inlet_split.azimuthal_phase_deg.to_radians() * b_frac);
                    let theta = base_theta + 1.65 * PI * u;
                    let r = 31.0 * (1.0 - 0.12 * u);
                    let z = 6.45
                        + prog.inlet_ramp.axial_drop_mm
                        + (18.0 + 1.8 * prog.inlet_ramp.axial_drop_mm) * u
                        + 3.2 * b_frac;
                    jet.push([round2(r * theta.cos()), round2(r * theta.sin()), round2(z)]);
                }
            }
            inlet_jets_3d_mm.push(jet);
        }
    }

    // 4. Internal helical swirl ribbon / wall rifling guide curve
    let mut helical_ribbon_3d_mm = Vec::with_capacity(40);
    let pitch_turns =
        1.6 + 0.15 * prog.inlet_ramp.axial_drop_mm + 0.25 * (prog.chamber_modes.len() as f64);
    for i in 0..40 {
        let s = (i as f64) / 39.0;
        let z_mm = 6.0 + 96.0 * s;
        let theta = 2.0 * PI * pitch_turns * s;
        let r = (Phenotype3D::inner_radius_mm(prog, &d, theta, z_mm) - 1.6).max(12.0);
        helical_ribbon_3d_mm.push([
            round2(r * theta.cos()),
            round2(r * theta.sin()),
            round2(z_mm),
        ]);
    }

    // 5. SCHEMATIC authored droplet and carrier paths (10, 20, 40 um oil + carrier water); no particle
    // is integrated (`V5-01-04`).
    let swirl_boost = ind.descriptors.swirl_flux_index;
    let mut droplet_trajectories_3d_mm = Vec::new();
    for &(label, d_um, is_water) in &[
        ("SCHEMATIC 40 μm oil droplet path (authored)", 40.0, false),
        ("SCHEMATIC 20 μm oil droplet path (authored)", 20.0, false),
        ("SCHEMATIC 10 μm oil droplet path (authored)", 10.0, false),
        (
            "SCHEMATIC treated-water outer vortex path (authored)",
            0.0,
            true,
        ),
    ] {
        let mut pts = Vec::with_capacity(48);
        for k in 0..48 {
            let s = (k as f64) / 47.0;
            let turns = (3.8 + 0.9 * swirl_boost) * s;
            let theta = 2.0 * PI * turns + (d_um * 0.04);
            let (r_mm, z_mm) = if is_water {
                let z = 6.0 + 178.0 * s;
                let r = (Phenotype3D::inner_radius_mm(prog, &d, theta, z) * 0.82).max(9.5);
                (r, z)
            } else {
                // Radial inward migration speed scales with d^2 * swirl_boost^2
                let mig_rate = (d_um / 22.0).powi(2) * swirl_boost * 1.15;
                let r_start = 29.5;
                let r_core = 0.85;
                let r = r_core + (r_start - r_core) * (-mig_rate * s * 2.2).exp();
                // Once r approaches the reverse-flowing inner vortex core (r < 6.5 mm), axial velocity reverses toward z = -22.8 mm!
                let z = if r > 6.5 {
                    6.0 + 115.0 * s
                } else {
                    let core_frac = ((6.5 - r) / (6.5 - r_core)).clamp(0.0, 1.0);
                    (6.0 + 75.0 * (1.0 - core_frac)) * (1.0 - core_frac) - 22.0 * core_frac
                };
                (r, z)
            };
            pts.push([
                round2(r_mm * theta.cos()),
                round2(r_mm * theta.sin()),
                round2(z_mm),
            ]);
        }
        droplet_trajectories_3d_mm.push(DropletTrajectory3D {
            label: label.to_string(),
            diameter_um: d_um,
            captured_to_overflow: !is_water && d_um >= ind.nominal_metrics.d50_um,
            points_mm: pts,
            provenance: schematic_trajectory_provenance(),
        });
    }

    let grade_curve_bins: Vec<(f64, f64)> = STANDARD_BINS_UM
        .iter()
        .map(|&d_bin| {
            let key = format!("{}um", d_bin as u32);
            let g = *ind
                .nominal_metrics
                .grade_efficiency_bins
                .get(&key)
                .unwrap_or(&0.0);
            (d_bin, (g * 10000.0).round() / 10000.0)
        })
        .collect();

    Ok(Interactive3DCandidate {
        id: ind.id.clone(),
        topology_class: ind.topology_class.clone(),
        genome_sha256: ind.genome_sha256.clone(),
        generation_born: ind.generation_born,
        island_origin: ind.island_origin,
        branches_per_port: branches,
        axial_drop_mm: prog.inlet_ramp.axial_drop_mm,
        radial_slot_factor: prog.inlet_ramp.radial_slot_factor,
        skimmer_length_mm: prog.skimmer.length_mm,
        skimmer_flutes: prog.skimmer.capture_flutes,
        chamber_mode_count: prog.chamber_modes.len(),
        nominal_cout_mg_l: ind.nominal_metrics.cout_mg_l,
        cout_p05_mg_l: ind.mc_robustness.cout_p05_mg_l,
        cout_p50_mg_l: ind.mc_robustness.cout_p50_mg_l,
        cout_p95_mg_l: ind.mc_robustness.cout_p95_mg_l,
        dp_u_pa: ind.nominal_metrics.dp_u_pa,
        pdr: ind.nominal_metrics.pdr,
        d50_um: ind.nominal_metrics.d50_um,
        grade_20um_p05: ind.mc_robustness.grade_20um_p05,
        max_entry_shear_s_inv: ind.nominal_metrics.max_entry_shear_s_inv,
        short_circuit_fraction: ind.nominal_metrics.short_circuit_fraction,
        p08_yield_safety_factor: sq.min_yield_safety_factor,
        p08_max_von_mises_mpa: sq.max_wall_von_mises_mpa,
        p08_vane_modal_freq_ratio: sq.vane_modal_freq_ratio,
        p08_lpbf_overhang_deg: sq.max_lpbf_overhang_from_vertical_deg,
        min_wall_clearance_mm: pheno.minimum_wall_clearance_mm,
        grade_curve_bins,
        outer_wall_rings_mm,
        skimmer_rings_mm,
        inlet_jets_3d_mm,
        helical_ribbon_3d_mm,
        droplet_trajectories_3d_mm,
    })
}

/// Export an interactive 3D preview & MAP-Elites/Pareto archive bundle (`docs/results/interactive-3d-archetypes-v1.json`).
pub fn export_interactive_3d_preview_bundle(
    campaign_summary_path: &Path,
    archive_path: &Path,
    out_path: &Path,
) -> Result<Interactive3DPreviewBundle, String> {
    if !campaign_summary_path.is_file() && out_path.is_file() {
        let raw_existing = fs::read_to_string(out_path).map_err(|e| e.to_string())?;
        let existing: Interactive3DPreviewBundle =
            serde_json::from_str(&raw_existing).map_err(|e| e.to_string())?;
        if existing.candidates_3d.is_empty() {
            return Err("existing interactive 3D preview bundle has empty candidates_3d".into());
        }
        return Ok(existing);
    }
    let raw_sum = fs::read_to_string(campaign_summary_path).map_err(|e| e.to_string())?;
    let campaign: crate::evolve::CampaignSummary =
        serde_json::from_str(&raw_sum).map_err(|e| e.to_string())?;
    let sum_sha = crate::openfoam::file_sha256(campaign_summary_path)?;

    let mut candidates_3d = Vec::new();
    candidates_3d.push(build_3d_candidate_preview(&campaign.baseline_reference)?);
    for elite in &campaign.top_pareto_elites {
        if !candidates_3d
            .iter()
            .any(|c| c.genome_sha256 == elite.genome_sha256)
        {
            candidates_3d.push(build_3d_candidate_preview(elite)?);
        }
    }
    let mut wave2_rng = crate::program::Rng::new(20261101);
    let wave2_seeds = crate::program::DesignProgram::seed_wave2_archetypes(&mut wave2_rng);
    let mut seen_w2_fam = std::collections::BTreeSet::new();
    for (idx, w2_prog) in wave2_seeds.into_iter().skip(15).enumerate() {
        let fam = w2_prog.classify_topology();
        if !seen_w2_fam.insert(fam) {
            continue;
        }
        let (ind, _) = crate::evolve::evaluate_individual(
            format!("wave2-exotic-{idx:02}-{}", w2_prog.topology_class),
            w2_prog,
            0,
            0,
            &campaign.config,
        )?;
        if !candidates_3d
            .iter()
            .any(|c| c.genome_sha256 == ind.genome_sha256)
        {
            candidates_3d.push(build_3d_candidate_preview(&ind)?);
        }
    }

    let mut archive_points = Vec::new();
    if archive_path.is_file() {
        let raw_arch = fs::read_to_string(archive_path).map_err(|e| e.to_string())?;
        let map: std::collections::BTreeMap<String, crate::evolve::Individual> =
            serde_json::from_str(&raw_arch).map_err(|e| e.to_string())?;
        for (cell_key, ind) in map {
            let is_pareto = campaign
                .top_pareto_elites
                .iter()
                .any(|e| e.genome_sha256 == ind.genome_sha256);
            archive_points.push(ArchiveExplorerPoint {
                id: ind.id,
                topology_class: ind.topology_class,
                cell_key,
                cout_p95_mg_l: round2(ind.mc_robustness.cout_p95_mg_l),
                dp_u_pa: round2(ind.nominal_metrics.dp_u_pa),
                d50_um: round2(ind.nominal_metrics.d50_um),
                grade_20um_p05: (ind.mc_robustness.grade_20um_p05 * 10000.0).round() / 10000.0,
                max_entry_shear_s_inv: round2(ind.nominal_metrics.max_entry_shear_s_inv),
                swirl_flux_index: (ind.descriptors.swirl_flux_index * 1000.0).round() / 1000.0,
                core_shield_ratio: (ind.descriptors.core_shield_ratio * 1000.0).round() / 1000.0,
                is_pareto_rank0: is_pareto,
            });
        }
    }

    let bundle = Interactive3DPreviewBundle {
        schema_version: 1,
        kind: "interactive_3d_hydrocyclone_preview_and_archive_bundle".to_string(),
        units: "mm".to_string(),
        true_scale: 1.0,
        campaign_summary_sha256: sum_sha,
        candidates_3d,
        archive_points,
    };
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json_str = serde_json::to_string(&bundle).map_err(|e| e.to_string())?;
    fs::write(out_path, format!("{json_str}\n")).map_err(|e| e.to_string())?;
    Ok(bundle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v5_01_04_new_trajectories_are_schematic() {
        let cfg = crate::evolve::EvolutionConfig {
            mc_samples: 4,
            ..crate::evolve::EvolutionConfig::default()
        };
        let (ind, _) = crate::evolve::evaluate_individual(
            "base".into(),
            crate::program::DesignProgram::baseline_ct35(),
            0,
            0,
            &cfg,
        )
        .unwrap();
        let preview = build_3d_candidate_preview(&ind).unwrap();
        assert_eq!(preview.droplet_trajectories_3d_mm.len(), 4);
        for traj in &preview.droplet_trajectories_3d_mm {
            assert_eq!(traj.provenance, SCHEMATIC_TRAJECTORY_PROVENANCE);
            assert!(traj.label.starts_with("SCHEMATIC "), "{}", traj.label);
        }
    }

    #[test]
    fn v5_01_04_historical_bundle_reads_as_schematic_not_measured() {
        // Read-only path: a missing campaign summary returns the existing bundle without writing.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let bundle_path = root.join("docs/results/interactive-3d-archetypes-v1.json");
        let before = fs::read(&bundle_path).unwrap();
        let bundle = export_interactive_3d_preview_bundle(
            &root.join("runs/v5-01-04-missing-campaign-summary.json"),
            &root.join("runs/v5-01-04-missing-archive.json"),
            &bundle_path,
        )
        .unwrap();
        assert!(!before.windows(10).any(|w| w == b"provenance"));
        let mut count = 0;
        for cand in &bundle.candidates_3d {
            for traj in &cand.droplet_trajectories_3d_mm {
                assert_eq!(traj.provenance, SCHEMATIC_TRAJECTORY_PROVENANCE);
                count += 1;
            }
        }
        assert!(count > 0);
        assert_eq!(fs::read(&bundle_path).unwrap(), before);
    }
}
