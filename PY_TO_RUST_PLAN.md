# Python to Rust execution plan

This is the language migration annex to docs/MASTER_PLAN.md, not a competing
research plan. Baseline: `1fa5d1b305c78e142a9b2c8ad8cb874b3967cb5c` (origin/main
at dispatch), exactly **113 tracked .py files / 26,275 physical lines**. LOC counts
newline bytes, including comments and blank lines, as `git show COMMIT:PATH | wc -l`.
The table is a frozen baseline inventory, not a claim that a port is complete.

## Ordered reads and invariant boundaries

Read AGENTS.md, docs/MASTER_PLAN.md, docs/SCIENCE.md, docs/WORK_PACKETS.json,
docs/STATUS.json, live ignored COORD.md, docs/testing.md, PYTHON_EXCEPTIONS.md,
then the table's source and its callers/tests. Claim exclusive paths before work.
No new Python. No threshold, duty, units, concentration basis, uncertainty or
scientific-admission changes. No running-job interruption. Historical receipt
bytes and hashes stay immutable. A missing Rust facility remains blocked, never
silently replaced by approximate physics. Integrator approval of shared contracts
is coordinated through the existing packet ownership mechanism.

## Migration tracker and PR sequence

Owner for every row: repository integrator (I010 worker for P1). Dates below are
review targets, not authority to skip parity or start computation. Each wave may
use multiple small PRs; each PR handles one source module plus its consumers.

| Task / PR | Target review | Scope and ordered steps | Dependency / completion |
| --- | --- | --- | --- |
| Wave P1 / P1a (I010) | 2026-10-19 | Port check_public.violations/main and check_portability control to crucible-cli binaries; compare positive/negative fixtures and actual controls; add audit/local commands | Initial Rust ports; Python compatibility remains until P1b consumer migration |
| Wave P1 / P1b | 2026-10-22 | Port tools/check.py check_steps/main receipt runner; move test_check and publication imports into Rust integration tests; route Foundation suites to Rust; remove obsolete Python after parity | P1a, byte/exit/log/source-hash parity; all eight Foundation suites |
| Wave P2 / P2a–P2c | 2026-10-29 | Evidence/strict JSON and units first; status and claim checks second; plan/explorer generators and tests third; retain exact serialization and frozen hash bindings | P1; joil-evidence; duplicate keys, NaN, signed zero, traversal and stale hashes fail closed |
| Wave P3 / P3a–P3c | 2026-11-05 | Geometry primitives, CAD/program compiler, then visualization/voxel/export callers and tests | P2; joil-geometry; independent volume, orientation, connectivity, units and frozen interfaces; no rendered validation |
| Wave P4 / P4a–P4c | 2026-11-15 | Runtime/mesh/carrier controls; oil and verification adapters; statistical protocol and KPI checks | P2/P3; integrator creates crucible-cfd only after interface review; preserve leases, timeout/restart, backflow, mass closure and uncertainty; no live simulations without lease |
| Wave P5 / P5a–P5b | 2026-11-22 | Review every kept file/hash; move Blender test/orchestration logic to Rust while retaining minimal bpy adapters; replace any historical script still used as active gate with Rust tests | P1–P4; frozen audit scripts remain archival only; verify no unlisted Python or active historical gate |

For each module: enumerate callers and output schema; capture deterministic
baseline and adversarial fixtures; implement Rust without a Python subprocess;
run both against identical inputs; compare values/bytes, exit codes and failure
semantics; migrate consumers; only then delete Python and update current counts.
Use the existing target/ and bounded jobs. `just fast`, `just portability` and
`just required` are entry points. Run the four canonical checks requested by I010:
`python3 tools/check_public.py`, `python3 tools/v5_plan.py check`,
`python3 tools/v5_explorer_manifest.py check`, `python3 tools/claims_audit.py`.
During migration, these are compatibility checks, not permission for new Python.

Stop on scope conflicts, mismatched hashes/units, unresolved numerical behavior,
parity failures, missing dependencies or resource pressure. Handoff: exact commit,
paths, commands/exits, log SHA-256, positive/negative results, remaining Python
count, limitations and next dependency. No physical validation is implied.

## Complete baseline inventory

