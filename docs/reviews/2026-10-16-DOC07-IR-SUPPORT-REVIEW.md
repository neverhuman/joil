# DOC-07 review: `docs/IR_SUPPORT.md` against the merged code

Reviewer: DOC lane (`joil_paper`), 2026-10-16. Code reviewed: `lane/paper` at `1229495`, which merges
`feat/joil-v4-truth-evidence` at `0139ca0` (IR-01..IR-12 merged at `2f15710`). `docs/IR_SUPPORT.md` is owned
by the IR lane; this review records what the code does and asks IR/INT for fixes. It does not edit their files.

Evidence: ignored `.private/checks/doc07-ir-review-20261016T062959Z/` (DOC lane worktree), not committed.

| Item | SHA-256 |
|---|---|
| CFD STL written by `MultiPatchSurface::from_program` for the Wave-2 `trifurcated-6jet-vortex-head` program | `a66e2b76cc74d8f7f3f7f4a42c1a18cd973cde3aa8a96a14d6cd4bf55ae01ae0` |
| Program (from `docs/results/wave2-multiseed-ablation-and-holdout-v1.json`, `branches_per_port = 3`) | `b4d3ad619a1875c42ace4d70ff0073d73ed1615089a469836297c85a34a32ec2` |
| Cross-section slice results | `3ae906c4eda80eedd35ebdf257659c811f6ce9f65041d114b343398f4af6036f` |
| `tools/f0_enactment.py --program … --stl …` output | `d6482d166e3380ead4fd2407aa3b87e2f5ce0d9bc35751b7e37667a1621aacfa` |

## Findings

| # | Severity | Finding | Evidence | Requested fix |
|---|---|---|---|---|
| IR-R1 | **High** | The OpenFOAM writer still emits two undivided inlet ducts for a split program, while the case manifest and audit attribute the geometry to `IrCompiledMesh`. In `crates/joil-core/src/cfd_case.rs:159`, `build_openfoam_candidate_case` writes `MultiPatchSurface::from_program`; that function builds its own ring surface and never reads `inlet_split`, `inlet_ramp` or `skimmer`. It then compiles `IrCompiledMesh` separately (`crates/joil-core/src/phenotype.rs:1054`) only to attach an enactment receipt and voxel F0 report to the audit. The manifest names `joil-ir::IrCompiledMesh` as `geometry_kernel` (`cfd_case.rs:492`). Enactment is therefore verified on a mesh that is not the CFD subject, and IR-02's hard-error rule never applies to the CFD writer. | Slicing the written STL at x = ±0.045 m and ±0.050 m (inside each inlet duct) gives **one** closed cross-section loop per duct (`slices.txt`); a 3-branch split would give three. `IrCompiledMesh`'s own enactment receipt reports 6 paths for the same program. | IR: write the `IrCompiledMesh` triangles that were verified into the OpenFOAM STL, or declare the writer's backend as `LegacyUnenactedCfdSurface`, which the matrix marks `Unsupported` for `inlet_split` and `skimmer`, and let it hard-fail. INT: hold INT-06 CFD runs until this lands, or the runs repeat correction C1. |
| IR-R2 | High | `tools/f0_enactment.py::audit_stl_inlet_enactment` counts horizontal facets at interior heights as "splitter vanes" and adds one path per facet height. It does not compute connectivity, so it does not measure disjoint paths. | On the STL above it reports `measured_disjoint_inlet_paths = 12` and `internal_splitter_vane_triangles = 18`, while the slice test shows one channel per duct. | IR: count closed loops of duct cross-sections (or connected fluid components behind each inlet) and keep the facet count only as a diagnostic. Re-check the IR-05 relabel numbers in `docs/IR_SUPPORT.md` §3 with the corrected tool. |
| IR-R3 | Medium | `docs/IR_SUPPORT.md` §5 says the CUDA counter was renamed `gpu_workspace_tensor_cells_evaluated`. The code still emits and stores `gpu_lagrangian_parcels_integrated`. | `physics/cuda/joil_cuda_eval.cu:553`; `crates/joil-core/src/evolve.rs:154`; `grep` finds no `gpu_workspace_tensor_cells_evaluated` in `crates/`, `physics/`, `tools/` or `analysis/`. | IR: complete the rename (IR-10), or correct the document. |
| IR-R4 | Medium | The matrix marks `ribbon_cascade` and `tpms_insert` as `ConservativeApproximation` on the CFD, B-rep and voxel backends. Those backends produce wall-radius corrugation, not free-standing vanes or a lattice. A corrugation bounds neither volume, wetted area nor topology of the intended feature, so it is not a conservative approximation in S5 §7.4's sense (`ConservativeApproximation { bound }`). | `phenotype.rs` `inner_radius_mm` (ribbon lobes and TPMS corrugation); `docs/GENOME.md` §2. | IR: label these `VisualizationOnly`/proxy, or `Unsupported` for qualification, until resolved geometry or a calibrated porous model exists. |
| IR-R5 | Low | IR-03 parity requires Betti numbers exactly `(1, 0, 1)` for every design. A resolved TPMS lattice or attached vanes would change β₁/β₂, so the check confirms that no internal topology exists rather than measuring topology. | `docs/IR_SUPPORT.md` §2. | IR: compare B-rep and mesh Betti numbers with each other, not with a constant. |
| IR-R6 | Note | IR-08 works as described for the phenotype: `trapped_cavities` now comes from the voxel observer's report rather than a constant. | `crates/joil-core/src/phenotype.rs:560` | none |
| IR-R7 | Note (TR/INT) | `joil-cli prepare-cfd-case --wave2-family …` can no longer read the committed Wave-2 receipt (`missing field genome_sha256`), so historical elites cannot be re-cased without a migration reader. | `artifacts/cfd-enactment-check.log` (same check directory) | TR/INT: add a versioned legacy reader, or document that historical receipts are not re-loadable. |

## Effect on the documentation

- WHITEPAPER correction C1 stays in force for every executed CFD run, and for any new run produced through
  `cfd_case.rs` until IR-R1 is fixed.
- `docs/GENOME.md` §2's "CFD surface" column describes the writer's actual output (`MultiPatchSurface`) and stays
  correct. Its pending tags for IR-02/IR-05 remain until IR-R1 lands.
