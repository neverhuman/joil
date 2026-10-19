use super::*;

/// Canonical OpenFOAM CFD boundary patch identifier (`CAD-06`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CfdPatch {
    Wall,
    Inlet1,
    Inlet2,
    Overflow,
    Underflow,
}

impl CfdPatch {
    pub fn as_str(&self) -> &'static str {
        match self {
            CfdPatch::Wall => "wall",
            CfdPatch::Inlet1 => "inlet1",
            CfdPatch::Inlet2 => "inlet2",
            CfdPatch::Overflow => "overflow",
            CfdPatch::Underflow => "underflow",
        }
    }

    pub fn openfoam_type(&self) -> &'static str {
        match self {
            CfdPatch::Wall => "wall",
            _ => "patch",
        }
    }

    pub fn all() -> [CfdPatch; 5] {
        [
            CfdPatch::Wall,
            CfdPatch::Inlet1,
            CfdPatch::Inlet2,
            CfdPatch::Overflow,
            CfdPatch::Underflow,
        ]
    }
}

/// Geometric and orientation audit for a single named CFD boundary patch (`CAD-06`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatchMetrics {
    pub patch_name: String,
    pub openfoam_type: String,
    pub triangle_count: usize,
    pub area_m2: f64,
    pub expected_area_m2: Option<f64>,
    pub relative_area_error: Option<f64>,
    pub centroid_m: [f64; 3],
    pub mean_normal: [f64; 3],
    pub expected_normal: Option<[f64; 3]>,
    pub min_normal_alignment: Option<f64>,
    pub plane_axis: Option<usize>,
    pub plane_position_m: Option<f64>,
}

/// Full audit summary for a multi-patch SI CFD surface (`CAD-04` & `CAD-06`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MultiPatchAuditReport {
    pub design_sha256: String,
    #[serde(default)]
    pub design_ir_sha256: String,
    /// Legacy in-memory field retained for internal callers; omitted from serialized subject (`IR-06`).
    #[serde(skip_serializing, default)]
    pub topology_class: String,
    pub units: String,
    pub outlet_extension_m: f64,
    pub total_vertices: usize,
    pub total_triangles: usize,
    pub signed_volume_m3: f64,
    pub census: EdgeCensus,
    pub watertight_and_manifold: bool,
    pub all_terminal_patches_valid: bool,
    pub patches: BTreeMap<String, PatchMetrics>,
    #[serde(default)]
    pub derived_taxon: Option<joil_ir::PhenotypeTaxon>,
    #[serde(default)]
    pub enactment_receipt: Option<joil_ir::FeatureEnactmentReceipt>,
}

/// Multi-patch watertight 3D fluid domain surface in SI metres (`m`) ready for OpenFOAM `snappyHexMesh`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MultiPatchSurface {
    pub vertices_m: Vec<P3>,
    pub triangles: Vec<[usize; 3]>,
    pub triangle_patches: Vec<CfdPatch>,
    #[serde(default)]
    pub triangle_origin_ids: Vec<u16>,
    #[serde(default)]
    pub origin_table: Vec<joil_ir::SemanticOriginTrace>,
    #[serde(default)]
    pub internal_feature_triangles: Vec<([P3; 3], u16)>,
    pub audit: MultiPatchAuditReport,
}