Targets name the owning Rust crate; crucible-cfd does not yet exist and is a
planned P4 dependency. Keep-with-reason rows are explained individually in
PYTHON_EXCEPTIONS.md. Test rows migrate alongside their implementation.

| Path | LOC | Purpose | Disposition | Target Rust crate/binary | Wave |
| --- | ---: | --- | --- | --- | --- |
| `analysis/__init__.py` | 1 | Joil evidence extraction; no validation is inferred from a run completing. | port-to-Rust | `joil-evidence` | Wave P2 |
| `analysis/baseline_report.py` | 94 | Publish small measured hydraulic observations, with qualification and fitness explicit. | port-to-Rust | `joil-evidence` | Wave P2 |
| `analysis/carrier_snapshot.py` | 119 | Bind same-iteration observations from a running carrier to immutable raw bytes. | port-to-Rust | `crucible-cfd (planned crate)` | Wave P4 |
| `analysis/case_identity.py` | 196 | Wave-4 exact artifact chain. Capture after meshing, before inputs are pruned. | port-to-Rust | `crucible-cfd (planned crate)` | Wave P4 |
| `analysis/evidence.py` | 312 | Exact-byte identities, immutable manifests and conservative research fitness. | port-to-Rust | `joil-evidence` | Wave P2 |
| `analysis/inventory.py` | 24 | Oil cohort accounting with unresolved mass assigned adversarially. | port-to-Rust | `joil-evidence` | Wave P2 |
| `analysis/kpi_fitness.py` | 771 | Source-bound KPI, 10-bin light-droplet grade curve, and multi-objective fitness evaluator. | port-to-Rust | `crucible-stats` | Wave P4 |
| `analysis/units.py` | 15 | Dimension-checked OpenFOAM pressure conversion; reject ambiguous p_rgh. | port-to-Rust | `joil-evidence` | Wave P2 |
| `analysis/validation/v5/__init__.py` | 26 | Deterministic, fixture-grade tools for prospective Joil confirmation. | port-to-Rust | `crucible-stats` | Wave P4 |
| `analysis/validation/v5/protocol.py` | 655 | Strict fixture-grade physical confirmation analysis. | port-to-Rust | `crucible-stats` | Wave P4 |
| `analysis/validation/v5/tests/test_protocol.py` | 233 | Regression tests for test protocol | port-to-Rust | `crucible-stats` | Wave P4 |
| `cad/ct35/build.py` | 149 | Build/audit exact CT-35 research CAD. No CFD-ready boundary mesh is claimed. | port-to-Rust | `joil-geometry` | Wave P3 |
| `cad/ct35/geometry.py` | 48 | Independent Python derivation of the public CT-35 reconstruction, mm. | port-to-Rust | `joil-geometry` | Wave P3 |
| `cad/program/build_proposals.py` | 58 | Compile/audit proposal CAD and export metre scenes. No flow or strength claims. | port-to-Rust | `joil-geometry` | Wave P3 |
| `cad/program/exotic_graph.py` | 780 | Compositional Exotic Design Program (ct35-exotic-head-v2 / v3) & B-Rep Compiler (IR-02, IR-03). | port-to-Rust | `joil-geometry` | Wave P3 |
| `cad/program/propose.py` | 38 | Deterministic mutation proposals. Geometry diversity only; no CFD performance selection. | port-to-Rust | `joil-geometry` | Wave P3 |
| `cad/program/wall_field.py` | 76 | Versioned smooth non-axisymmetric chamber programme with frozen CT-35 interfaces. | port-to-Rust | `joil-geometry` | Wave P3 |
| `crates/joil-core/tests/truth_ev12_capture.py` | 100 | Retain finalized EV-12 case inputs before pruning; never run or delete a case. | port-to-Rust | `joil-evidence` | Wave P2 |
| `crates/joil-core/tests/truth_ev12_capture_test.py` | 129 | Synthetic byte-retention tests, not an OpenFOAM or physical validation run. | port-to-Rust | `joil-evidence` | Wave P2 |
| `crates/joil-evidence/receipts/ev12_disposition_audit.py` | 194 | Account for every retained Wave-4 case without promoting incomplete identities. | keep-with-reason | `joil-evidence` | Wave P5 |
| `crates/joil-evidence/receipts/ev12_every_run_audit.py` | 107 | Read every Wave-4 case, including missing manifests and quarantined cases. | keep-with-reason | `joil-evidence` | Wave P5 |
| `crates/joil-evidence/receipts/ev12_writer_review.py` | 179 | Independent executable probes of the IR EV12 writer; all mutations are synthetic. | keep-with-reason | `joil-evidence` | Wave P5 |
| `crates/joil-evidence/receipts/ev13_figure_trace.py` | 163 | Read-only proof-figure audit; renders only into an EV-private directory. | keep-with-reason | `joil-evidence` | Wave P5 |
| `crates/joil-evidence/receipts/ev13_figure_values.py` | 544 | Independent receipt projections for the EV13 captured plotting calls. | keep-with-reason | `joil-evidence` | Wave P5 |
| `crates/joil-evidence/receipts/ev13_followup_reverify.py` | 195 | Reverify the EV13 follow-up from exported git snapshots and plot captures. | keep-with-reason | `joil-evidence` | Wave P5 |
| `crates/joil-evidence/receipts/ev_final_caption_audit.py` | 116 | Read-only audit of the lead/hero captions at an explicit committed revision. | keep-with-reason | `joil-evidence` | Wave P5 |
| `crates/joil-evidence/receipts/plate10_field_array_review.py` | 144 | Verify committed Plate 10 arrays against the prior direct plotting capture. | keep-with-reason | `joil-evidence` | Wave P5 |
| `crates/joil-evidence/receipts/plate10_field_arrays.py` | 226 | Capture legacy Plate 10 grids/profiles from retained raw OpenFOAM fields. | keep-with-reason | `joil-evidence` | Wave P5 |
| `crates/joil-evidence/receipts/wave4_gci_trace.py` | 253 | Read-only, per-numeric-leaf audit of Wave-4 GCI inputs and derived arithmetic. | keep-with-reason | `joil-evidence` | Wave P5 |
| `crates/joil-evidence/receipts/wave4_number_trace.py` | 180 | Independent arithmetic audit of the committed Wave-4 receipt; no Joil imports. | keep-with-reason | `joil-evidence` | Wave P5 |
| `crates/joil-evidence/receipts/wave4_raw_proof.py` | 430 | Recompute Wave-4 from retained candidate/population and prediction records. | keep-with-reason | `joil-evidence` | Wave P5 |
| `crates/joil-evidence/receipts/wave4_raw_proof_tests.py` | 219 | Functional oracles and captured ten-seed arithmetic regressions. | keep-with-reason | `joil-evidence` | Wave P5 |
| `crates/joil-evidence/tests/test_python_contract.py` | 90 | Synthetic mutations of frozen contracts, never physical validation. | port-to-Rust | `joil-evidence` | Wave P2 |
| `docs/receipts/v5-00-checks.py` | 78 | Source-bound baseline checks; no simulation or scientific admission. | keep-with-reason | `joil-evidence` | Wave P5 |
| `docs/receipts/v5-00-python-probes.py` | 101 | Synthetic, bounded probes of baseline Python functions and frozen dimensions. | keep-with-reason | `joil-evidence` | Wave P5 |
| `docs/receipts/v5-g012-frozen-wave6-control-driver.py` | 784 | Executed red/positive controls for the 16 source-only V5-00 baseline findings (grant C006). | keep-with-reason | `joil-evidence` | Wave P5 |
| `physics/openfoam/carrier/configure.py` | 115 | Foundation 14 carrier dictionaries. Research initializer, not accepted F2. | port-to-Rust | `crucible-cfd (planned crate)` | Wave P4 |
| `physics/openfoam/carrier/point.py` | 79 | Prepare a content-bound operating point; failed mesh needs explicit diagnostic mode. | port-to-Rust | `crucible-cfd (planned crate)` | Wave P4 |
| `physics/openfoam/environment/runtime.py` | 101 | Restricted, project-scoped OpenFOAM invocation. Never uses a mutable image tag. | port-to-Rust | `crucible-cfd (planned crate)` | Wave P4 |
| `physics/openfoam/mesh/audit.py` | 126 | Independent ASCII polyMesh terminal audit, separate from CAD and snappy. | port-to-Rust | `crucible-cfd (planned crate)` | Wave P4 |
| `physics/openfoam/mesh/extend_outlet.py` | 87 | Append a declared numerical outlet pipe; never mutate the baseline genotype. | port-to-Rust | `crucible-cfd (planned crate)` | Wave P4 |
| `physics/openfoam/mesh/gmsh_mesh.py` | 108 | Alternative exact-STEP tetrahedral mesh, independently checked in OpenFOAM. | port-to-Rust | `crucible-cfd (planned crate)` | Wave P4 |
| `physics/openfoam/mesh/prepare.py` | 131 | Make a labelled closed SI surface and a bounded full-liner snappy pilot case. | port-to-Rust | `crucible-cfd (planned crate)` | Wave P4 |
| `physics/openfoam/mesh/reference_plane.py` | 52 | Recover exact original outlet faces after extrusion, without a planar approximation. | port-to-Rust | `crucible-cfd (planned crate)` | Wave P4 |
| `physics/openfoam/oil/oil_grade.py` | 1201 | Frozen-carrier 10-bin monodisperse light-droplet grade efficiency & inventory tracker (OpenFOAM 14). | port-to-Rust | `crucible-cfd (planned crate)` | Wave P4 |
| `physics/openfoam/verification/droplet.py` | 94 | Native Foundation 14 liquid-density droplet terminal-rise/response oracle. | port-to-Rust | `crucible-cfd (planned crate)` | Wave P4 |
| `physics/openfoam/verification/poiseuille.py` | 79 | Executed laminar plane-channel oracle, with independent analytical solution. | port-to-Rust | `crucible-cfd (planned crate)` | Wave P4 |
| `physics/openfoam/verification/rotation.py` | 276 | OIL-03: Native Foundation 14 light-droplet inward migration in solid-body rotation. | port-to-Rust | `crucible-cfd (planned crate)` | Wave P4 |
| `render/blender/plate.py` | 161 | Manifest-first Blender probe and restartable plate coordinator (stdlib only). | port-to-Rust | `crucible-cli` | Wave P5 |
| `render/blender/render_job.py` | 62 | Minimal Blender side of the adapter. Run only through plate.py after semantic checks. | keep-with-reason | `crucible-cli` | Wave P5 |
| `render/blender/test_plate.py` | 104 | Regression tests for test plate | port-to-Rust | `crucible-cli` | Wave P5 |
| `schemas/v5/check_portability.py` | 15 | Independent Linux control: architecture/interchange tests with rtk absent. | port-to-Rust | `crucible-cli` | Wave P1 |
| `schemas/v5/generate_contracts.py` | 174 | Freeze structural interchange and generate TS. Rust enforces semantics/reviews. | port-to-Rust | `joil-evidence` | Wave P2 |
| `schemas/v5/validate_fixtures.py` | 19 | Independent JSON Schema structural check (requires available jsonschema). | port-to-Rust | `joil-evidence` | Wave P2 |
| `tests/test_baseline_report.py` | 22 | Regression tests for test baseline report | port-to-Rust | `joil-evidence` | Wave P2 |
| `tests/test_carrier_snapshot.py` | 35 | Synthetic adversarial streams test extraction, never CFD performance. | port-to-Rust | `crucible-cfd (planned crate)` | Wave P4 |
| `tests/test_check.py` | 107 | Check selection and receipts must preserve coverage and fail closed. | port-to-Rust | `crucible-cli` | Wave P1 |
| `tests/test_design_program.py` | 28 | Regression tests for test design program | port-to-Rust | `joil-geometry` | Wave P3 |
| `tests/test_doc_genome.py` | 99 | DOC-06: docs/GENOME.md must describe the genome the code implements. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tests/test_doc_numbers_trace.py` | 143 | DOC-04: every numeral in README and WHITEPAPER "Read this first", Abstract and §5 traces to a receipt or a checked declaration. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tests/test_doc_page1_figure.py` | 45 | PAGE1-WHY-MATTERS: the WHITEPAPER lead figure is generated from the Wave-4 receipt and stays bound to it. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tests/test_doc_traceability.py` | 165 | DOC-01/DOC-02: review inputs stay verbatim and every review P0 traces to a workplan item. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tests/test_doc_wave4_gci_figure.py` | 41 | IR NEED-DOC-06: the WHITEPAPER §4.6 figure is generated from committed CFD receipts and stays bound to them. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tests/test_doc_wording.py` | 54 | DOC-08: strong evidence words need a tier or limitation; descriptor-backed claims need a prior label. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tests/test_ev13_generator_fixes.py` | 184 | EV-13: receipt-dependent narratives and honest numerical-policy labels. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tests/test_evidence.py` | 617 | Regression tests for test evidence | port-to-Rust | `joil-evidence` | Wave P2 |
| `tests/test_foundation.py` | 43 | Regression tests for test foundation | port-to-Rust | `joil-evidence` | Wave P2 |
| `tests/test_mesh_section.py` | 23 | Regression tests for test mesh section | port-to-Rust | `joil-geometry` | Wave P3 |
| `tests/test_rust_cuda_and_hardening.py` | 86 | Regression tests for hardened KPI provenance, Exotic CAD canonicalization, and Lagrangian parsers. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tests/test_v5_baseline_audit.py` | 114 | Independent audit mutations: inventory is never physical or unexecuted PASS. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tests/test_v5_baseline_source_controls.py` | 368 | C006: executed red/positive controls for the 16 source-only V5-00 findings on frozen e1e9203 components. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tests/test_v5_defect_closure.py` | 128 | Receipt-gate boundary tests; synthetic receipt metadata never validates physics. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tests/test_v5_explorer_manifest.py` | 49 | Independent software metadata and stale-source controls; no physical claims. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tests/test_v5_integrated_fixtures.py` | 8 | Keep imported protocol and render-boundary controls in ordinary software CI. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tests/test_v5_plan.py` | 206 | Plan integrity boundaries; no physical qualification is asserted. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tests/test_v5_private_dossier.py` | 44 | Regression tests for test v5 private dossier | port-to-Rust | `joil-evidence` | Wave P2 |
| `tests/test_v5_release_scope.py` | 56 | Release-scope and parity checks for the V5-31 software handoff. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tests/test_v5_trace_boundary.py` | 40 | Operational task receipts must not become uncited scientific provenance. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tests/test_v5_truth.py` | 646 | V5-01 truth repairs: production-path regressions for corrected claim semantics. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tools/audit_supplied.py` | 50 | Independently audit all supplied STL subjects and duplicate archives. Private outputs. | port-to-Rust | `joil-geometry` | Wave P3 |
| `tools/bench_cfd.py` | 71 | Execute bounded mesh or carrier stages; preserve every log and return code. | port-to-Rust | `crucible-cfd (planned crate)` | Wave P4 |
| `tools/check.py` | 95 | Run foundation checks and archive exact logs/receipts in ignored private storage. | port-to-Rust | `crucible-cli` | Wave P1 |
| `tools/check_public.py` | 44 | Check Git-selected/public candidates; optional private deny rules never enter Git. | port-to-Rust | `crucible-cli` | Wave P1 |
| `tools/claims_audit.py` | 1700 | Claims-to-receipts audit for docs/WHITEPAPER.md, docs/SCIENCE.md, README.md and docs/RESULTS.md. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tools/f0_enactment.py` | 376 | Feature-Enactment Receipt Verifier & Legacy 6-Jet Forensic Auditor (IR-04, IR-05). | port-to-Rust | `joil-geometry` | Wave P3 |
| `tools/f0_voxel.py` | 602 | Independent Voxel/SDF F0 Observer, B-Rep/Rust Mesh Parity Verifier, and Negative Corpus Auditor (IR-03, IR-08, IR-09, IR-12). | port-to-Rust | `joil-geometry` | Wave P3 |
| `tools/gen_status.py` | 150 | Generate conservative public status from the byte-pinned evidence inventory. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tools/generate_gci_rsm_report_and_figures.py` | 564 | Compute P04/P05 3-grid Roache GCI and RSM (LRR/SSG) cross-check dossier + publication figure plate. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tools/generate_openfoam14_flowfield_plate.py` | 278 | Extract 3D OpenFOAM 14 cell-center fields (r, z, U_theta, U_z, p, k) and render vortex physics comparison plate. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tools/generate_page1_figure.py` | 168 | WHITEPAPER lead figure (owner request PAGE1-WHY-MATTERS, workplan DOC-10). | port-to-Rust | `joil-evidence` | Wave P2 |
| `tools/generate_wave2_figures_and_animations.py` | 1417 | Generate Wave-2 publication figure plates (Plates 5-8) and animated GIF + manifest sidecars. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tools/generate_wave2_vs_wave1_cfd_plate.py` | 436 | Generate Plate 10: Side-by-Side Wave-1 vs. Wave-2 Elite OpenFOAM 14 3D CFD Field Comparison | port-to-Rust | `joil-evidence` | Wave P2 |
| `tools/generate_wave3_proof_figures.py` | 796 | Generate the Wave-3 README Hero Plot and White-Paper Evolutionary Proof Plate (Plate 11) | port-to-Rust | `joil-evidence` | Wave P2 |
| `tools/generate_wave4_gci_figure.py` | 168 | WHITEPAPER §4.6 figure (IR NEED-DOC-06, workplan DOC-10): same-pipeline CT-35 control vs archived shells and | port-to-Rust | `joil-evidence` | Wave P2 |
| `tools/render_evolution_atlas.py` | 521 | Render the 3D Topology & Droplet-Vortex Atlas, Pareto/MAP-Elites Convergence Plate, | port-to-Rust | `joil-evidence` | Wave P2 |
| `tools/run_elite_3grid_gci_and_rsm.py` | 824 | Mandatory 3-Grid (G0/G1/G2) + Dual 7-Eq. LRR/SSG RSM elite CFD admission gate. | port-to-Rust | `crucible-cfd (planned crate)` | Wave P4 |
| `tools/run_wave2_elites_openfoam14.py` | 707 | Execute 5-case OpenFOAM 14 (G0/G1/G2 Roache GCI + Dual LRR/SSG RSM) verification | port-to-Rust | `crucible-cfd (planned crate)` | Wave P4 |
| `tools/run_wave4_gci_sweep.py` | 443 | INT-06 & G-09: Wave-4 OpenFOAM 14 3-Grid (G0/G1/G2) Spatial Diagnostic Sweep | port-to-Rust | `crucible-cfd (planned crate)` | Wave P4 |
| `tools/run_wave4_smoke.py` | 307 | INT-03: Wave-4 smoke campaign after TR-01..04 + TR-13 to verify zero silent baseline fallbacks | port-to-Rust | `crucible-cfd (planned crate)` | Wave P4 |
| `tools/v5_baseline_audit.py` | 120 | Audit frozen baseline inventory; this never qualifies a physical claim. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tools/v5_baseline_source_controls.py` | 840 | Executed red/positive controls for the 16 source-only V5-00 baseline findings (grant C006). | port-to-Rust | `joil-evidence` | Wave P2 |
| `tools/v5_d18_scheduler_control.py` | 188 | Run bounded controls against the frozen D-18 joil-core library. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tools/v5_d21_trajectory_control.py` | 172 | Runtime controls for the D-21 presentation and trajectory claim boundary. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tools/v5_d28_ownership_control.py` | 68 | Source-bound software control for D-28 coordination ownership policy. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tools/v5_d29_blind_judge_control.py` | 102 | Exercise D-29 frozen source bindings and blind-judge dependency boundary. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tools/v5_explorer_manifest.py` | 88 | Generate software-progress metadata from canonical records; no scientific observations. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tools/v5_fixture_smoke.py` | 137 | Record a bounded fresh-clone witness for the shipped synthetic fixture. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tools/v5_plan.py` | 370 | Build/check the planning views; progress is never scientific qualification. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tools/v5_private_dossier.py` | 52 | Software-only private dossier export gate; this does not grant installation authority. | port-to-Rust | `joil-evidence` | Wave P2 |
| `tools/verify_environment.py` | 53 | Check the exact local CPU image and archive native capability/version receipts. | port-to-Rust | `crucible-cfd (planned crate)` | Wave P4 |
| `visualization/render_baseline.py` | 66 | Render actual baseline B-rep geometry, with no synthetic physics fields. | port-to-Rust | `joil-geometry` | Wave P3 |
| `visualization/render_carrier.py` | 83 | Render actual saved OpenFOAM p/U fields; no invented flow or averaging. | port-to-Rust | `joil-geometry` | Wave P3 |
| `visualization/render_proposals.py` | 57 | Plot sections through the actual exported CAD surfaces, with no performance coloring. | port-to-Rust | `joil-geometry` | Wave P3 |

