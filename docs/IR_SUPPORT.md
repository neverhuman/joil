# JOIL Design IR v1, Backend Support Matrix & Independent F0 Architecture (`IR-01`..`IR-12`)

This document specifies the canonical **Design Intermediate Representation (`DesignIrV1`)**, the backend support matrix across all geometry/mesh/screen compilers, semantic boundary origin tracing, feature-enactment verification, geometry-derived taxonomy, prior-vs-observed descriptor separation, and the independent F0 qualification pipeline implemented in [`crates/joil-ir`](../crates/joil-ir/src/lib.rs), [`crates/joil-core`](../crates/joil-core/src/phenotype.rs), [`cad/program/exotic_graph.py`](../cad/program/exotic_graph.py), [`tools/f0_enactment.py`](../tools/f0_enactment.py), and [`tools/f0_voxel.py`](../tools/f0_voxel.py).

---

## 1. Canonical Design IR (`DesignIrV1`, `IR-01`)

Every `DesignProgram` (`ct35-chamber-wall-field-v1`, `ct35-exotic-head-v2`, `ct35-exotic-head-v3`, `ct35-legacy-macro-v4`) lowers into a single canonical SI-unit (`m`, `m^2`, `m^3`, `rad`) [`DesignIrV1`](../crates/joil-ir/src/lib.rs) graph before any CAD B-rep, OpenFOAM multi-patch surface, glTF scene, CUDA screen bundle, or F0 audit is constructed.

### Invariant: Complete Gene-to-IR Coverage
Every top-level gene group in `DesignProgram` is explicitly recorded in `DesignIrV1::gene_mapping`:
- **`GeneIrMapping::Active { node_ids, semantic_boundary_ids }`**: Maps an active mechanism to $\ge 1$ concrete `IrNode` in `DesignIrV1::nodes` and $\ge 1$ `SemanticOriginTrace` in `DesignIrV1::semantic_origins`.
- **`GeneIrMapping::Inactive { reason, canonical_default_verified: true }`**: Certifies that an inactive mechanism has been canonicalized to its exact baseline default so inactive parameter noise cannot alter `ir_sha256` or create duplicate genotypes.

| Gene Group | Active Condition | Lowered `IrNode` Variant(s) | Semantic Origin Prefix |
| :--- | :--- | :--- | :--- |
| `body_envelope` | Always active | `IrNode::AxisymmetricBodyEnvelope` | `wall.{plenum,orifice,chamber,cone,collar_and_taper,tail,inlet1_duct,inlet2_duct}` |
| `terminals` | Always active (immutable) | `IrNode::TerminalPatch` ($\times 4$: `inlet1`, `inlet2`, `overflow`, `underflow`) | `terminal.{inlet1,inlet2,overflow,underflow}` |
| `inlet_ramp` | `axial_drop_mm > 0` or `radial_slot_factor != 1.0` | `IrNode::InletRamp` | `feature.inlet_ramp` |
| `inlet_split` | `enabled == true` (`branches_per_port` $\in \{2, 3, 4\}$) | `IrNode::SplitterVaneSolid` ($\times 2(B-1)$ vanes across both ports) | `feature.inlet_split.p{1,2}.v{0..B-2}` |
| `ribbon_cascade` | `vane_count > 0` and `axial_span_mm > 0` | `IrNode::HelicalRibbonCascade` | `feature.ribbon_cascade` |
| `tpms_insert` | `basis != None` (`Gyroid`, `SchwarzP`, `Diamond`, `CylindricalGyroid`) | `IrNode::TpmsPeripheralShell` | `feature.tpms_insert` |
| `chamber_modes` | $\ge 1$ harmonic mode with $\sqrt{c_m^2 + s_m^2} > 0$ | `IrNode::ChamberHarmonicMode` (sorted canonically by `(m, cos, sin, twist)`) | `feature.chamber_mode.{idx}.m{order}` |
| `cone_profile` | $|m_1| + |m_2| + |m_3| > 0$ | `IrNode::ConeSplineProfile` | `feature.cone_profile` |
| `skimmer` | `length_mm > 0` (`capture_flutes` $\in \{0, 4, 6, 8\}$) | `IrNode::VortexFinderSkimmer` | `feature.skimmer` |

---

## 2. Backend Support Matrix (`IR-02`, `IR-03`, `IR-R4`, `IR-R5`)

