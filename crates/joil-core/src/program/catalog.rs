//! Topological exoticness index, closed-catalog validity audit, operator-chain audit, and Wave-3 de-novo initialization.

use super::*;

#[path = "wave3_de_novo.rs"]
mod wave3_de_novo;

impl DesignProgram {
    /// Compute a dimensionless 3D topological exoticness score measuring non-axisymmetric branching,
    /// TPMS lattice grading, helical guide-ribbon twist, harmonic chamber rifling, and S-cone inflection.
    pub fn exoticness_index(&self) -> Result<f64, String> {
        let p = self.normalize()?;
        let desc = p.descriptors()?;
        let branch_score = if p.inlet_split.enabled {
            (p.inlet_split.branches_per_port as f64 - 1.0) * 0.28
        } else {
            0.0
        };
        let tpms_score = match p.tpms_insert.basis {
            TpmsBasis::None => 0.0,
            TpmsBasis::CylindricalGyroid => 0.85,
            TpmsBasis::Gyroid => 0.78,
            TpmsBasis::SchwarzP => 0.74,
            TpmsBasis::Diamond => 0.72,
        };
        let ribbon_score = if !p.ribbon_cascade.is_inactive() {
            0.42 * (p.ribbon_cascade.vane_count as f64 / 4.0)
                + 0.18 * (p.ribbon_cascade.helical_twist_rad.abs() / 3.5)
        } else {
            0.0
        };
        let mode_score = 0.25 * desc.wall_rifling_stabilization
            + 0.20 * (1.0 - desc.shear_mitigation_factor).max(0.0);
        let skim_score = 0.12 * (p.skimmer.capture_flutes as f64 / 8.0);
        Ok(round6(
            branch_score + tpms_score + ribbon_score + mode_score + skim_score,
        ))
    }

    /// Run a stratified 11-family sampling within the authored family catalog; this tests compiler validity, not open-ended expressiveness.
    pub fn closed_catalog_validity(
        n_per_family: usize,
        seed: u64,
    ) -> Result<ClosedCatalogValidityAudit, String> {
        let mut rng = Rng::new(seed);
        let mut family_stats = Vec::with_capacity(11);
        let mut candidates_pool: Vec<ExoticPhenotypeShowcase> = Vec::new();
        let mut total_sampled = 0usize;
        let mut total_passed = 0usize;

        for (f_idx, &fam_name) in Self::CANONICAL_11_FAMILIES.iter().enumerate() {
            let mut sampled = 0usize;
            let mut passed = 0usize;
            let mut exotic_sum = 0.0;
            let mut sym_sum = 0.0;
            let mut min_clearance = f64::INFINITY;
            let mut min_jacobian = f64::INFINITY;

            for s_idx in 0..n_per_family {
                let prog = Self::sample_stratified_11family(&mut rng, f_idx)?.into_child()?;
                sampled += 1;
                total_sampled += 1;
                if let Ok((norm, pheno, mesh)) = prog.canonicalize_and_preflight() {
                    let census = mesh.edge_census();
                    if census.open_boundary_edges == 0
                        && census.non_manifold_edges == 0
                        && mesh.signed_volume() > 0.0
                    {
                        passed += 1;
                        total_passed += 1;
                        let ex = norm.exoticness_index()?;
                        exotic_sum += ex;
                        sym_sum += pheno.descriptors.azimuthal_symmetry_order;
                        min_clearance = min_clearance.min(pheno.minimum_wall_clearance_mm);
                        min_jacobian = min_jacobian.min(pheno.min_section_jacobian);
                        if f_idx > 0 && s_idx < 4 {
                            candidates_pool.push(ExoticPhenotypeShowcase {
                                id: format!("exotic-{fam_name}-{s_idx:02}"),
                                topology_class: norm.classify_topology(),
                                genome_sha256: norm.canonical_sha256()?,
                                exoticness_index: ex,
                                total_entry_jets: if norm.inlet_split.enabled {
                                    2 * norm.inlet_split.branches_per_port
                                } else {
                                    2
                                },
                                tpms_basis: format!("{:?}", norm.tpms_insert.basis),
                                ribbon_vanes: norm.ribbon_cascade.vane_count,
                                skimmer_flutes: norm.skimmer.capture_flutes,
                                azimuthal_symmetry_order: pheno
                                    .descriptors
                                    .azimuthal_symmetry_order,
                                minimum_wall_clearance_mm: round4(pheno.minimum_wall_clearance_mm),
                                min_section_jacobian: round6(pheno.min_section_jacobian),
                            });
                        }
                    }
                }
            }

            let denom = passed.max(1) as f64;
            family_stats.push(FamilySamplingStats {
                topology_class: fam_name.to_string(),
                sampled_count: sampled,
                preflight_passed_count: passed,
                preflight_pass_fraction: round6(passed as f64 / sampled.max(1) as f64),
                mean_exoticness_index: round6(exotic_sum / denom),
                mean_azimuthal_symmetry_order: round6(sym_sum / denom),
                min_wall_clearance_mm: round4(min_clearance),
                min_section_jacobian: round6(min_jacobian),
            });
        }

        // Select top 8 most exotic valid phenotypes with at most 1 per family first to maximize topological diversity
        candidates_pool.sort_by(|a, b| b.exoticness_index.total_cmp(&a.exoticness_index));
        let mut top_8 = Vec::with_capacity(8);
        let mut used_fams = std::collections::BTreeSet::new();
        for c in &candidates_pool {
            if used_fams.insert(c.topology_class.clone()) {
                top_8.push(c.clone());
                if top_8.len() == 8 {
                    break;
                }
            }
        }
        top_8.sort_by(|a, b| b.exoticness_index.total_cmp(&a.exoticness_index));

        Ok(ClosedCatalogValidityAudit {
            seed,
            n_per_family,
            total_sampled,
            total_preflight_passed: total_passed,
            overall_preflight_pass_fraction: round6(
                total_passed as f64 / total_sampled.max(1) as f64,
            ),
            family_stats,
            top_8_exotic_valid_phenotypes: top_8,
        })
    }