## I012 additive port handoff

`crates/crucible-cli/src/bin/check-foundation.rs` ports `tools/check.py` suite
selection and execution, schema-v1 receipts, raw stdout-then-stderr log SHA-256,
Git-selected source hashes, child failures/signals and the 600-second timeout.
Run from the repository root. `foundation_parity.rs` compares all eight suites
on synthetic success/failure commands, receipt fields and bound log bytes;
volatile run IDs and durations are excluded from equality. Invalid suite/flag
combinations must retain exit 2. Infrastructure failures fail closed; their
platform-specific diagnostic text is not a byte-parity promise.

`crates/crucible-cli/src/bin/v5-private-dossier.rs PRIVATE.json REVIEW.json`
ports the private dossier projection gate. `dossier_parity.rs` invokes the
unchanged Python function as oracle for a positive projection and nine negative
classification, authorization, review, allowlist, type and identity-token cases.
This is a software disclosure gate, not installation authority.

Both original Python files remain intact. Shared local/CI shell entrypoints still
use the compatibility runner because publication/claims/UI CI lanes do not yet
provision Rust. P1b remains open for that provisioning/consumer migration, CAD,
timeout/platform diagnostics and broader Unicode edge parity. No Python file or
LOC reduction is claimed by adding these binaries. P2 plan/explorer ports remain
separate work; this packet chose the authorized dossier alternative.

