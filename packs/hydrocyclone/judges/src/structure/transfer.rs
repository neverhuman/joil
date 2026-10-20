use super::*;

// ---------------------------------------------------------------- V5-19-02 traction mapping

/// A traction facet. Force on the solid is `(-pressure * normal + shear) * area`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Facet {
    pub patch: String,
    pub position_m: V3,
    pub normal: V3,
    pub area_m2: f64,
    pub pressure_pa: f64,
    pub shear_pa: V3,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reaction {
    pub position_m: V3,
    pub force_n: V3,
}

/// Preallocated mapping budget: errors are `absolute + relative * |source|`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MappingBudget {
    pub relative: f64,
    pub force_abs_n: f64,
    pub moment_abs_n_m: f64,
    pub area_relative: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TractionTransfer {
    pub design_sha256: Sha256Hex,
    /// Subject the accepted CFD traction field belongs to.
    pub source_design_sha256: Sha256Hex,
    pub interpolation_id: String,
    pub interpolation_sha256: Sha256Hex,
    pub solid_mesh_sha256: Sha256Hex,
    pub origin_m: V3,
    pub required_patches: Vec<String>,
    pub source: Vec<Facet>,
    pub mapped: Vec<Facet>,
    pub reactions: Vec<Reaction>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Sums {
    force: V3,
    shear_force: V3,
    moment: V3,
    vector_area: V3,
    area: f64,
}

fn integrate<'a>(facets: impl Iterator<Item = &'a Facet>, origin: V3) -> Sums {
    let mut s = Sums::default();
    for f in facets {
        let shear: V3 = std::array::from_fn(|i| f.shear_pa[i] * f.area_m2);
        let force: V3 =
            std::array::from_fn(|i| -f.pressure_pa * f.normal[i] * f.area_m2 + shear[i]);
        s.force = add(s.force, force);
        s.shear_force = add(s.shear_force, shear);
        s.moment = add(s.moment, cross(sub(f.position_m, origin), force));
        s.vector_area = add(
            s.vector_area,
            std::array::from_fn(|i| f.normal[i] * f.area_m2),
        );
        s.area += f.area_m2;
    }
    s
}

