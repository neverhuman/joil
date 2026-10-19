use crate::*;

impl DesignIrV1 {
    /// Refuse an IR whose identity or recorded envelope differs from the CT-35 reference
    /// (`V5-01-05`). A changed downstream tail or a schema-1 envelope is a different subject and is
    /// never compiled as if it were the reference.
    pub fn verify_reference_envelope(&self) -> Result<(), IrError> {
        if self.ir_schema_version != IR_SCHEMA_VERSION || self.ir_family != IR_FAMILY {
            return Err(IrError::SchemaOrFamilyUnsupported {
                schema_version: self.ir_schema_version,
                family: self.ir_family.clone(),
                reason: "legacy IR identity recorded a non-reference envelope; rebuild the IR from its program"
                    .into(),
            });
        }
        let envelope = self.nodes.iter().find_map(|node| match node {
            IrNode::BodyEnvelope {
                d1_m,
                d2_m,
                d3_m,
                l1_m,
                l2_m,
                l3_m,
                z_underflow_m,
                ..
            } => Some([*d1_m, *d2_m, *d3_m, *l1_m, *l2_m, *l3_m, *z_underflow_m]),
            _ => None,
        });
        let Some(recorded) = envelope else {
            return Err(IrError::InvalidParameter {
                gene_path: "node.body_envelope".into(),
                reason: "IR has no BodyEnvelope".into(),
            });
        };
        let lc_m = ((CT35_D1_M - CT35_D2_M) / 2.0) / 10.0_f64.to_radians().tan();
        let reference = [
            ("d1_m", CT35_D1_M),
            ("d2_m", CT35_D2_M),
            ("d3_m", CT35_D3_M),
            ("l1_m", CT35_L1_M),
            ("l2_m", CT35_L2_M),
            ("l3_m", CT35_L3_M),
            ("z_underflow_m", CT35_L1_M + lc_m + CT35_L2_M + CT35_L3_M),
        ];
        for (got, (name, want)) in recorded.into_iter().zip(reference) {
            if !got.is_finite() || (got - want).abs() > 1e-9 {
                return Err(IrError::InvalidParameter {
                    gene_path: format!("node.body_envelope.{name}"),
                    reason: format!(
                        "recorded {got} m differs from the CT-35 reference {want} m (changed frozen envelope or downstream tail)"
                    ),
                });
            }
        }
        Ok(())
    }

    /// Evaluate the backend support matrix for a target `BackendId` (`IR-02`).
    /// Fails closed with `Err(IrError::UnsupportedFeature)` if any active gene is unsupported on `backend`.
    pub fn check_backend_support(
        &self,
        backend: BackendId,
    ) -> Result<BackendCoverageReport, IrError> {
        let mut coverages = Vec::new();
        let mut all_qualified = true;

        for (gene_name, mapping) in &self.gene_mapping {
            let (is_active, node_ids) = match mapping {
                GeneIrMapping::Active { node_ids, .. } => (true, node_ids.clone()),
                GeneIrMapping::Inactive { .. } => (false, Vec::new()),
            };

            let support = if !is_active {
                SupportLevel::Exact
            } else {
                self.evaluate_active_gene_support(gene_name, backend)
            };

            if is_active {
                if let SupportLevel::Unsupported { ref reason } = support {
                    return Err(IrError::UnsupportedFeature {
                        backend,
                        feature: gene_name.clone(),
                        reason: reason.clone(),
                    });
                }
                if !support.is_supported_for_geometry_qualification() {
                    all_qualified = false;
                }
            }

            coverages.push(FeatureBackendCoverage {
                gene_name: gene_name.clone(),
                is_active,
                node_ids,
                backend,
                support,
            });
        }

        Ok(BackendCoverageReport {
            ir_sha256: self.ir_sha256.clone(),
            backend,
            qualified_for_geometry: all_qualified,
            feature_coverages: coverages,
        })
    }