## I013 additive ports

`v5-d28-ownership-control` ports the frozen-source ownership control;
`v5-explorer-manifest` ports build/check and its native `v5_plan.load` validation
dependency (`crates/crucible-cli/src/plan_validation.rs`). No Python subprocess
is used by either production binary. The explorer defaults to the source checkout;
`--root ROOT` supports isolated replay against another canonical record tree.

Parity integration tests compare D28 positive/red controls, missing frozen commit
and digest mismatch exits/stdout/stderr; explorer tests compare generated manifest
and snapshot bytes, successful check output, stale outputs, symlinks, immutable
collisions, schema/receipt/hash/defect/dependency controls and malformed JSON.
Syntax-error diagnostics remain parser-specific; both implementations reject them.
CLI usage diagnostics use native binary names. Python files and legacy callers
remain until P1b/P2 consumer migration. No scientific observations are generated.

Run `cargo test --locked -p crucible-cli --test d28_ownership_parity --test explorer_manifest_parity`
and `cargo test --locked -p crucible-cli --bin v5-d28-ownership-control`.
Exact results and retained limitations are in `docs/receipts/i013-rust-ci-20261018.json`.

## I014 additive ports

`v5-plan` ports build/check planning views and update/rollback. It reuses the native
`plan_validation.rs` gates and preserves insertion order for the embedded dashboard,
Python's sorted snapshot hash, and all 34 generated view bytes. `--root ROOT` selects
an isolated canonical tree for replay. Progress completion remains distinct from
scientific qualification. The Python script remains available to legacy callers.