`DesignIrV1::check_backend_support(backend)` in Rust and `exotic_graph.check_backend_support(program, backend_mode)` in Python evaluate every active gene against the target backend before compilation. Any active mechanism with `SupportLevel::Unsupported` returns a hard `Err(IrError::UnsupportedFeature)` (or raises `UnsupportedFeatureError` in Python)—silent feature dropping is impossible.

For Wave-2 `ribbon_cascade` and `tpms_insert`, the radial wall-field backends enact a single-valued outer-wall harmonic corrugation $R(\theta, z)$ rather than free-standing internal helical vanes or a 3D periodic minimal surface lattice. Because a radial wall corrugation bounds neither enclosed volume, wetted area, nor topological genus of internal vanes/TPMS sheets, `check_backend_support` classifies active `ribbon_cascade` and `tpms_insert` as `SupportLevel::WallCorrugationProxy` (`is_supported_for_geometry_qualification() == false`, `IR-R4`), never `ConservativeApproximation`.

| Gene / Feature | `RustMultiPatchCfdSurface` | `PythonCadQueryBrep` (`brep`) | `PythonCadQueryStrictCsgOnly` (`strict_csg`) | `VoxelSdfObserver` | `RustFastRadialSurface` | `LegacyUnenactedCfdSurface` | `CudaReducedScreen` |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `body_envelope` | `Exact` | `Exact` | `Exact` | `Exact` | `ConservativeApproximation` | `ConservativeApproximation` | `ScreenOnly` |
| `terminals` | `Exact` | `Exact` | `Exact` | `Exact` | `ConservativeApproximation` | `ConservativeApproximation` | `ScreenOnly` |
| `inlet_ramp` | `Exact` | `Exact` | `Exact` | `Exact` | `ConservativeApproximation` | `ConservativeApproximation` | `ScreenOnly` |
| `inlet_split` ($B \in \{2,3,4\}$) | `Exact` | `Exact` | `Exact` | `Exact` | `ScreenOnly` | **`Unsupported`** | `ScreenOnly` |
| `ribbon_cascade` | `WallCorrugationProxy` | `WallCorrugationProxy` | **`Unsupported`** | `WallCorrugationProxy` | `WallCorrugationProxy` | `WallCorrugationProxy` | `ScreenOnly` |
| `tpms_insert` | `WallCorrugationProxy` | `WallCorrugationProxy` | **`Unsupported`** | `WallCorrugationProxy` | `WallCorrugationProxy` | `WallCorrugationProxy` | `ScreenOnly` |
| `chamber_modes` | `Exact` | `Exact` | `Exact` | `Exact` | `ConservativeApproximation` | `ConservativeApproximation` | `ScreenOnly` |
| `cone_profile` | `Exact` | `Exact` | `Exact` | `Exact` | `ConservativeApproximation` | `ConservativeApproximation` | `ScreenOnly` |
| `skimmer` | `Exact` | `Exact` | `Exact` | `Exact` | `ScreenOnly` | **`Unsupported`** | `ScreenOnly` |

### Python B-Rep vs Rust Mesh Parity (`IR-03`, `IR-R5`)
For all supported mechanisms (including Wave-2 wall corrugations and $B \in \{1,2,3,4\}$-branch `inlet_split`), [`tools/f0_voxel.py::verify_brep_rust_mesh_parity`](../tools/f0_voxel.py) verifies:
- Enclosed fluid volume agreement within $\pm 2.0\%$
- Total wetted surface area agreement within $\pm 3.0\%$
- Exact Betti number agreement $(\beta_0, \beta_1, \beta_2) = (1, 2(B - 1), 1)$ computed from the 2-manifold Euler characteristic $\chi = V - E + F = 2 - 2\beta_1$ (where each solid splitter vane spanning an inlet duct adds one topological handle of genus $1$, rather than comparing against a hardcoded `(1, 0, 1)` constant, `IR-R5`)

---

## 3. Semantic Origin Traces (`IR-11`) & Feature-Enactment Receipts (`IR-04`, `IR-05`, `IR-R1`, `IR-R2`)