impl MultiPatchSurface {
    /// Compile any `DesignProgram` into a conformal, watertight 5-patch SI surface (`wall`, `inlet1`, `inlet2`, `overflow`, `underflow`)
    /// directly from the canonical `joil-ir::IrCompiledMesh` (`IR-R1`).
    pub fn from_program(prog: &DesignProgram, outlet_extension_m: f64) -> Result<Self, String> {
        if !outlet_extension_m.is_finite() || !(0.0..=0.5).contains(&outlet_extension_m) {
            return Err("outlet_extension_m must be finite in [0.0, 0.5] m".into());
        }
        let p = prog.normalize()?;
        let g = baseline();
        let d = g.derive()?;
        let par = &g.parameters;

        let r_plenum_m = par.plenum_radius_mm * 1e-3;
        let r3_m = (d.d3_mm / 2.0) * 1e-3;
        let z_orifice_m =
            -(2.0 * (d.overflow_diameter_mm / 2.0) * par.orifice_length_over_do) * 1e-3;
        let z_overflow_m = z_orifice_m - par.plenum_length_mm * 1e-3;
        let z_underflow_m = d.z_underflow_mm * 1e-3 + outlet_extension_m;
        let x_inlet_m = par.inlet_length_cfd_mm * 1e-3;

        // Compile DesignIrV1 and build the canonical watertight 2-manifold IrCompiledMesh (`IR-02`, `IR-04`, `IR-R1`)
        let ir = crate::geometry::compile_design_ir(&p).map_err(|e| e.to_string())?;
        let ir_mesh = joil_ir::IrCompiledMesh::compile_with_extension(
            &ir,
            joil_ir::BackendId::RustMultiPatchCfdSurface,
            outlet_extension_m,
        )
        .map_err(|e| e.to_string())?;
        let enactment = ir_mesh
            .verify_feature_enactment(&ir)
            .map_err(|e| e.to_string())?;
        let f0_report = joil_ir::VoxelSdfObserver::observe(&ir, &ir_mesh);

        let oid_to_patch: BTreeMap<u16, CfdPatch> = ir_mesh
            .origin_table
            .iter()
            .map(|o| {
                let patch = match o.cfd_patch.as_str() {
                    "inlet1" => CfdPatch::Inlet1,
                    "inlet2" => CfdPatch::Inlet2,
                    "overflow" => CfdPatch::Overflow,
                    "underflow" => CfdPatch::Underflow,
                    _ => CfdPatch::Wall,
                };
                (o.origin_id, patch)
            })
            .collect();

        let vertices_m = ir_mesh.vertices_m;
        let triangles = ir_mesh.triangles;
        let triangle_patches: Vec<CfdPatch> = ir_mesh
            .triangle_origin_ids
            .iter()
            .map(|oid| oid_to_patch.get(oid).copied().unwrap_or(CfdPatch::Wall))
            .collect();

        // Topological watertightness & volume check on the compiled IR mesh
        let combined = SolidMesh {
            vertices: vertices_m.clone(),
            triangles: triangles.clone(),
        };
        let census = combined.edge_census();
        let signed_volume_m3 = combined.signed_volume();
        let watertight_and_manifold = census.open_boundary_edges == 0
            && census.non_manifold_edges == 0
            && signed_volume_m3 > 0.0;

        // Compute per-patch metrics and terminal patch verification (CAD-06)
        let expected_inlet_area_m2 = d.inlet_area_each_mm2 * 1e-6;
        let expected_overflow_area_m2 = PI * r_plenum_m * r_plenum_m;
        let expected_underflow_area_m2 = PI * r3_m * r3_m;

        let mut patches_map = BTreeMap::new();
        let mut all_terminal_patches_valid = true;

        for patch in CfdPatch::all() {
            let mut tri_cnt = 0usize;
            let mut area_sum = 0.0;
            let mut cx = 0.0;
            let mut cy = 0.0;
            let mut cz = 0.0;
            let mut nx_sum = 0.0;
            let mut ny_sum = 0.0;
            let mut nz_sum = 0.0;
            let (exp_area, exp_norm, plane_axis, plane_pos) = match patch {
                CfdPatch::Wall => (None, None, None, None),
                CfdPatch::Inlet1 => (
                    Some(expected_inlet_area_m2),
                    Some([-1.0, 0.0, 0.0]),
                    Some(0usize),
                    Some(-x_inlet_m),
                ),
                CfdPatch::Inlet2 => (
                    Some(expected_inlet_area_m2),
                    Some([1.0, 0.0, 0.0]),
                    Some(0usize),
                    Some(x_inlet_m),
                ),
                CfdPatch::Overflow => (
                    Some(expected_overflow_area_m2),
                    Some([0.0, 0.0, -1.0]),
                    Some(2usize),
                    Some(z_overflow_m),
                ),
                CfdPatch::Underflow => (
                    Some(expected_underflow_area_m2),
                    Some([0.0, 0.0, 1.0]),
                    Some(2usize),
                    Some(z_underflow_m),
                ),
            };
            let mut min_align = 1.0_f64;

            for (idx, &[i0, i1, i2]) in triangles.iter().enumerate() {
                if triangle_patches[idx] != patch {
                    continue;
                }
                tri_cnt += 1;
                let a = vertices_m[i0];
                let b = vertices_m[i1];
                let c = vertices_m[i2];
                let ux = b[0] - a[0];
                let uy = b[1] - a[1];
                let uz = b[2] - a[2];
                let vx = c[0] - a[0];
                let vy = c[1] - a[1];
                let vz = c[2] - a[2];
                let crx = uy * vz - uz * vy;
                let cry = uz * vx - ux * vz;
                let crz = ux * vy - uy * vx;
                let dbl_area = (crx * crx + cry * cry + crz * crz).sqrt();
                let area = 0.5 * dbl_area;
                let unit_n = if dbl_area > 1e-24 {
                    [crx / dbl_area, cry / dbl_area, crz / dbl_area]
                } else {
                    [0.0, 0.0, 0.0]
                };
                area_sum += area;
                cx += area * (a[0] + b[0] + c[0]) / 3.0;
                cy += area * (a[1] + b[1] + c[1]) / 3.0;
                cz += area * (a[2] + b[2] + c[2]) / 3.0;
                nx_sum += area * unit_n[0];
                ny_sum += area * unit_n[1];
                nz_sum += area * unit_n[2];
                if let Some(en) = exp_norm {
                    let dot = unit_n[0] * en[0] + unit_n[1] * en[1] + unit_n[2] * en[2];
                    min_align = min_align.min(dot);
                }
            }

            let centroid_m = if area_sum > 1e-24 {
                [cx / area_sum, cy / area_sum, cz / area_sum]
            } else {
                [0.0, 0.0, 0.0]
            };
            let n_mag = (nx_sum * nx_sum + ny_sum * ny_sum + nz_sum * nz_sum)
                .sqrt()
                .max(1e-24);
            let mean_normal = [nx_sum / n_mag, ny_sum / n_mag, nz_sum / n_mag];
            let rel_err = exp_area.map(|ea| (area_sum / ea) - 1.0);

            if let (Some(re), Some(_)) = (rel_err, exp_norm) {
                if tri_cnt == 0 || re.abs() > 0.002 || min_align < 0.999 {
                    all_terminal_patches_valid = false;
                }
            }

            patches_map.insert(
                patch.as_str().to_string(),
                PatchMetrics {
                    patch_name: patch.as_str().to_string(),
                    openfoam_type: patch.openfoam_type().to_string(),
                    triangle_count: tri_cnt,
                    area_m2: area_sum,
                    expected_area_m2: exp_area,
                    relative_area_error: rel_err,
                    centroid_m,
                    mean_normal,
                    expected_normal: exp_norm,
                    min_normal_alignment: exp_norm.map(|_| min_align),
                    plane_axis,
                    plane_position_m: plane_pos,
                },
            );
        }

        let audit = MultiPatchAuditReport {
            design_sha256: p.canonical_sha256()?,
            design_ir_sha256: ir.ir_sha256.clone(),
            topology_class: p.topology_class,
            units: "m".to_string(),
            outlet_extension_m,
            total_vertices: vertices_m.len(),
            total_triangles: triangles.len(),
            signed_volume_m3,
            census,
            watertight_and_manifold,
            all_terminal_patches_valid,
            patches: patches_map,
            derived_taxon: Some(f0_report.derived_taxon),
            enactment_receipt: Some(enactment),
        };

        Ok(Self {
            vertices_m,
            triangles,
            triangle_patches,
            triangle_origin_ids: ir_mesh.triangle_origin_ids,
            origin_table: ir_mesh.origin_table,
            internal_feature_triangles: ir_mesh.internal_feature_triangles,
            audit,
        })
    }