`v5-d29-blind-judge-control --mode positive|red` verifies both frozen source hashes
before resolving the locked offline all-features Cargo graph. It preserves sorted
transitive traversal and the synthetic two-edge red control. `--root ROOT` selects
a replay checkout; the default is the source checkout, as in Python.

`plan_parity` tests generated bytes, semantic registry/progress/receipt/defect
failures, malformed inputs, template markers, update bytes and rollback.
`d29_blind_judge_parity` tests real positive/red controls, missing frozen commit,
digest mismatch, direct/indirect forbidden paths, cycles, missing judge/package IDs
and Cargo failure. Semantic controls compare exact stdout/stderr/exit; malformed
JSON and OS diagnostics remain language-specific, and CLI usage names are native.
Original Python files and consumer entrypoints remain unchanged. No Python LOC
reduction or scientific admission is claimed. Exact check results are recorded in
`docs/receipts/i014-rust-ci-20261018.json`.

## I015 additive ports

`v5-baseline-audit [--source-check]` audits the 31 frozen baseline allegations,
15 recorded probes and 12 retained deferrals. Optional historical Git reads
verify source digests and anchors. `gen-status [--check]` imports byte-pinned
software receipts, validates the requirement/preregistration/objective contract
and preserves conservative public STATUS bytes and scientific hash identity.
Both accept `--root ROOT` for isolated replay and default to this source checkout.
Production binaries invoke no Python. Python files and legacy callers remain.