    fn evaluate_active_gene_support(&self, gene_name: &str, backend: BackendId) -> SupportLevel {
        // Check for invalid/unsupported internal node configurations first
        for node in &self.nodes {
            match node {
                IrNode::TpmsPeripheralShell { basis, .. }
                    if *basis == TpmsBasisIr::UnsupportedCustom =>
                {
                    return SupportLevel::Unsupported {
                        reason: "unknown/custom TPMS basis is unsupported on all backends".into(),
                    };
                }
                IrNode::VortexFinderSkimmer { capture_flutes, .. }
                    if !matches!(capture_flutes, 0 | 4 | 6 | 8) =>
                {
                    return SupportLevel::Unsupported {
                        reason: format!(
                            "skimmer capture_flutes={capture_flutes} not in supported set {{0, 4, 6, 8}}"
                        ),
                    };
                }
                _ => {}
            }
        }

        match (backend, gene_name) {
            (_, "body_envelope" | "terminals") => SupportLevel::Exact,

            // Strict analytical CSG B-rep backend rejects implicit field lofts (TPMS & helical ribbon fields)
            (BackendId::PythonCadQueryStrictCsgOnly, "tpms_insert") => SupportLevel::Unsupported {
                reason: "tpms_insert is an implicit periodic surface field; unsupported in strict analytical CSG-only B-rep mode".into(),
            },
            (BackendId::PythonCadQueryStrictCsgOnly, "ribbon_cascade") => SupportLevel::Unsupported {
                reason: "ribbon_cascade uses stream-surface harmonic modulation; unsupported in strict analytical CSG-only B-rep mode".into(),
            },
            (BackendId::PythonCadQueryStrictCsgOnly, _) => SupportLevel::Exact,

            // Legacy pre-v4 CFD surface generator did not enact internal splitter vanes or skimmer sleeves
            (BackendId::LegacyUnenactedCfdSurface, "inlet_split") => SupportLevel::Unsupported {
                reason: "legacy pre-v4 CFD surface extruded 2 unsplit ports without internal splitter vanes".into(),
            },
            (BackendId::LegacyUnenactedCfdSurface, "skimmer") => SupportLevel::Unsupported {
                reason: "legacy pre-v4 CFD surface omitted internal skimmer sleeve walls".into(),
            },
            (BackendId::LegacyUnenactedCfdSurface, "tpms_insert" | "ribbon_cascade") => {
                SupportLevel::WallCorrugationProxy {
                    proxy_note: "radial outer-wall corrugation proxy r_wall(theta, z); does not bound volume, wetted area, or topology of free-standing vanes or a 3D TPMS lattice".into(),
                }
            }
            (BackendId::LegacyUnenactedCfdSurface, _) => SupportLevel::ConservativeApproximation {
                bound_note: "radial wall ring modulation only".into(),
            },

            // Fast radial screen mesh is a single-shell enclosing surface (screen only for internal splitters/skimmer)
            (BackendId::RustFastRadialSurface, "inlet_split" | "skimmer") => SupportLevel::ScreenOnly {
                reason: "single-shell radial preview mesh does not embed internal splitter vanes or skimmer bore walls; use RustMultiPatchCfdSurface or PythonCadQueryBrep".into(),
            },
            (BackendId::RustFastRadialSurface, "tpms_insert" | "ribbon_cascade") => {
                SupportLevel::WallCorrugationProxy {
                    proxy_note: "radial outer-wall corrugation proxy r_wall(theta, z); does not bound volume, wetted area, or topology of free-standing vanes or a 3D TPMS lattice".into(),
                }
            }
            (BackendId::RustFastRadialSurface, _) => SupportLevel::ConservativeApproximation {
                bound_note: "48x64 structured radial-axial surface tessellation".into(),
            },

            // Full v4 Multi-Patch CFD Surface, Python CadQuery B-rep (with NURBS field loft), and Voxel/SDF observer:
            // ribbon_cascade and tpms_insert are radial wall corrugations r_wall(theta, z), NOT bounds on free-standing vanes or 3D TPMS lattices (`IR-R4`).
            (
                BackendId::RustMultiPatchCfdSurface
                | BackendId::PythonCadQueryBrep
                | BackendId::VoxelSdfObserver,
                "tpms_insert" | "ribbon_cascade",
            ) => SupportLevel::WallCorrugationProxy {
                proxy_note: "radial outer-wall corrugation proxy r_wall(theta, z); does not bound volume, wetted area, or topology of free-standing vanes or a 3D TPMS lattice".into(),
            },
            (
                BackendId::RustMultiPatchCfdSurface
                | BackendId::PythonCadQueryBrep
                | BackendId::VoxelSdfObserver,
                _,
            ) => SupportLevel::Exact,

            // CUDA reduced screen is strictly a low-order screen
            (BackendId::CudaReducedScreen, _) => SupportLevel::ScreenOnly {
                reason: "CUDA axial SDF + reduced trajectory tensor screen".into(),
            },
        }
    }
}

// ============================================================================
// IR-09: Split Conflated Geometry Metrics (Physical Meaning & Unit per Metric)
// ============================================================================