1. **Unified Watertight `IrCompiledMesh` & OpenFOAM CFD STL (`IR-R1`, `IR-11`)**:
   `MultiPatchSurface::from_program` (`crates/joil-core/src/phenotype.rs`) delegates directly to `IrCompiledMesh::compile_with_extension(&ir, BackendId::RustMultiPatchCfdSurface, outlet_extension_m)` (`crates/joil-ir/src/lib.rs`). All active splitter vanes (`inlet_split`) and the vortex-finder skimmer sleeve (`skimmer`) are woven directly into the watertight 2-manifold boundary `triangles` (`internal_feature_triangles` is empty):
   - Each tangential inlet duct (`inlet1`, `inlet2`) is extruded in 3 conformal segments (`wall_idx -> inner_idx (|x|=0.024 m) -> outer_idx (|x|=0.056 m) -> port_idx (|x|=0.060 m)`), where Segment 2 (`|x| \in [0.024, 0.056]\text{ m}`) extrudes $B$ disjoint closed rectangular sub-channels on even $z$-bands and caps the solid splitter vanes on odd $z$-bands.
   - When `skimmer.length_mm > 0`, the overflow ceiling folds down the outer skimmer frustum wall (`r_outer -> r_tip` at $z = L_{\text{skim}}$) and back up the inner overflow bore (`r = r_o = 0.70\text{ mm}` to $z = 0$) before continuing into the overflow pipe (`z < 0`).
   - Every triangle in `IrCompiledMesh` and `MultiPatchSurface` carries a `triangle_origin_ids[face_idx] -> u16` index into `origin_table: Vec<SemanticOriginTrace>`. `verify_all_boundary_faces_have_origin_trace()` fails closed (`IrError::MissingSemanticOriginTrace`) if any face lacks a valid trace.
2. **Feature-Enactment Receipts & Plane-Slice Connectivity (`IR-04`, `IR-R2`)**:
   `IrCompiledMesh::verify_feature_enactment(&ir)` and `tools/f0_enactment.py::build_enactment_receipt(program)` verify that every active gene produces $>0$ attributed mesh triangles, $>0\text{ m}^2$ enacted surface area, and $>0\text{ mm}$ geometric displacement, and that `measured_disjoint_inlet_paths == expected_disjoint_inlet_paths`. Disjoint inlet paths are measured via **2D plane-slice closed-loop connectivity** (`IrCompiledMesh::count_cross_section_loops_at_x` and `tools/f0_enactment.py::_count_cross_section_loops_at_x`) by intersecting the mesh with planes $x = \pm 0.045\text{ m}$ and $x = \pm 0.050\text{ m}$ and counting connected polygon cycles where every vertex has degree $\ge 2$ (`IR-R2`). Horizontal facet counts (`|n_z| > 0.85`) are retained strictly as a secondary diagnostic (`horizontal_splitter_facet_diagnostic`).
3. **Forensic Audit of Legacy `trifurcated-6jet` CFD Surface (`IR-05`, `IR-R1`, `IR-R2`)**:
   Auditing `runs/p08-candidate-cfd-verification-v1/candidates/trifurcated-6jet-Q50/constant/triSurface/hydrocyclone.stl` with `IrCompiledMesh::audit_ascii_stl_inlet_enactment` and `tools/f0_enactment.py::audit_stl_inlet_enactment` proves that the pre-v4 CFD surface generator extruded 1 closed cross-section loop per port at $x = \pm 0.045\text{ m}$ (`measured_disjoint_inlet_paths: 2`, relabeled `actual_simulated_geometry: "2port-unsplit-wall-modulated"`), whereas the v4 `MultiPatchSurface` / `IrCompiledMesh` compiler writes a watertight 2-manifold OpenFOAM STL (`constant/triSurface/ct35.stl`) that slices to **3 closed loops per inlet duct (6 total)** at both $x = \pm 0.045\text{ m}$ and $x = \pm 0.050\text{ m}$ (`6jet-split-manifold`).

---

## 4. Geometry-Derived Taxonomy (`IR-06`) & Prior vs Observed Descriptors (`IR-07`)

- **Geometry-Derived `PhenotypeTaxon` (`IR-06`)**:
  Authored `topology_class` strings are stripped from serialized subject artifacts (`Phenotype3D`, `MultiPatchAuditReport`, `CfdCaseManifest`, `GlbSceneManifest`) via `#[serde(skip_serializing)]`. Instead, `VoxelSdfObserver::derive_taxon_from_mesh` computes `PhenotypeTaxon` purely from the compiled mesh (Betti numbers $\beta_0, \beta_1, \beta_2$, Euler characteristic $\chi$, measured disjoint inlet/outlet channel counts, FFT-like azimuthal Fourier order on chamber rings, cone radial inflection count, axial corrugation, and skimmer protrusion), including `is_novel_unclassified_cluster: true` for novel morphologies outside the 11 legacy families.