`baseline_audit_parity` and `gen_status_parity` compare unchanged Python oracles,
output bytes/exits, status SHA-256 and frozen-source checks, plus tampered/missing
inventory, stale/malformed evidence and attempted promotion controls. Error text
for syntax, missing keys/files and Git failures remains implementation-specific;
semantic status errors retain exact parity. Baseline Python CLI tracebacks remain visible; parity asserts that they contain
the Rust failure message after stripping its `FAIL:` prefix. No Python LOC
reduction, consumer migration or physical qualification is claimed.

The insertion-order codec is shared internally in `ordered_json.rs` with
`v5-plan`; its extraction preserves existing plan wire behavior and is checked
by `plan_parity` as well as the full Foundation gate.

## I016 additive ports

`v5-d21-trajectory-control --mode positive|red [--root ROOT]` binds the two
frozen source digests, D-21 inventory and renderer contract. It runs the real
locked offline renderer regression. I017 removes the test bypass; isolated
negative controls reject source/contract tampering before renderer execution.
Native manifest validation preserves the subject/time/field and scope boundary.

`v5-d18-scheduler-control positive|red --scratch DIR --frozen-rust DIR
[--root ROOT]` checks the frozen scheduler hash/anchors, compiles the original
probe against the supplied frozen libraries and checks evaluator records.
The red evaluator uses `/bin/sh`, `date +%s%N` and `sleep 0.001`; production
Rust controls invoke no Python. Wall-clock timestamps must be strictly increasing;
a clock step backwards fails closed. The supplied library build provenance remains
the caller's responsibility, matching the original control.