    /// Run a chained multi-step mutation + crossover random-walk stress audit starting from each of
    /// the 11 canonical families (`steps_per_family >= 20`, verifying `canonicalize_and_preflight`
    /// and `open_boundary_edges == 0` / `non_manifold_edges == 0` on every intermediate offspring).
    pub fn operator_chain_validity(
        steps_per_family: usize,
        seed: u64,
    ) -> Result<OperatorChainValidityAudit, String> {
        if steps_per_family < 20 {
            return Err("steps_per_family must be >= 20 for operator random-walk audit".into());
        }
        let mut rng = Rng::new(seed);
        let initial_pool: Vec<Self> = (0..11)
            .map(|f_idx| {
                Self::sample_stratified_11family(&mut rng, f_idx)
                    .map_err(String::from)?
                    .into_child()
            })
            .collect::<Result<_, String>>()?;

        let mut family_walk_stats = Vec::with_capacity(11);
        let mut total_steps = 0usize;
        let mut total_passed = 0usize;
        let mut total_family_transitions = 0usize;
        let mut global_visited_families = std::collections::BTreeSet::new();

        for (f_idx, &start_family) in Self::CANONICAL_11_FAMILIES.iter().enumerate() {
            let mut current = initial_pool[f_idx].clone();
            let mut prev_fam = current.classify_topology();
            global_visited_families.insert(prev_fam.clone());
            let mut local_visited = std::collections::BTreeSet::new();
            local_visited.insert(prev_fam.clone());

            let mut steps_passed = 0usize;
            let mut transitions = 0usize;
            let mut max_open_edges = 0usize;
            let mut max_non_manifold_edges = 0usize;
            let mut min_clearance = f64::INFINITY;
            let mut min_jacobian = f64::INFINITY;
            let mut exotic_sum = 0.0;

            for step in 0..steps_per_family {
                let partner_idx = ((rng.next_u64() as usize) + step + f_idx + 1) % 11;
                let partner = &initial_pool[partner_idx];
                current = match step % 5 {
                    0 => current.mutate_diffeomorphic_spline(&mut rng),
                    1 => current.mutate_branch_topology(&mut rng),
                    2 => current.mutate_tpms_grading(&mut rng),
                    3 => current.crossover_non_homologous_macro(partner, &mut rng),
                    _ => current.crossover(partner, &mut rng),
                }?
                .into_child()?;
                total_steps += 1;
                if let Ok((norm, pheno, mesh)) = current.canonicalize_and_preflight() {
                    let census = mesh.edge_census();
                    max_open_edges = max_open_edges.max(census.open_boundary_edges);
                    max_non_manifold_edges = max_non_manifold_edges.max(census.non_manifold_edges);
                    if census.open_boundary_edges == 0
                        && census.non_manifold_edges == 0
                        && mesh.signed_volume() > 0.0
                    {
                        steps_passed += 1;
                        total_passed += 1;
                        let cur_fam = norm.classify_topology();
                        if cur_fam != prev_fam {
                            transitions += 1;
                            total_family_transitions += 1;
                            prev_fam = cur_fam.clone();
                        }
                        local_visited.insert(cur_fam.clone());
                        global_visited_families.insert(cur_fam);
                        min_clearance = min_clearance.min(pheno.minimum_wall_clearance_mm);
                        min_jacobian = min_jacobian.min(pheno.min_section_jacobian);
                        exotic_sum += norm.exoticness_index()?;
                    }
                }
            }

            let denom = steps_passed.max(1) as f64;
            family_walk_stats.push(FamilyRandomWalkStats {
                start_topology_class: start_family.to_string(),
                steps_executed: steps_per_family,
                steps_passed,
                pass_fraction: round6(steps_passed as f64 / steps_per_family as f64),
                family_transitions: transitions,
                distinct_families_visited: local_visited.len(),
                max_open_edges,
                max_non_manifold_edges,
                min_wall_clearance_mm: round4(min_clearance),
                min_section_jacobian: round6(min_jacobian),
                mean_exoticness_index: round6(exotic_sum / denom),
            });
        }

        Ok(OperatorChainValidityAudit {
            seed,
            steps_per_family,
            total_steps,
            total_passed,
            overall_pass_fraction: round6(total_passed as f64 / total_steps.max(1) as f64),
            total_family_transitions,
            global_distinct_families_visited: global_visited_families.len(),
            family_walk_stats,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FamilyRandomWalkStats {
    pub start_topology_class: String,
    pub steps_executed: usize,
    pub steps_passed: usize,
    pub pass_fraction: f64,
    pub family_transitions: usize,
    pub distinct_families_visited: usize,
    pub max_open_edges: usize,
    pub max_non_manifold_edges: usize,
    pub min_wall_clearance_mm: f64,
    pub min_section_jacobian: f64,
    pub mean_exoticness_index: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct OperatorChainValidityAudit {
    pub seed: u64,
    pub steps_per_family: usize,
    pub total_steps: usize,
    pub total_passed: usize,
    pub overall_pass_fraction: f64,
    pub total_family_transitions: usize,
    pub global_distinct_families_visited: usize,
    pub family_walk_stats: Vec<FamilyRandomWalkStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FamilySamplingStats {
    pub topology_class: String,
    pub sampled_count: usize,
    pub preflight_passed_count: usize,
    pub preflight_pass_fraction: f64,
    pub mean_exoticness_index: f64,
    pub mean_azimuthal_symmetry_order: f64,
    pub min_wall_clearance_mm: f64,
    pub min_section_jacobian: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExoticPhenotypeShowcase {
    pub id: String,
    pub topology_class: String,
    pub genome_sha256: String,
    pub exoticness_index: f64,
    pub total_entry_jets: u32,
    pub tpms_basis: String,
    pub ribbon_vanes: u32,
    pub skimmer_flutes: u32,
    pub azimuthal_symmetry_order: f64,
    pub minimum_wall_clearance_mm: f64,
    pub min_section_jacobian: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ClosedCatalogValidityAudit {
    pub seed: u64,
    pub n_per_family: usize,
    pub total_sampled: usize,
    pub total_preflight_passed: usize,
    pub overall_preflight_pass_fraction: f64,
    pub family_stats: Vec<FamilySamplingStats>,
    pub top_8_exotic_valid_phenotypes: Vec<ExoticPhenotypeShowcase>,
}