- **Strict Split of `PriorDescriptors` vs `ObservedDescriptors` (`IR-07`)**:
  [`crates/joil-core/src/inventor/mod.rs`](../crates/joil-core/src/inventor/mod.rs) and [`crates/joil-ir`](../crates/joil-ir/src/lib.rs) separate genotype-derived `PriorDescriptors` (which carry `evidence_class: PriorOnlyNotAdmissibleForQualification`) from `ObservedDescriptors` (`GeometryObservedDescriptors` measured on the compiled mesh + `Option<PhysicsObservedDescriptors>` measured from converged CFD/experiments). `verify_qualification_descriptor_gate` rejects any attempt to pass prior descriptors or missing physics observations into a `Qualified` tier evaluation.

---

## 5. Independent Voxel/SDF F0 (`IR-08`), Split Geometry Metrics (`IR-09`), CUDA Subject Binding (`IR-10`, `IR-R3`) & Negative Corpus (`IR-12`)

- **Independent Voxel/SDF F0 (`IR-08`)**:
  `VoxelSdfObserver::observe` (`crates/joil-ir/src/lib.rs`) and `tools/f0_voxel.py` perform a 3D 6-connected flood-fill to count `connected_fluid_components`, `trapped_fluid_cavities`, and `trapped_powder_cavities` (replacing hardcoded `trapped_cavities: 0`), measure `frozen_collar_and_tail_diff_mm3` directly from mesh vertices on $z \ge 166.0\text{ mm}$, and integrate terminal patch areas and normals directly from triangle geometry.
- **Split Conflated Geometry Metrics (`IR-09`)**:
  `SplitGeometryMetrics` replaces the single conflated `minimum_wall_clearance_mm` with 8 physically distinct metrics in `mm`:
  1. `minimum_shell_thickness_mm` (outer pressure-retaining cartridge shell, gate $\ge 1.0\text{ mm}$)
  2. `minimum_solid_ligament_mm` (internal splitter vanes, ribbons, skimmer wall, TPMS sheet, gate $\ge 1.0\text{ mm}$)
  3. `minimum_fluid_hydraulic_diameter_mm` ($D_h = 4A/P$ across all fluid channels, gate $\ge 1.4\text{ mm}$)
  4. `minimum_local_flow_gap_mm` (point-to-point open channel gap, gate $\ge 1.2\text{ mm}$)
  5. `minimum_clearance_to_frozen_interface_mm` (distance from active internal features to frozen collar/terminals, gate $\ge 2.0\text{ mm}$)
  6. `minimum_tool_access_diameter_mm` (overflow bore finishing access, gate $\ge 1.4\text{ mm}$)
  7. `minimum_powder_exit_diameter_mm` (LPBF powder evacuation bottleneck diameter, gate $\ge 1.4\text{ mm}$)
  8. `minimum_support_removal_access_mm` (post-build support removal access, gate $\ge 1.4\text{ mm}$)
- **CUDA Subject Bundle Reference & Workspace Counter (`IR-10`, `IR-R3`)**:
  [`CudaSubjectBundleRef`](../crates/joil-ir/src/lib.rs) binds `design_ir_sha256`, `geometry_artifact_sha256`, `geometry_artifact_bytes_len`, and `axial_radial_lut_sha256` to the exact compiled STL artifact bytes. In [`crates/joil-core/src/evolve.rs`](../crates/joil-core/src/evolve.rs), `CudaSdfAuditReport` renames the un-reduced workspace counter field to `gpu_workspace_tensor_cells_evaluated` (with `#[serde(alias = "gpu_lagrangian_parcels_integrated")]` for backward compatibility with [`physics/cuda/joil_cuda_eval.cu`](../physics/cuda/joil_cuda_eval.cu), which retains the legacy JSON key `"gpu_lagrangian_parcels_integrated"` in its stdout diagnostic line to preserve its hash-frozen source digest in `WHITEPAPER.md` and `tools/claims_audit.py`).
- **Negative Geometry Corpus & Metamorphic Suite (`IR-12`)**:
  22 malformed geometry fixtures in `tests/fixtures/negative/neg_01_*.json`..`neg_22_*.json` cover every `F0RejectionReason` variant and are verified by both Rust (`cargo test -p joil-ir`) and Python (`python3 tools/f0_voxel.py --audit-negative-corpus`), alongside metamorphic invariance tests for rigid 3D rotation, isotropic scaling ($V \propto s^3, A \propto s^2$), inactive-gene canonicalization, and chamber-mode permutation invariance.