    /// Write multi-region ASCII STL (`solid wall ... endsolid wall`, `solid inlet1 ...`, etc.)
    /// directly compatible with OpenFOAM `snappyHexMesh` multi-region feature snapping.
    pub fn write_openfoam_ascii_stl(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let file = File::create(path).map_err(|e| e.to_string())?;
        let mut w = BufWriter::new(file);

        let write_facet = |w: &mut BufWriter<File>, a: P3, b: P3, c: P3| -> Result<(), String> {
            let ux = b[0] - a[0];
            let uy = b[1] - a[1];
            let uz = b[2] - a[2];
            let vx = c[0] - a[0];
            let vy = c[1] - a[1];
            let vz = c[2] - a[2];
            let nx = uy * vz - uz * vy;
            let ny = uz * vx - ux * vz;
            let nz = ux * vy - uy * vx;
            let norm = (nx * nx + ny * ny + nz * nz).sqrt().max(1e-24);
            writeln!(
                w,
                " facet normal {:.8e} {:.8e} {:.8e}",
                nx / norm,
                ny / norm,
                nz / norm
            )
            .map_err(|e| e.to_string())?;
            writeln!(w, "  outer loop").map_err(|e| e.to_string())?;
            for v in [a, b, c] {
                writeln!(w, "   vertex {:.8e} {:.8e} {:.8e}", v[0], v[1], v[2])
                    .map_err(|e| e.to_string())?;
            }
            writeln!(w, "  endloop\n endfacet").map_err(|e| e.to_string())?;
            Ok(())
        };

        for patch in CfdPatch::all() {
            let name = patch.as_str();
            writeln!(w, "solid {name}").map_err(|e| e.to_string())?;
            for (idx, &[i0, i1, i2]) in self.triangles.iter().enumerate() {
                if self.triangle_patches[idx] != patch {
                    continue;
                }
                write_facet(
                    &mut w,
                    self.vertices_m[i0],
                    self.vertices_m[i1],
                    self.vertices_m[i2],
                )?;
            }
            if patch == CfdPatch::Wall {
                for &(pts, _) in &self.internal_feature_triangles {
                    write_facet(&mut w, pts[0], pts[1], pts[2])?;
                }
            }
            writeln!(w, "endsolid {name}").map_err(|e| e.to_string())?;
        }
        w.flush().map_err(|e| e.to_string())?;
        Ok(())
    }
}
