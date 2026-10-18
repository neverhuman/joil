# Source reconciliation and engineering decisions

Reviewed 17 October 2026 against `neverhuman/joil@cc86edc90d06f0eac69022f84354abc94bb453c4`. This review uses all ten files in the two requested directories. Exact input identities, text lengths and Markdown section indexes are in `SOURCE_MANIFEST.json`. Source inputs are private review material; their filename is not a normative priority rule. Source self-scores are editorial opinions, not engineering evidence.

## Input roles

| ID | Input | Useful contribution | Qualification or implementation correction |
| --- | --- | --- | --- |
| S01 | `final/JOIL v5 Engineering Specification.md` | Detailed Crucible/MPG direction, vortex-first concern, HPC, Blender plates, objectives and migration | Treat GPU timing as planning arithmetic; investigate PDR causation and use mature reference first |
| S02 | `final/JOIL_Final_Engineering_Specification.md` | Deep architecture/evidence review, independent observables, DecisionProfile, simultaneous-confidence fixture, current visualization catalogue | Strong source for inference semantics; its examples remain synthetic and thresholds provisional |
| S03 | `final/JOIL_Final_Engineering_Specification_Synthesis.md` | Concrete deterministic `reference-search/1`, complete attempt accounting, revocation, actual-cost versus logical replay distinction, staged native adapters | Preferred initial search/runtime policy over an immediate adaptive bandit/learned-archive stack |
| S04 | `final/JOIL_v5_Final_Engineering_Specification.md` | Concise product synthesis and blind-judge/open-genome direction | Compressed ladder differs from other drafts; do not infer implementation or validation from its “final” label |
| S05 | `final/Joil_Engineering_Specification.pdf` | 38-page native-adapter-first specification, typed contracts, material/process and independent artifact observation, all-test visual plan | PDF text extracted and representative contract page visually checked; proposed APIs remain future work |
| S06 | `final/joil v5 Engineering Spec — Open-Ended Evolutionary Design with Hard Engineering Gates.md` | Five-layer genome separation, backend coverage, measured topology, experiment controls, code-retirement guidance | Keep fail-closed evidence kernel while reducing manual receipt duplication; no deletion before replacement/parity |
| S07 | `final/joil1.md` | Broad product brief: open 3D programmes, physics-blind evaluation, automatic run/checkpoint, reference vortex study and mass/reserve | Prose export has extensive blank lines; read as a brief, not independent proof of audited claims |
| S08 | `final/joil2.md` | EvoGate intent: modular traits, flexible topology, tiering, HPC, checkpoint and visual deliverables | Replace raw safety-factor/mass or allowable-stress/density objective with separate mass and common-demand reserve |
| S09 | `xfinal/JOIL_FINAL_ENGINEERING_SPEC.md` | Most detailed D-01…D-31 defect ledger, 16 decisions, module catalogue, operational/recovery/benchmark definitions and test→gate→render map | Several claims are too strong: exact-distance grids, universal PDR bands, Wilson parcel counts, mandatory custom GPU path and immediate new framework scope |
| S10 | `xfinal/JOIL_v5_Final_Engineering_Specification.md` | Corrects S09 toward two tracks, bounded partition geometry, reference native solvers, weighted/clustered uncertainty, fixed initial policy, careful causal/validation scope | Preferred resolution of these substantive conflicts; preserve S09 operational detail where compatible |

The section index maps subject areas to execution packets. It is navigation/coverage evidence, not a claim that a simple keyword matcher decomposes every sentence into a formal requirement. The human reconciliation below and task-level acceptance/negative controls are the execution decisions.

## Decisions resolving actual conflicts