fn check_facets(name: &str, facets: &[Facet]) -> Result<(), JudgeError> {
    if facets.is_empty() {
        return Err(invalid(format!("{name} has no facets")));
    }
    for f in facets {
        if f.patch.trim().is_empty()
            || !f.area_m2.is_finite()
            || f.area_m2 <= 0.0
            || !f.pressure_pa.is_finite()
            || !finite3(&f.position_m)
            || !finite3(&f.normal)
            || !finite3(&f.shear_pa)
            || (norm(f.normal) - 1.0).abs() > 1e-9
        {
            return Err(invalid(format!(
                "{name}: finite SI facets with positive area, unit normals and a patch id required"
            )));
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq)]
pub struct PatchError {
    pub force_residual_n: f64,
    pub shear_residual_n: f64,
    pub moment_residual_n_m: f64,
    pub area_relative_error: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransferReport {
    pub design_sha256: Sha256Hex,
    pub interpolation_id: String,
    pub interpolation_sha256: Sha256Hex,
    pub solid_mesh_sha256: Sha256Hex,
    pub source_force_n: V3,
    pub mapped_force_n: V3,
    pub source_moment_n_m: V3,
    pub mapped_moment_n_m: V3,
    pub reaction_force_n: V3,
    pub reaction_moment_n_m: V3,
    pub patch_errors: BTreeMap<String, PatchError>,
    pub verdict: Verdict,
}

fn group_by_patch(facets: &[Facet]) -> BTreeMap<&str, Vec<&Facet>> {
    let mut m: BTreeMap<&str, Vec<&Facet>> = BTreeMap::new();
    for f in facets {
        m.entry(f.patch.as_str()).or_default().push(f);
    }
    m
}

/// Judge a CFD-to-solid traction transfer: patches, shear, normals, conservation and reactions.
pub fn judge_transfer(
    t: &TractionTransfer,
    budget: &Field<MappingBudget>,
    structural_mesh: &Sha256Hex,
) -> Result<TransferReport, JudgeError> {
    check_facets("source", &t.source)?;
    check_facets("mapped", &t.mapped)?;
    if !finite3(&t.origin_m)
        || t.interpolation_id.trim().is_empty()
        || t.required_patches.is_empty()
        || t.reactions
            .iter()
            .any(|r| !finite3(&r.position_m) || !finite3(&r.force_n))
    {
        return Err(invalid(
            "transfer needs finite origin/reactions, an interpolation id and required patches",
        ));
    }
    let mut parts = Vec::new();
    let b = match budget {
        Field::Admitted { value, .. } => {
            let ok = [value.force_abs_n, value.moment_abs_n_m]
                .iter()
                .all(|x| x.is_finite() && *x >= 0.0)
                && [value.relative, value.area_relative]
                    .iter()
                    .all(|x| x.is_finite() && (0.0..1.0).contains(x));
            if !ok {
                return Err(invalid(
                    "mapping budget needs finite nonnegative terms, ratios in [0,1)",
                ));
            }
            Some(value)
        }
        _ => {
            parts.push(open("mapping_budget:not_admitted"));
            None
        }
    };
    if t.source_design_sha256 != t.design_sha256 {
        parts.push(fail("source_field_belongs_to_another_design"));
    }
    if &t.solid_mesh_sha256 != structural_mesh {
        parts.push(fail("solid_mesh_is_not_the_structural_mesh"));
    }
    let (src, map) = (group_by_patch(&t.source), group_by_patch(&t.mapped));
    let within = |err: f64, abs: f64, rel: f64, reference: f64| err <= abs + rel * reference;

    let mut patch_errors = BTreeMap::new();
    for patch in t.required_patches.iter().collect::<BTreeSet<_>>() {
        let (Some(s), Some(m)) = (src.get(patch.as_str()), map.get(patch.as_str())) else {
            parts.push(fail(format!("patch_missing:{patch}")));
            continue;
        };
        let (s, m) = (
            integrate(s.iter().copied(), t.origin_m),
            integrate(m.iter().copied(), t.origin_m),
        );
        let err = PatchError {
            force_residual_n: norm(sub(m.force, s.force)),
            shear_residual_n: norm(sub(m.shear_force, s.shear_force)),
            moment_residual_n_m: norm(sub(m.moment, s.moment)),
            area_relative_error: (m.area - s.area).abs() / s.area,
        };
        if let Some(b) = b {
            if !within(
                err.force_residual_n,
                b.force_abs_n,
                b.relative,
                norm(s.force),
            ) {
                parts.push(fail(format!("patch_force_not_conserved:{patch}")));
            }
            if !within(
                err.shear_residual_n,
                b.force_abs_n,
                b.relative,
                norm(s.shear_force),
            ) {
                parts.push(fail(format!("patch_shear_not_conserved:{patch}")));
            }
            if !within(
                err.moment_residual_n_m,
                b.moment_abs_n_m,
                b.relative,
                norm(s.moment),
            ) {
                parts.push(fail(format!("patch_moment_not_conserved:{patch}")));
            }
            if err.area_relative_error > b.area_relative
                || norm(sub(m.vector_area, s.vector_area)) > b.area_relative * s.area
            {
                parts.push(fail(format!("patch_area_or_normals_not_preserved:{patch}")));
            }
        }
        patch_errors.insert(patch.clone(), err);
    }

    let (s, m) = (
        integrate(t.source.iter(), t.origin_m),
        integrate(t.mapped.iter(), t.origin_m),
    );
    if norm(s.shear_force) == 0.0 {
        parts.push(fail("shear_absent_in_source_field"));
    } else if norm(m.shear_force) == 0.0 {
        parts.push(fail("shear_missing_after_transfer"));
    }
    if t.mapped
        .iter()
        .all(|f| (f.pressure_pa - COPIED_CT35_PRESSURE_PA).abs() <= 0.005)
    {
        parts.push(fail("copied_ct35_reference_pressure"));
    }
    let (mut rf, mut rm) = ([0.0; 3], [0.0; 3]);
    for r in &t.reactions {
        rf = add(rf, r.force_n);
        rm = add(rm, cross(sub(r.position_m, t.origin_m), r.force_n));
    }
    if t.reactions.is_empty() {
        parts.push(open("reactions:not_supplied"));
    } else if let Some(b) = b {
        if !within(
            norm(add(m.force, rf)),
            b.force_abs_n,
            b.relative,
            norm(m.force),
        ) || !within(
            norm(add(m.moment, rm)),
            b.moment_abs_n_m,
            b.relative,
            norm(m.moment),
        ) {
            parts.push(fail("reactions_do_not_balance_mapped_load"));
        }
    }
    if let Some(b) = b {
        if !within(
            norm(sub(m.force, s.force)),
            b.force_abs_n,
            b.relative,
            norm(s.force),
        ) || !within(
            norm(sub(m.moment, s.moment)),
            b.moment_abs_n_m,
            b.relative,
            norm(s.moment),
        ) {
            parts.push(fail("total_force_or_moment_not_conserved"));
        }
    }
    Ok(TransferReport {
        design_sha256: t.design_sha256.clone(),
        interpolation_id: t.interpolation_id.clone(),
        interpolation_sha256: t.interpolation_sha256.clone(),
        solid_mesh_sha256: t.solid_mesh_sha256.clone(),
        source_force_n: s.force,
        mapped_force_n: m.force,
        source_moment_n_m: s.moment,
        mapped_moment_n_m: m.moment,
        reaction_force_n: rf,
        reaction_moment_n_m: rm,
        patch_errors,
        verdict: Verdict::combine(parts),
    })
}