D18 parity builds the pinned Git sources in the lane target cache under a separate
frozen profile. Positive/red outputs and exits match Python, as do missing-library,
invalid-log and source-tamper rejections. D21 parity compares isolated contract
mutations exactly and runs the actual renderer separately; Cargo's elapsed-time
and build diagnostics are variable, while the control output is exact. Native
usage, JSON parsing, OS and compiler/Git setup diagnostics can differ by language.
Python scripts and callers remain for the later consumer migration. These are
ports 11–12, not physical validation or resolution of the frozen defects.

I016 uses the user-authorized targeted local gate set with two build jobs/test
threads; the full Foundation matrix remains a PR CI dependency. Exact evidence
is in `docs/receipts/i016-rust-ci-20261018.json`.

## I017 additive ports (13–14)

`verify-environment --case PATH [--root ROOT] [--doctor-source PATH]` verifies
the locked recipe/key, native image identity and four restricted Docker probes,
then writes capability and command receipts. The source identity defaults to the
Python doctor, preserving the original evidence contract. Runtime source identity
continues to bind `runtime.py`; this is compatibility metadata, not the Rust
implementation digest (recorded separately in the I017 receipt). The Docker
invocation, resource lease checks, timeout and shared output budget are retained.

`v5-fixture-smoke --checkout PATH --evidence PATH --expected-commit SHA`
checks clean source/origin/independent Git objects, private evidence/cache and
physical core availability, then executes whole/partial/resumed/offline/status,
one-worker and invalid-input controls. It preserves the ordered source, worker,
report, command and inventory formats. Both production ports invoke native
programs directly, with no Python subprocess. Python sources and callers remain.

Parity uses synthetic Docker/Cargo executables, explicitly not solver or campaign
validation. Environment tests compare stable receipt fields and exact log bytes,
excluding measured wall time and observed output bytes. Fixture tests compare
source/capability/report bytes exactly at the same paths, verify every raw
inventory hash, then compare inventory after normalizing command wall times and
its bound hash/size; summary comparison normalizes only the resulting inventory
hash. Independent measured timings preclude unconditional raw inventory/stdout
byte equality. Native parser/OS diagnostics and non-ASCII JSON escaping are not
byte-parity promises. Full Foundation remains CI work; local verification uses
the user-authorized targeted gates, recorded in the I017 receipt.