| Topic | Conflict | Selected implementation policy | Packet |
| --- | --- | --- | --- |
| Source authority | Several “final” documents with different ladders/numbers | Existing AGENTS/master/science govern; this annex is the explicit next-revision planning decision; no automatic requirement activation | 00,02 |
| Engine naming | Crucible vs EvoGate vs Joil; immediate crate renames | New domain-neutral modules use working `crucible-*` names; preserve current joil-evidence identity owner; no branding-only migration | 03,31 |
| Reference versus custom solvers | S09 makes new GPU LBM/FEA central; S02/S05/S10 prefer native first | Qualify corrected pinned native adapters, then optional accelerator admission with reference-only fallback | 14,19,24 |
| Geometry authority | Exact SDF language vs error-bounded implicit partition | One disjoint material partition and accepted artifact manifest with explicit approximation bounds | 08–10 |
| “Exact” re-distancing | Jump flooding/sweeping plus grid pitch described as proof | Approximate algorithm needs validated error; unresolved cells/features stay unresolved | 08 |
| Independent morphology | Open fluid and solid separately | Maintain partition consistency through filters; record all repair and reject excessive changes | 08 |
| Topology | Volume Betti compared with boundary Euler/genus | Compare two volume complexes and separately validate boundary consistency | 09 |
| Novelty | Gene/taxon labels and hull escape used as proof | Measured archive/structural/mechanism/external novelty are separate; finite hull escape alone proves no external novelty | 12,21,28 |
| Vortex causation | First-order/suction observations asserted to prove missing vortex | Controlled same-subject factor study; independent data decide cause | 14,15 |
| PDR and gauge pressure | Borrowed [1.4,2.6] band and no-negative-gauge condition | PDR observed under split control; range/source and absolute datum must match actual geometry/application | 02,14,15 |
| Fidelity labels | L0–L4 vs L0–L5 with physical level shifted | Store evaluator/fidelity explicitly plus separate E0–E3 observable evidence class and R1–R4 release outcome; never migrate by ordinal alone | 03,18,31 |
| Statistical qualification | Three-times-uncertainty slogan vs controlled confidence | One-third budget is run planning; DecisionProfile governs inference, multiplicity, correlation/bias and adaptive attempts | 02,04 |
| Oil confidence | Raw 8000-per-bin Wilson calculation | Estimate represented-mass capture with valid independent/clustered units and quadrature error | 04,16,17 |
| Robustness | Sobol 72 draws used as IID p05/p95 proof | IID tail intervals, replicated randomized QMC for suitable means, finite envelopes for finite scenarios | 04,18 |
| Error combination | Blanket linear or RSS rules | Preserve uncertainty types; model joint correlation or conservatively propagate justified bounds; absent discrepancy stays Unknown | 04,20 |
| Mass objective | Original master makes grams non-goal; later briefs prioritize mass | Plan four constrained bounded objectives including actual printed mass and reserve; activate only in new requirement/policy identity | 02,13,18 |
| Reserve | Mix yield/buckling/frequency/fatigue ratios | Common-demand load multipliers with explicit compatible limits; modal/fatigue remain separate gates unless justified multiplier definition exists | 19,20 |
| Preferences | Various four/five-objective weights and scalar KPI | Retain Pareto front; propose 0.35 mass/0.20 reserve/0.30 capacity/0.15 energy with no-regression floors and sensitivity decisions | 18 |
| Process thresholds | 1.4 mm overflow interpreted as printed passage; 3 mm draft passport | Machined overflow exception with access/metrology; all printed feature limits require process passport and status | 02,13 |
| Material eligibility | Literature/wrought values treated as AM allowables | Process/orientation/finish/environment source-bound passports; missing data blocks dependent claims | 13,20 |
| Energy | Static pressure work per feed volume mislabeled treated energy | Preserve old metric identity; introduce flux loss per treated m³ at common duty | 01,14,18 |
| Search complexity | Immediate NSGA-III/MOME/AURORA/bandits/motifs | Deterministic fixed reference-search/1 first; experimental v2 policies only after audit | 21,22 |
| Fair compute | Equal raw GPU-seconds across unlike workloads/devices | Report resource cost vector and declared frozen normalization; include every attempt, audit, training and confirmation | 06,23,28 |
| Checkpoint determinism | Empty contract checks and bit-identical actual cost promises | Nonempty crash tests; logical result-stream replay distinct from native tolerances and real elapsed/retry cost | 06,07 |
| Tail operator | Frozen geometry assumed to imply pressure-only response | Bidirectional state/conservation/validation and full-liner abstention fallback | 25 |
| Rendering | Every test as heroic physical picture; invented motion | Physical recipes where data exists, exact cards otherwise; unavailable states explicit, unsimulated motion labeled | 26,27 |
| Blender version | Unverified future Blender 5.2 LTS pin | Choose an actually available tested build at implementation; record exact build; no presumed release | 26 |
| Historical work | Rewrite dossiers, move everything and delete duplicates now | Append new interpretation and preserve bytes; retire production duplicates only after parity and independent fixtures | 00,01,31 |
| Coordination | Collapse all channels immediately | Keep root COORD authoritative now; migrate only by acknowledged ownership change without destroying journals | 00,31 |
| Single PR | Source roadmaps suggest numerous PRs | One current draft integration PR containing this complete preparation; later implementation may accumulate here under owner direction, without opening lane PRs | 31 |
| Reuse | Port another project to prove reuse | Bracket pack inside this repository; cross-project karambit work requires separate scope/authorization | 29 |
| Disclosure/license | Push arbitrary elite assets; choose license automatically | Plan/code can be reviewed; raw inputs/candidate artifacts stay private; owner chooses license/disclosure | 00,05,28,31 |

## Code findings versus hypotheses

Selected code paths were checked directly at the baseline; `CODE_AUDIT.json` records exact file hashes and matched anchors. Confirmed examples include the preregistration key mismatch, per-feed energy formula, unused mesh argument/hard-coded powder result, copied enactment delta, compatibility-only checkpoint flag, implicit VRAM activation and stale UI pressure/trajectory wording. These are concrete software defects or claim-semantic defects.

Physical explanations for the vortex/PDR discrepancy, suitability of a specific wall model, prospective GPU speed, actual fatigue reserve, printing limits and attainable improvement remain untested. The source authors’ statements that all their findings were verified are source assertions; this plan does not inherit that qualification. Existing STATUS explicitly records no qualified baseline/oil grade/structure/physical validation.

D-05 needs precision: the Rust flood fill still ignores its mesh and assigns zero powder, while the current Python observer has a derived split-metric condition rather than the older literal-zero description. Its independence/coverage remains a separate issue. Repair the actual current behavior, not stale source line text.

## Primary references checked for planning constraints

The runtime must follow actual [SQLite WAL same-host and durability behavior](https://www.sqlite.org/wal.html). Numerical qualification should assess assumptions behind [NASA’s spatial-convergence guidance](https://www.grc.nasa.gov/www/wind/valid/tutorial/spatconv.html), not borrow a decorative GCI certificate. Kernel and rendering dependencies must be rechecked against [fidget’s own documentation](https://docs.rs/fidget/latest/fidget/) and [Blender’s official release notes](https://developer.blender.org/docs/release_notes/) before pinning. No specific future release/API or benchmark is promised here. The physical papers mentioned in sources still need accession of usable original data and complete metadata; this planning review does not claim to have acquired or validated those datasets.
