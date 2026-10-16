# 2026-10-15 v4 review synthesis: finding → workplan item → status

Owner: DOC lane (`joil_paper`), workplan item DOC-02. This file is the traceability table for the six
external reviews supplied on 2026-10-15 and checksummed in
[`docs/review-inputs/2026-10-15/`](../review-inputs/2026-10-15/INDEX.md). Every finding that a review marks
P0, stop-the-line, critical, immediate or no-regret appears below with the workplan item(s) that answer it.

- Reviews audited JOIL `main` at `d1ac285`. The DOC lane re-checked each finding against this lane's base
  commit `7353526`. That base includes Wave-3 (`eb39ee2`), so Wave-3 claims are within scope.
- Workplan: [`docs/WORKPLAN-2026-10-15.md`](../WORKPLAN-2026-10-15.md) (59 items).
- **Status as of 2026-10-16T11:39Z.** Statuses come only from `DONE`/`MERGED` lines on the live
  coordination board, never from this file's author. `python3 tools/claims_audit.py` checks that this
  table is internally consistent: every required finding is present, every item ID exists in the
  workplan, and every finding status matches the ledger in §2.

| Key | Review | SHA-256 (verified, `sha256sum -c`) |
|---|---|---|
| S1 | JOIL vNext: Alien Invention Engine | `c26d5b97c65230112c3beab8e099804adab7d995aa78206985c480b7ef573b89` |
| S2 | JOIL Alien Invention Engine (ADG-v1) | `49623ed181f9ff35ec95f99a3425893ae5907f74eff593a125228c363c082181` |
| S3 | JOIL Xenogen v4 | `2ed1486c8d9b71cfce8e1b79aff605e0cf910b1266934f7db6aeba846898ab39` |
| S4 | Engineering verdict (TDFG) | `1d29e9a1af2b765a6cb81f280877a94211ad3069d1d3c087aeb182ca9a51d6e8` |
| S5 | JOIL v4: Open-World Scientific Invention Engine | `0c33ec23acffd69c6fe610370726a3246061ec0d10ceb21ef8c9c3ed8d806c05` |
| S6 | JOIL XenoGenome v4 | `f7f82721e83cac1bb0cdb0d6ee251475c5e61d768c879d485ceee9a34228a9ac` |

## 1. How to read the tables

**Verified@7353526** column. `V:` means the DOC lane reproduced the finding in code at `7353526`; the
location follows. Bare file names are under `crates/joil-core/src/`. `R:` means the finding is taken from
the review and has not been independently re-checked (the reason is given). `N/A` marks a design
recommendation with nothing to reproduce.

**Coverage** column:

- `FULL`: the mapped items, once merged, close the finding in this PR.
- `PARTIAL`: this PR closes the stop-the-line part. The remainder is a workplan §6 follow-on, named in
  the row.
- `FOLLOW-ON`: workplan §6 only. Not in this PR.
- `GAP`: no workplan item exists yet. §5 lists a proposed owner. Until the planner or integrator assigns
  it, the finding stays `UNASSIGNED`.

**Status** column (derived, never hand-set):

- For `FULL` and `PARTIAL` rows, status is the least-advanced status among the mapped items in §2:
  `BLOCKED` < `OPEN` < `WIP` < `DONE` < `MERGED`.
- `FOLLOW-ON` rows show `DEFERRED`. `GAP` rows show `UNASSIGNED`.
- A `PARTIAL` row at `MERGED` still has its follow-on remainder open.

## 2. Item ledger (source of truth for derived status)

Update rule: `DONE <sha>` when the lane posts `DONE <item> <sha>`, `MERGED <sha>` when the integrator posts
`MERGED <item> <sha>`, `WIP` after a lane posts `START`/`WIP` for that item, and `BLOCKED` only with a posted
reason.

| Item | Lane | Status | Evidence (board line / sha) |
|---|---|---|---|
| TR-01 | TR | MERGED | `4715671` (board `MERGED TR-01`) |
| TR-02 | TR | MERGED | `92be04f` (board `MERGED TR-02`) |
| TR-03 | TR | MERGED | `17a620b` (board `MERGED TR-03`) |
| TR-04 | TR | MERGED | `4715671` (board `MERGED TR-04`) |
| TR-05 | TR | MERGED | `bf9155b` (board `MERGED TR-05`) |
| TR-06 | TR | MERGED | `bf9155b` (board `MERGED TR-06`) |
| TR-07 | TR | MERGED | `92be04f` (board `MERGED TR-07`) |
| TR-08 | TR | MERGED | `0441561` (board `MERGED TR-08`) |
| TR-09 | TR | MERGED | `0441561` (board `MERGED TR-09`) |
| TR-10 | TR | MERGED | `a5f16c0` (board `MERGED TR-10`) |
| TR-11 | TR | MERGED | `6873a7f` (board `MERGED TR-11`) |
| TR-12 | TR | MERGED | `d13355b` (board `MERGED TR-12`) |
| TR-13 | TR | MERGED | `1ed5865` (board `MERGED TR-13`) |
| TR-14 | TR | MERGED | `92be04f` (board `MERGED TR-14`) |
| TR-15 | TR | MERGED | `1ed5865` (board `MERGED TR-15`) |
| TR-16 | TR | MOVED-§6 | board `MOVED-§6 TR-16`: physical limits and experimental/statistical policy criteria deferred to workplan §6 pending admitte |
| TR-17 | TR | MERGED | `1ed5865` (board `MERGED TR-17`) |
| EV-01 | EV | MERGED | `1433a71` (board `MERGED EV-01`) |
| EV-02 | EV | MERGED | `1433a71` (board `MERGED EV-02`) |
| EV-03 | EV | MERGED | `1433a71` (board `MERGED EV-03`) |
| EV-04 | EV | MERGED | `1433a71` (board `MERGED EV-04`) |
| EV-05 | EV | MERGED | `1433a71` (board `MERGED EV-05`) |
| EV-06 | EV | MERGED | `84d0c2e` (board `MERGED EV-06`) |
| EV-07 | EV | MERGED | `cae5bac` (board `MERGED EV-07`) |
| EV-08 | EV | MERGED | `e83c833` (board `MERGED EV-08`) |
| EV-09 | EV | MOVED-§6 | board `MOVED-§6 EV-09`: independent Cout labels and reviewed observed-mechanism dataset joil-independent-oil-observations-v1 |
| EV-10 | EV | MERGED | `ceaa03d` (board `MERGED EV-10`) |
| EV-11 | EV | MERGED | `d3a5449` (board `MERGED EV-11`) |
| EV-12 | EV | MERGED | `eeee9f9` (board `MERGED EV-12`) |
| IR-01 | IR | MERGED | `2f15710` (board `MERGED IR-01`) |
| IR-02 | IR | MERGED | `2f15710` (board `MERGED IR-02`) |
| IR-03 | IR | MERGED | `2f15710` (board `MERGED IR-03`) |
| IR-04 | IR | MERGED | `2f15710` (board `MERGED IR-04`) |
| IR-05 | IR | MERGED | `2f15710` (board `MERGED IR-05`) |
| IR-06 | IR | MERGED | `2f15710` (board `MERGED IR-06`) |
| IR-07 | IR | MERGED | `2f15710` (board `MERGED IR-07`) |
| IR-08 | IR | MERGED | `2f15710` (board `MERGED IR-08`) |
| IR-09 | IR | MERGED | `2f15710` (board `MERGED IR-09`) |
| IR-10 | IR | MERGED | `2f15710` (board `MERGED IR-10`) |
| IR-11 | IR | MERGED | `2f15710` (board `MERGED IR-11`) |
| IR-12 | IR | MERGED | `2f15710` (board `MERGED IR-12`) |
| INT-01 | INT | MERGED | `cb31ba3` (board `MERGED INT-01`) |
| INT-02 | INT | WIP | board HB 2026-10-16T07:06:55Z |
| INT-03 | INT | MERGED | `6aa68b4` (board `MERGED INT-03`) |
| INT-04 | INT | MERGED | `cb31ba3` (board `MERGED INT-04`) |
| INT-05 | INT | MERGED | `92be04f` (board `MERGED INT-05`) |
| INT-06 | INT | MERGED | `cab01e8` (board `MERGED INT-06`) |
| INT-07 | INT | MERGED | `cb31ba3` (board `MERGED INT-07`) |
| INT-08 | INT | OPEN | - |
| DOC-01 | DOC | MERGED | `dd82f45` (board `MERGED DOC-01`) |
| DOC-02 | DOC | MERGED | `dd82f45` (board `MERGED DOC-02`) |
| DOC-03 | DOC | MERGED | `631e2e3` (board `MERGED DOC-03`) |
| DOC-04 | DOC | MERGED | `cf9df59` (board `MERGED DOC-04`) |
| DOC-05 | DOC | MERGED | `0139ca0` (board `MERGED DOC-05`) |
| DOC-06 | DOC | MERGED | `940d0b4` (board `MERGED DOC-06`) |
| DOC-07 | DOC | MERGED | `cf9df59` (board `MERGED DOC-07`) |
| DOC-08 | DOC | MERGED | `6aa68b4` (board `MERGED DOC-08`) |
| DOC-09 | DOC | MERGED | `5f95ad2` (board `MERGED DOC-09`) |
| DOC-10 | DOC | WIP | merged at `5947b75` without board `DONE DOC-10` |

## 3. Consensus themes

The reviews agree on eleven themes. The workplan §0 numbers them the same way.

| # | Theme | Raised by | Answering items in this PR | Remainder (§6 follow-on) |
|---|---|---|---|---|
| 1 | Silent fallback to `baseline_ct35()` | S1 S2 S3 S4 S5 S6 | TR-01, TR-02, TR-03, TR-04, EV-08 | none |
| 2 | Schema identity erased (v3→v2) | S2 S4 S5 | TR-05, TR-06 | none |
| 3 | Genome ≠ geometry across Rust mesh / Python B-rep / CFD | S1 S2 S3 S4 S5 S6 | IR-01, IR-02, IR-03, IR-04, IR-05 | multi-region compiler emitting real internal surfaces |
| 4 | Topology is an authored label | S1 S3 S4 S5 S6 | IR-06 | measured-descriptor QD archives |
| 5 | Reduced evaluator is circular | S1 S2 S3 S4 S5 S6 | IR-07, DOC-03, DOC-08 | judge process/crate separation |
| 6 | Structural verification bugs | S1 S3 S5 S6 | TR-07, TR-08, TR-09, TR-10, INT-05 | candidate-specific solid FEA |
| 7 | F0 facts asserted, not measured | S2 S3 S6 | IR-08, IR-09, IR-12 | none |
| 8 | Surrogate imputes defaults; CUDA "parcels" are tensor cells | S1 S2 S5 S6 | EV-08, EV-09, IR-10 | multi-fidelity surrogate |
| 9 | Requirements not admitted; objective/HV/MC statistics drift | S1 S3 S4 | EV-06, EV-10, TR-11, TR-15, TR-16 | see §5 GAP rows for bootstrap/LHS |
| 10 | "Expressiveness" 220/220 is closed-catalog validity | S2 S5 | TR-14, DOC-03 | open-world challenge oracle |
| 11 | Target architecture (evidence kernel, DesignIR, blind judge, QD) | all | EV-01..EV-05, EV-07, EV-11, EV-12, IR-01, IR-11 | genome v4, QD rewrite, mechanism cards, F3/F4 |

## 4. Per-source traceability

### 4.1 S1: JOIL vNext Alien Invention Engine

| ID | Source | Finding | Verified@7353526 | Items | Coverage | Status |
|---|---|---|---|---|---|---|
| S1-3.1 | S1 §3.1 | Genome is a fixed catalog of seven layers plus 11 hand-classified labels, not an open language | V: `program.rs:209` struct, `program.rs:1346` `CANONICAL_11_FAMILIES` | DOC-06, IR-01 | PARTIAL (genome v4 is §6) | MERGED |
| S1-3.2 | S1 §3.2 | Branch, ribbon and TPMS genes do not become the geometry sent to the judge; CFD surface is five patches with two inlets | V: `cfd_case.rs:3`, `cfd_case.rs:334`; ribbons/TPMS are wall-radius perturbations `phenotype.rs:229`, `phenotype.rs:243` | IR-02, IR-03, IR-04, IR-05 | PARTIAL (real internal surfaces are §6) | MERGED |
| S1-3.3 | S1 §3.3 | Topology is named by the generator (`classify_topology`), not measured | V: `program.rs:551` | IR-06 | FULL | MERGED |
| S1-3.4 | S1 §3.4 | Reduced evaluator rewards genome assumptions (gene-derived descriptors feed the score) | V: `program.rs:618` `descriptors()`; e.g. TPMS shear ×0.86 at `program.rs:684` | IR-07, DOC-03, DOC-08 | PARTIAL (judge separation is §6) | MERGED |
| S1-3.5 | S1 §3.5 | Invalid designs silently become the baseline | V: 26 sites: 24 in `program.rs` (first `program.rs:1040`), `surrogate.rs:361`, `evolve.rs:2268` | TR-01, TR-02, TR-03, EV-08 | FULL | MERGED |
| S1-3.6 | S1 §3.6 | Requirement set not admitted while runtime invents thresholds | V: `requirements/RS-joil-research-v1.json` status `draft_no_campaign_admission`; `evolve.rs:427` (1.15× baseline Δp), `evolve.rs:431` (PDR 4.5) | EV-06, EV-10, TR-16 | FULL | DEFERRED |
| S1-3.7 | S1 §3.7 | Structural "qualification" has non-independent checks; structural failure ignored by dominance | V: `structure.rs:232` `div_euclid` then clamp to [1,4]; `structure.rs:243`, `structure.rs:250` reaction := load; `evolve.rs:435` result stored, not in violation | TR-07, TR-08, TR-09, TR-10 | PARTIAL (solid FEA is §6) | MERGED |
| S1-3.8 | S1 §3.8 | CUDA tensor size is reported as Lagrangian parcel count; candidate Cout/d50 are closed-form | V: `physics/cuda/joil_cuda_eval.cu:452` `parcels_integrated = num_cells`; d50/Cout closed-form at `:507`, `:535`; no reduction over cells | IR-10, DOC-03 | FULL | MERGED |
| S1-3.9 | S1 §3.9 | Surrogate LOFO uses authored labels and gene-derived features; not out-of-ontology | V: `surrogate.rs:535` folds keyed on `topology_class` | EV-08, EV-09 | FULL | DEFERRED |
| S1-15.01 | S1 §15-1 | Remove every silent fallback to `baseline_ct35()` or default performance features | V: see S1-3.5 | TR-01, TR-02, EV-08 | FULL | MERGED |
| S1-15.02 | S1 §15-2 | Invalid branch counts and schema mismatches become hard errors | V: `program.rs:329` silently coerces `branches_per_port` to 2; `cad/program/exotic_graph.py:125` coerces to {2,3} | TR-06, IR-03 | FULL | MERGED |
| S1-15.03 | S1 §15-3 | Stop accepting one schema/family and serialising it as another | V: `program.rs:280` accepts v2/v3, `program.rs:483` emits v2 | TR-05 | FULL | MERGED |
| S1-15.04 | S1 §15-4 | Derive topology from final geometry | V: see S1-3.3 | IR-06 | FULL | MERGED |
| S1-15.05 | S1 §15-5 | Compile branch, ribbon, TPMS and skimmer into the exact solver subject | V: see S1-3.2 | IR-02, IR-03 | PARTIAL (this PR: hard `UnsupportedFeature`; real compilation is §6) | MERGED |
| S1-15.06 | S1 §15-6 | Feature-enactment receipts | N/A (absent) | IR-04 | FULL | MERGED |
| S1-15.07 | S1 §15-7 | Relabel the `6jet` CFD result to the geometry actually simulated until six paths are proven | V: CFD case has `inlet1`/`inlet2` only (`cfd_case.rs:334`) | IR-05, DOC-03 | FULL | MERGED |
| S1-15.08 | S1 §15-8 | Treat reduced-order descriptors as proposal heuristics, not observations | V: see S1-3.4 | IR-07, DOC-03, DOC-08 | FULL | MERGED |
| S1-15.09 | S1 §15-9 | Structural failure enters constrained dominance | V: `evolve.rs:435` `.ok()` result never added to violation | TR-09 | FULL | MERGED |
| S1-15.10 | S1 §15-10 | Observed order by ordinary division, not `div_euclid` | V: `structure.rs:232` | TR-07, INT-05 | FULL | MERGED |
| S1-15.11 | S1 §15-11 | Replace constructed-zero load-transfer check with independent integration | V: `structure.rs:243`, `structure.rs:250` | TR-08 | FULL | MERGED |
| S1-15.12 | S1 §15-12 | Compute trapped cavities, frozen-interface difference, passage sizes and terminal geometry independently | V: `phenotype.rs:520` `trapped_cavities: 0`; trusted at `redteam.rs:90` | IR-08, IR-09 | FULL | MERGED |
| S1-15.13 | S1 §15-13 | Evidence-tier enums and evidence references instead of mutable strings | R: string tiers in `analysis/evidence.py` not enumerated here | EV-04 | FULL | MERGED |
| S1-15.14 | S1 §15-14 | Parent IDs, operator ID/version, seed, repair receipt, compiler identity on every individual | V: `Individual` struct `evolve.rs:134` has no parent/operator fields | TR-13, TR-04 | FULL | MERGED |
| S1-15.15 | S1 §15-15 | Stop calling tensor-cell count an integrated parcel count | V: see S1-3.8 | IR-10, DOC-03 | FULL | MERGED |
| S1-15.16 | S1 §15-16 | Use admitted requirement values; no unexplained search thresholds | V: see S1-3.6 | EV-06, TR-16 | FULL | DEFERRED |
| S1-15.17 | S1 §15-17 | Rename Monte Carlo "probability" outputs unless distributions are justified | R: wording audit pending in DOC-08 | TR-15, DOC-08 | FULL | MERGED |
| S1-15.18 | S1 §15-18 | Train surrogates only on independent labels; real OOD holdouts | V: see S1-3.9 | EV-09 | FULL | DEFERRED |
| S1-15.19 | S1 §15-19 | Judge blind to topology labels and generator origin | V: MAP-Elites key includes `topology_class` (`evolve.rs:589`) | IR-07, IR-06 | PARTIAL (process-level blind judge is §6) | MERGED |
| S1-15.20 | S1 §15-20 | Explicit sandbox vs claim-bearing campaign boundary | N/A (absent) | EV-10 | FULL | MERGED |

### 4.2 S2: Alien Design Genome (ADG-v1)

| ID | Source | Finding | Verified@7353526 | Items | Coverage | Status |
|---|---|---|---|---|---|---|
| S2-4.01 | S2 §4.1 | Implemented genome is a feature catalogue, not a general design program | V: see S1-3.1 | DOC-06 | PARTIAL (genome v4 is §6) | MERGED |
| S2-4.02 | S2 §4.2 | Primary Rust phenotype is star-shaped `R(θ,z)` rings; cannot hold internal bodies or branch networks | V: `phenotype.rs:185` `inner_radius_mm(prog, d, theta, z)` | IR-02, DOC-06 | PARTIAL (multi-region compiler is §6) | MERGED |
| S2-4.03 | S2 §4.3 | F0 facts (frozen-tail diff, trapped cavities, TPMS pore size, mass) asserted, not measured | V: `phenotype.rs:520`; arithmetic TPMS/ribbon volumes `phenotype.rs:457` | IR-08, IR-09 | FULL | MERGED |
| S2-4.04 | S2 §4.4 | Screening physics is self-referential | V: see S1-3.4 | IR-07, DOC-03, DOC-08 | PARTIAL (evaluator isolation is §6) | MERGED |
| S2-4.05 | S2 §4.5 | Invalid offspring silently become the baseline | V: see S1-3.5 | TR-01, TR-02, TR-03 | FULL | MERGED |
| S2-4.06 | S2 §4.6 | Schema identity erased by normaliser | V: see S1-15.03 | TR-05 | FULL | MERGED |
| S2-4.07 | S2 §4.7 | "Homologous graph crossover" is fixed-field blending, not graph crossover | V: `program.rs:1181` `crossover_homologous_graph` blends fixed fields | TR-12 | FULL | MERGED |
| S2-4.08 | S2 §4.8 | Structure model not shape-general; observed order uses Euclidean division | V: `structure.rs:232` | TR-07, TR-10 | PARTIAL (general solid FEA is §6) | MERGED |
| S2-4.09 | S2 §4.9 | F1 DSD bin weights from pointwise lognormal PDF without declared basis or bin-edge integration | R: not re-checked in `evolve.rs` this pass | - | GAP (§5 G-01) | UNASSIGNED |
| S2-4.10 | S2 §4.10 | Surrogate silently imputes missing physical state | V: `surrogate.rs:361` | EV-08 | FULL | MERGED |
| S2-4.11 | S2 §4.11 | 100% valid operator walk is not proof of open-endedness | V: `program.rs:1603`, `program.rs:1706` audits sample the 11 authored families | TR-14, DOC-03 | PARTIAL (open-world challenge is §6) | MERGED |
| S2-A.1 | S2 §20 Milestone A-1 | Freeze current representation as a legacy lane | N/A | DOC-06, IR-01 | FULL | MERGED |
| S2-A.4 | S2 §20 Milestone A-4 | Mark current reduced-order physics screening-only | N/A | DOC-03, IR-07 | FULL | MERGED |
| S2-A.5 | S2 §20 Milestone A-5 | Replace hard-coded unknown geometry facts with typed unknowns | V: see S2-4.03 | IR-08, EV-03 | FULL | MERGED |
| S2-A.7 | S2 §20 Milestone A-7 | Operator identity and rejection accounting | N/A | TR-03, TR-13 | FULL | MERGED |

S2 Milestone A items 2, 3 and 6 are the same as S2-4.05, S2-4.06 and S2-4.08.

### 4.3 S3: Xenogen v4

| ID | Source | Finding | Verified@7353526 | Items | Coverage | Status |
|---|---|---|---|---|---|---|
| S3-P0-01 | S3 P0-01 | Rust search genomes and canonical B-rep are not equivalent (Python CAD is v2-only, coerces branches, lacks ribbon/TPMS) | V: `cad/program/exotic_graph.py:87` rejects non-v2; `:125` coerces branches | IR-01, IR-02, IR-03 | FULL | MERGED |
| S3-P0-02 | S3 P0-02 | Invalid offspring silently become the baseline | V: see S1-3.5 | TR-01, TR-02 | FULL | MERGED |
| S3-P0-03 | S3 P0-03 | Structural convergence order is wrong (`div_euclid`); needs p=1.5/2.0/3.2 laws, oscillatory and near-zero controls | V: `structure.rs:232`, plus clamp to [1,4] that would hide a correct p=3.2 near the bound and any p<1 | TR-07, INT-05 | FULL (see §5 note N-1 on test scope) | MERGED |
| S3-P0-04 | S3 P0-04 | Load-transfer conservation is self-confirming | V: see S1-15.11 | TR-08 | FULL | MERGED |
| S3-P0-05 | S3 P0-05 | F0 fields asserted (cavities, frozen tail, terminals, TPMS/ribbon volumes) | V: see S2-4.03 | IR-08 | FULL | MERGED |
| S3-P0-06 | S3 P0-06 | Fast Rust mesh is not a general representation of claimed mechanisms | V: see S2-4.02 | IR-02, IR-05 | PARTIAL (resolved internals are §6) | MERGED |
| S3-P0-07 | S3 P0-07 | Candidate-specific structural loads not tied to the hard constraint | V: see S1-15.09 | TR-09 | FULL | MERGED |
| S3-P0-08 | S3 P0-08 | One "minimum wall clearance" conflates ligament, throat, gap, vane and wall distance | V: `phenotype.rs:460`, `phenotype.rs:468` fold vane/sheet thickness into `min_clearance` | IR-09 | FULL | MERGED |
| S3-P0-09 | S3 P0-09 | Reduced-order evaluator is circular | V: see S1-3.4 | IR-07, DOC-03, DOC-08 | PARTIAL (judge separation is §6) | MERGED |
| S3-P0-10 | S3 P0-10 | "Exoticness" is a hand-weighted gene-activation score | V: `program.rs:1573` `exoticness_index` weights branch count and TPMS basis | DOC-03 | PARTIAL (wording now; novelty vector is §6; code rename is §5 G-05) | MERGED |
| S3-P0-11 | S3 P0-11 | p95 tail quantiles over-read at 12–24 MC samples | V: `evolve.rs:401` `percentile_sorted(&couts, 0.95)` over `mc_samples` | TR-15 | FULL | MERGED |
| S3-P0-12 | S3 P0-12 | Topology names are programmer-authored labels | V: see S1-3.3 | IR-06 | FULL | MERGED |

### 4.4 S4: Engineering verdict (TDFG)

| ID | Source | Finding | Verified@7353526 | Items | Coverage | Status |
|---|---|---|---|---|---|---|
| S4-1.Q | S4 §1, §4.2 | Four-decimal rounding inside normalisation is undeclared quantization, not exact identity | V: `program.rs:60` `round4` applied throughout `normalize()` | DOC-06 | PARTIAL (documented now; `NumericContract` is §5 G-04) | MERGED |
| S4-2.01 | S4 §2.1 | Genome is a feature menu; remove `topology_class` from genotype | V: `program.rs:212` field in `DesignProgram` | IR-06, DOC-06 | PARTIAL (genome v4 is §6) | MERGED |
| S4-2.02 | S4 §2.2 | CFD phenotype lacks the claimed internal topology; CFD results hold only for the compiled shell | V: see S1-3.2 | IR-02, IR-04, IR-05, DOC-03 | PARTIAL (multi-region compiler is §6) | MERGED |
| S4-2.03 | S4 §2.3 | Fast evaluator is self-fulfilling; needs evaluator blindness (crate-dependency CI test) | V: see S1-3.4 | IR-07 | PARTIAL (crate split + CI dependency test is §6) | MERGED |
| S4-2.04 | S4 §2.4 | Invalid offspring silently become the baseline | V: see S1-3.5 | TR-01, TR-02 | FULL | MERGED |
| S4-2.05 | S4 §2.5 | Schema v3 normalised into v2 | V: see S1-15.03 | TR-05 | FULL | MERGED |
| S4-2.06 | S4 §2.6 | Preregistered objectives ≠ executed objective vector | V: prereg `experiments/wave2-preregistration-v1.json` `objectives_minimize_5d` = [Cout p95, specific energy, 1−E20 p05, d50 p95, Δp]; code `evolve.rs:412` = [Cout p95, Δp, −E20 p05, entry shear, short-circuit] | EV-06 | FULL | MERGED |
| S4-2.07 | S4 §2.7 | "Hypervolume" is not Pareto HV; bootstrap count/method differ from protocol; `RandomLhsExotic3d` is not LHS | V: `evolve.rs:1388` max single box + diversity bonus; `evolve.rs:1435` `n_boot = 2000` vs prereg `bootstrap_resamples: 10000`; `evolve.rs:1569` LHS arm calls `sample_random_exotic` | TR-11, DOC-03 | PARTIAL (HV fixed by TR-11; bootstrap and LHS are §5 G-02, G-03) | MERGED |
| S4-2.08 | S4 §2.8 | Requirement set not admitted; non-admitted sets must not produce Promoted/Qualified/Validated states | V: see S1-3.6 | EV-10, DOC-08 | FULL | MERGED |
| S4-2.09 | S4 §2.9 | Fixed family labels make QD archive anti-alien | V: `evolve.rs:589` key `(topology_class, swirl_bin, shear_bin, shield_bin)` | IR-06 | PARTIAL (measured-descriptor archives are §6) | MERGED |
| S4-2.10 | S4 §2.10 | Structural evaluator is a screen, not arbitrary-topology FEA | V: `structure.rs` Lamé/beam formulas | TR-10 | PARTIAL (general FEA is §6) | MERGED |
| S4-19.01 | S4 §19-1 | Stop baseline fallback | V: see S1-3.5 | TR-02 | FULL | MERGED |
| S4-19.02 | S4 §19-2 | Stop normalising v3 into v2 | V: see S1-15.03 | TR-05 | FULL | MERGED |
| S4-19.03 | S4 §19-3 | Freeze and repair the preregistration/objective mismatch | V: see S4-2.06 | EV-06 | FULL | MERGED |
| S4-19.04 | S4 §19-4 | Replace the hypervolume function | V: see S4-2.07 | TR-11 | FULL | MERGED |
| S4-19.05 | S4 §19-5 | Make splitter, ribbon and TPMS geometry appear in the analysis domain | V: see S1-3.2 | IR-02 | PARTIAL (this PR: refuse; real geometry is §6) | MERGED |
| S4-19.06 | S4 §19-6 | Remove direct gene-name effects from the physics evaluator | V: see S1-3.4 | IR-07 | PARTIAL (judge separation is §6) | MERGED |
| S4-19.07 | S4 §19-7 | Evaluators depend only on phenotype artifacts | N/A | IR-07 | PARTIAL (crate split is §6) | MERGED |
| S4-19.08 | S4 §19-8 | Import Jarambit scientific identity and lineage | N/A | EV-01, EV-02, TR-13 | FULL | MERGED |
| S4-19.09 | S4 §19-9 | Requirement files are the sole source of thresholds | V: see S1-3.6 | EV-06, TR-16 | FULL | DEFERRED |
| S4-19.10 | S4 §19-10 | Demote the eleven families from ontology to seed recipes | N/A | IR-06, DOC-06 | FULL | MERGED |
| S4-19.11 | S4 §19-11 | Typed graph plus implicit-field IR | N/A | IR-01 | PARTIAL (field DAG and genome v4 are §6) | MERGED |
| S4-19.12 | S4 §19-12 | Morphology and mechanism archives from measured outputs | N/A | - | FOLLOW-ON | DEFERRED |

### 4.5 S5: Open-World Scientific Invention Engine

| ID | Source | Finding | Verified@7353526 | Items | Coverage | Status |
|---|---|---|---|---|---|---|
| S5-P0.1 | S5 P0.1 | Silent fallback; six corruptions including budget accounting | V: see S1-3.5 | TR-01, TR-02, TR-03 | FULL | MERGED |
| S5-P0.2 | S5 P0.2 | Schema identity not preserved; `validate()` must not mutate; explicit `migrate_v2_to_v4` | V: see S1-15.03 | TR-05 | FULL | MERGED |
| S5-P0.3 | S5 P0.3 | Genome and exact CAD do not share one feature set | V: see S3-P0-01 | IR-01, IR-02, IR-03 | FULL | MERGED |
| S5-P0.4 | S5 P0.4 | Proxy geometry asked to certify topology; `trapped_cavities` default zero | V: see S2-4.03 | IR-04, IR-08 | FULL | MERGED |
| S5-P0.5 | S5 P0.5 | Human priors leak directly into the score (`PriorDescriptors` vs `ObservedDescriptors`) | V: see S1-3.4 | IR-07, DOC-08 | FULL | MERGED |
| S5-P0.6 | S5 P0.6 | Topology is a hard-coded label catalog; derive `PhenotypeTaxon` | V: see S1-3.3 | IR-06 | FULL | MERGED |
| S5-P0.7 | S5 P0.7 | Surrogate defaults hide invalidity and novelty | V: see S2-4.10 | EV-08 | FULL | MERGED |
| S5-P0.8 | S5 P0.8 | "Expressiveness" is a closed-catalog validity test; rename and add open-world challenge | V: see S2-4.11 | TR-14, DOC-03 | PARTIAL (open-world challenge oracle is §6) | MERGED |
| S5-M0.1 | S5 §22 M0 | Zero silent baseline fallbacks | V: see S1-3.5 | TR-02 | FULL | MERGED |
| S5-M0.2 | S5 §22 M0 | Zero silent schema downcasts | V: see S1-15.03 | TR-05 | FULL | MERGED |
| S5-M0.3 | S5 §22 M0 | Zero active features ignored by a qualifying backend | V: see S1-3.2 | IR-02 | FULL | MERGED |
| S5-M0.4 | S5 §22 M0 | Zero scientific fields defaulted from unknown to physical values | V: see S2-4.03, S2-4.10 | EV-03, EV-08, IR-08 | FULL | MERGED |
| S5-M0.5 | S5 §22 M0 | 100% of variation attempts in the budget ledger | N/A | TR-03 | FULL | MERGED |

### 4.6 S6: XenoGenome v4

| ID | Source | Finding | Verified@7353526 | Items | Coverage | Status |
|---|---|---|---|---|---|---|
| S6-3.1 | S6 §3.1 | Documented graph genome is not the implemented genome; drop "flexible graph CAD implemented" from status | V: `docs/STATUS.json` `flexible_graph_cad_implemented: true`; `docs/GENOME.md` labels the graph API as proposed | EV-07, DOC-06 | FULL | MERGED |
| S6-3.2 | S6 §3.2 | Topology labels are not topology | V: see S1-3.3 | IR-06 | FULL | MERGED |
| S6-3.3 | S6 §3.3 | Screening mesh does not instantiate claimed mechanisms | V: see S1-3.2, S2-4.03 | IR-02, IR-04, IR-08 | PARTIAL (resolved internals are §6) | MERGED |
| S6-3.4 | S6 §3.4 | Exact CAD paths disagree (`wall_field.py` smooth wall only, `exotic_graph.py` v2, Rust v3) | V: `cad/program/wall_field.py:15` accepts only `modes`; see S3-P0-01 | IR-03 | FULL | MERGED |
| S6-3.5 | S6 §3.5 | Invalid variation can silently become the baseline | V: see S1-3.5 | TR-01, TR-02 | FULL | MERGED |
| S6-3.6 | S6 §3.6 | Reduced-order evaluator contains the design theory it tests | V: see S1-3.4 | IR-07, DOC-03 | PARTIAL (science-court separation is §6) | MERGED |
| S6-3.7 | S6 §3.7 | Structural "qualification" is an F1 analytic screen; zero-by-construction load transfer; `div_euclid` | V: see S1-3.7 | TR-07, TR-08, TR-10 | PARTIAL (3D FEA is §6) | MERGED |
| S6-3.8 | S6 §3.8 | CUDA and surrogate subjects incomplete (partial genome JSON + descriptors; 12-D handcrafted features) | V: CUDA reads gene-derived descriptors (`physics/cuda/joil_cuda_eval.cu:265`); see S1-3.9 | IR-10, EV-08, EV-09 | FULL | DEFERRED |
| S6-3.9 | S6 §3.9 | Tests prove internal consistency, not external truth; classify tests and never combine verdicts | V: `crates/joil-core/tests/regression.rs` family-validity audits | DOC-03 | PARTIAL (wording now; test taxonomy is §5 G-06) | MERGED |
| S6-A.01 | S6 App.A-1 | All `mutate_*`/crossover return `Result<VariationOutcome, VariationError>` | V: see S1-3.5 | TR-01 | FULL | MERGED |
| S6-A.02 | S6 App.A-2 | Delete every `unwrap_or_else(\|_\| baseline_ct35())` in variation | V: see S1-3.5 | TR-02 | FULL | MERGED |
| S6-A.03 | S6 App.A-3 | Remove `topology_class` from serialised genome | V: see S4-2.01 | IR-06 | FULL | MERGED |
| S6-A.04 | S6 App.A-4 | Remove hard-coded `trapped_cavities: 0` and frozen-difference constants | V: `phenotype.rs:520` | IR-08 | FULL | MERGED |
| S6-A.05 | S6 App.A-5 | Remove arithmetic feature volumes from geometry qualification | V: `phenotype.rs:461` vane volume = count × span × chord × thickness | IR-08, IR-09 | FULL | MERGED |
| S6-A.06 | S6 App.A-6 | Reject a candidate when a claimed gene is not represented in the canonical subject | V: see S1-3.2 | IR-02 | FULL | MERGED |
| S6-A.07 | S6 App.A-7 | Replace partial CUDA JSON with subject-bundle reference plus exact artifact bytes | V: see S6-3.8 | IR-10 | FULL | MERGED |
| S6-A.08 | S6 App.A-8 | Fix observed-order arithmetic; add GCI validity states | V: see S1-15.10 | TR-07 | FULL | MERGED |
| S6-A.09 | S6 App.A-9 | Rename structure verdict `AnalyticStructuralScreen` | V: `evolve.rs:145` field `structural_qualification` | TR-10 | FULL | MERGED |
| S6-A.10 | S6 App.A-10 | OpenFOAM/FEA case manifests embed full scientific identity | N/A | EV-12 | FULL | MERGED |
| S6-A.11 | S6 App.A-11 | Move `MechanismDescriptors` to an inventor-only module | V: `program.rs:227` | IR-07 | FULL | MERGED |
| S6-A.12 | S6 App.A-12 | Replace `Option<f64>` science fields with typed observations | R: not enumerated here | EV-03 | FULL | MERGED |
| S6-A.13 | S6 App.A-13 | Generate `STATUS.json` from the evidence registry | V: `docs/STATUS.json` is hand-written | EV-07 | FULL | MERGED |
| S6-A.14 | S6 App.A-14 | Split `joil-core` into genome/compiler/evidence/evaluator/search crates | N/A | - | FOLLOW-ON | DEFERRED |
| S6-A.15 | S6 App.A-15 | Two compilers for one genome cannot share a subject ID | N/A | EV-02 | FULL | MERGED |
| S6-A.16 | S6 App.A-16 | Inactive genes canonicalise; active unsupported genes fail | N/A | IR-02 | FULL | MERGED |
| S6-A.17 | S6 App.A-17 | Every boundary face has a semantic origin trace | N/A | IR-11 | FULL | MERGED |
| S6-A.18 | S6 App.A-18 | Trapped cavity found independently by B-rep and voxel audits | N/A | IR-08 | FULL | MERGED |
| S6-A.19 | S6 App.A-19 | Solver failure can never become a performance number | N/A | TR-17 | FULL | MERGED |
| S6-A.20 | S6 App.A-20 | A revoked dependency revokes every downstream claim | N/A | EV-05 | FULL | MERGED |

### 4.7 Findings raised by the DOC lane while verifying the reviews

These are not review P0s, but they change what README and WHITEPAPER may claim now, whatever the code
fixes do. DOC-03 corrects the wording. Code-side owners are in §5.

| ID | Source | Finding | Verified@7353526 | Items | Coverage | Status |
|---|---|---|---|---|---|---|
| DOC-F01 | DOC (relates S1 §3.8) | WHITEPAPER §3.1 says the CUDA kernel "integrates … droplet trajectories in FP64 using a 4th-order Runge–Kutta (`RK4`) kernel with `0.0` CPU/GPU parity error". The candidate kernel is a single FP32 closed-form drift per tensor cell. FP64 RK4 parity applies only to the 10-droplet `--verify-oil03` oracle. | V: `physics/cuda/joil_cuda_eval.cu:253` (FP32, one algebraic step), `:82`/`:335` (RK4 oracle) | DOC-03, IR-10 | FULL | MERGED |
| DOC-F02 | DOC (relates S4 §2.7) | WHITEPAPER and README report "10,000-resample bootstrap" CIs; code runs 2,000 resamples (percentile method) | V: `evolve.rs:1435` | DOC-03 | PARTIAL (wording now; code parity is §5 G-02) | MERGED |
| DOC-F03 | DOC | Family names in WHITEPAPER §2.1 (`baseline-colman-thew`, `quad-spiral-manifold`, `helical-ribbon-core`, `s-cone-fluted-finder`, `double-vortex-skimmer`, `scalloped-parabolic-chamber`), §5.3 table and README (`axisymmetric-stepped`, `twin-tangential-volute`, `quad-entry-involute-head`) do not exist in code | V: `program.rs:1346` `CANONICAL_11_FAMILIES` | DOC-03, DOC-05, DOC-06 | FULL | MERGED |
| DOC-F04 | DOC | `tools/claims_audit.py` numeric scan accepts any token within 0.15% of a hand-maintained `derived_constants` list, so a new number can be "traced" by adding it to the list | V: `tools/claims_audit.py:574` | DOC-04, DOC-08 | FULL | MERGED |
| DOC-F05 | DOC (relates S6 §3.1) | `docs/STATUS.json` asserts `structural_qualification: true` and `flexible_graph_cad_implemented: true` | V: `docs/STATUS.json` | EV-07, DOC-05 | FULL | MERGED |
| DOC-F06 | DOC (relates S1 §3.2, S4 §2.2, S6 App.A-10) | The headline elite-vs-CT-35 CFD comparison (−36.5% Δp) mixes geometry pipelines and domains. CT-35 (`ct35-sst-extended-000`) was meshed from the CAD labelled-surface pipeline with a 0.12 m outlet extension. Every elite case was meshed from Rust `MultiPatchSurface` (6,416 triangles, 48-sector rings, two inlet patches) with `outlet_extension_m: 0.0`. No CT-35 case through the same Rust pipeline exists. The elite cases' `input-manifest.json` files are copies (all declare `case_dir` `runs/rust-cfd-elite-gen024-isl2-ind014` and turbulence `SSG`), so per-run identity is not bound. | V: ignored integrator-owned `runs/*/input-manifest.json` and `runs/ct35-sst-extended-000/mesh-input.json`, read-only at 2026-10-16T05:30Z; `phenotype.rs:611` | DOC-03, EV-12 | PARTIAL (wording and identity now; same-pipeline control is §5 G-09) | MERGED |
| DOC-F07 | DOC (relates S5 P0.1, P0.8) | The 220/220 operator random-walk audit checks each operator output with `canonicalize_and_preflight`. At the audited commit a failed operator silently returned CT-35, which passes, so 220/220 and the "85 cross-family transitions" are upper bounds. The ring mesh is closed by construction for any finite positive `R(θ,z)`, so "0 open edges" tests parameter bounds, not geometry. | V: `program.rs:1756` audit loop; `phenotype.rs` `build_fluid_mesh` ring construction | TR-01, TR-02, TR-14, DOC-03 | FULL | MERGED |
| DOC-F08 | DOC (relates S3 §15.8) | WHITEPAPER §5.3 per-family table (F01–F11) uses three family names that exist in no code or receipt. Its per-family numbers (vertex/triangle counts, exoticness, pitch, porosity) appear in no receipt; they pass the audit only through the `derived_constants` whitelist. | V: `grep` over `docs/results/*.json` and `docs/receipts/*.json` finds no `axisymmetric-stepped`, `twin-tangential-volute` or `quad-entry-involute-head` | DOC-03, DOC-04 | FULL | MERGED |
| DOC-F09 | DOC (relates S3 P0-03) | The CFD GCI script clamps the observed order to [0.5, 4.0], so the reported Δp observed order p = 4.00 is the clamp bound, not a computed order. With R < 0 a standard GCI is not valid per `docs/SCIENCE.md`, so only the three-grid span may be used. | V: `tools/generate_gci_rsm_report_and_figures.py:104` `p_obs = max(0.5, min(p, 4.0))` | TR-07, INT-05, DOC-03 | FULL | MERGED |
| DOC-F10 | DOC (relates S3 P0-11) | WHITEPAPER described the screen's Monte Carlo as "24-sample Latin hypercube" over LPBF bias ±80 µm, roughness Ra 3–18 µm, feed-flow drift ±8 % and split ±0.35 %. At every commit since `5b3b847` the code draws four independent uniform values instead: feed concentration [475, 525] mg/L, feed d50 [35, 41] µm, split ±0.0012 and wall tolerance ±0.05 mm. | V: `evolve.rs:370`–`evolve.rs:373` (same at `5b3b847`, `d1ac285`, `eb39ee2`) | DOC-03 | FULL | MERGED |
| DOC-F11 | DOC (relates S3 §15.8) | README and WHITEPAPER printed Wave-3 values that are not in the committed receipt. Generation-0 KPI `31.838823` (receipt: seed-20261101 value `32.047462`, 10-seed mean `31.645679`); B1/B2 generation-0 `44.782516`/`28.887811` (receipt means `44.786326`/`28.90497`); six label-held-out scores and RMSEs (e.g. `47.484214 ± 4.7779` vs receipt `47.483183 ± 4.683`); "147.8" mean label changes per seed (receipt mean `150.4`). The loose numeric scan passed them because it accepts any receipt number within 0.15 %. | V: `docs/results/wave3-evolution-proof-v1.json` `generation_band_curves/*/0`, `arm_outcomes/*`, `held_out_geometry_generalization` | DOC-03, DOC-04 | FULL | MERGED |
| DOC-F12 | DOC | WHITEPAPER cited a "54 / 540" rejected-cohort audit in a receipt that records 64 / 640, and a regression test `loocv_11family_surrogate_generalization_and_false_omission_audit` that has never existed in the repository history. It also gave MAP-Elites coverage as percentages of 192 cells, while the key allows 11 × 6 × 6 × 5 cells. | V: `docs/results/wave2-multiseed-ablation-and-holdout-v1.json` `rejected_cohort_10pct_audit`; `git log -S` over `crates/` finds no such test; `evolve.rs:589` key bins | DOC-03, DOC-04 | FULL | MERGED |
| DOC-F13 | DOC-07 review IR-R1 (relates S1 §3.2, S5 P0.3, S6 App.A-6) | After IR-01..IR-12 merged, the OpenFOAM writer (`cfd_case.rs:159` → `MultiPatchSurface::from_program`) still writes two undivided inlet ducts for split programs. Enactment and voxel F0 are verified on a separately compiled `IrCompiledMesh`, and the case manifest names that mesh as its geometry kernel. | V: slice test, one closed loop per inlet duct for a 3-branch program; after IR `c21e405` (merged `f49f67e`) three per duct, and `prepare-cfd-case` writes the same STL (`docs/reviews/2026-10-16-DOC07-IR-SUPPORT-REVIEW.md`, Resolution) | DOC-07 | FULL | MERGED |
| DOC-F14 | DOC-07 review IR-R2..IR-R5 | The IR-05 forensic STL tool counts horizontal facets rather than connected paths; the IR-10 counter rename is claimed but not in the code; ribbon/TPMS corrugation is labelled `ConservativeApproximation`; Betti parity compares against a constant. | V: `docs/reviews/2026-10-16-DOC07-IR-SUPPORT-REVIEW.md`, Resolution: path count, relabel, counter key and B-rep-vs-mesh Betti checked after `cab01e8` | DOC-07 | FULL | MERGED |

## 5. Gaps and notes for the planner and integrator

Findings with no owning workplan item. These are proposals only. The planner or integrator assigns them
by adding items, or moves them to workplan §6 with a reason. Requests are posted as `NEED` lines on the board.

| Gap | Finding rows | Proposal | Owner / decision |
|---|---|---|---|
| G-01 | S2-4.09 | F1 DSD bins declare their basis (number/volume/mass) and integrate over bin edges; screening outputs carry the basis | §6 proposed (workplan §5 decision 10) |
| G-02 | S4-2.07, DOC-F02 | Bootstrap: run the preregistered 10,000 resamples or amend the preregistration with a versioned reason; name the method (percentile vs BCa) | TR (ACK 2026-10-16T05:24Z; `n_boot = 10000` since `2f24c18`; published intervals used 2,000) |
| G-03 | S4-2.07 | Implement real Latin hypercube for arm `RandomLhsExotic3d`, or rename the arm "uniform random"; docs follow | TR (ACK 2026-10-16T05:24Z; arm naming with TR-15) |
| G-04 | S4-1.Q | Declare the `round4` quantization as a numeric contract (storage, quantum, rounding mode) | §6 proposed (workplan §5 decision 10); documented in `docs/GENOME.md` §3 |
| G-05 | S3-P0-10 | Rename `exoticness_index` to a name that states it measures gene activation (e.g. `catalog_activation_index`) and keep it out of any novelty claim | §6 proposed for code rename (workplan §5 decision 10); docs relabelled in DOC-03 |
| G-06 | S6-3.9 | Classify tests as software-contract, independent-geometry, numerical-verification, model-validation or scientific-benchmark; report verdicts separately | DOC (docs) + EV (`tools/gen_status.py`); open |
| G-07 | DOC-F01 | Regenerate figure plates whose baked-in titles say "qualification" or "billion trajectories" (Plates 2, 8, 9) | INT (ACK 2026-10-16T05:33Z) |
| G-08 | DOC-F03 | Explorer UI (`apps/explorer/src/`) carries family names and claim text; no lane owns it in this workplan | INT (ACK 2026-10-16T05:33Z; owns `apps/explorer`) |
| G-09 | DOC-F06 | Same-pipeline CT-35 control: run CT-35 through Rust `MultiPatchSurface` at G0/G1/G2 with the same outlet extension as the elites, and re-run the elites with the 0.12 m extension, before any Δp-gain sentence is promoted | INT (ACK 2026-10-16T05:33Z) |
| G-10 | DOC-F13 | Make the CFD writer emit the verified `IrCompiledMesh` triangles, or declare it `LegacyUnenactedCfdSurface` and hard-fail split/skimmer programs. Hold INT-06 until then. | Resolved: IR `c21e405`, merged `f49f67e`; DOC re-sliced the CLI-written STL (DOC-07 review, Resolution) |
| G-11 | DOC-F14 | Remainder after `f49f67e`: the campaign summary still serializes `total_gpu_lagrangian_parcels_integrated`; Betti numbers are checked against an expected formula, not B-rep against mesh. | Resolved: IR `e58f3f3`, merged `cab01e8`; DOC checked (DOC-07 review). Residual: the exact-baseline branch asserts `[1, 0, 1]` |

Notes on assigned items (no new item needed):

- **N-1 (TR-07 scope).** S3 P0-03 asks for manufactured error laws with p = 1.5, 2.0 and 3.2, oscillatory and
  non-monotone negative controls, and near-zero-difference conditioning. TR-07's acceptance names only
  p = 2.000. The clamp to [1, 4] at `structure.rs:232` hides wrong orders, so it should go when the
  division is fixed.
- **N-2 (TR-06 + IR-03).** The silent branch-count coercion exists in two places: Rust `program.rs:329` and
  Python `cad/program/exotic_graph.py:125`. Fixing one leaves the other.
- **N-3 (TR-09).** `evolve.rs:435` computes `qualify_structure(...).ok()`. An `Err` becomes `None`, so a
  failing structure looks like "not evaluated" rather than "failed". EV-03's typed observation should be used.
- **N-4 (DOC-03 timing).** Corrections to existing WHITEPAPER and README text (DOC-F01..F03, Wave-3
- **N-5 (TR, evolve.rs).** The Wave-3 result writer embeds the string "Evaluated with validated F0/F1/P08 3D evaluator" (`evolve.rs:2946`) into the result JSON. Already-written receipts stay as historical records, but new runs (INT-04) should not emit "validated" for a screen.
- **N-6 (scope of P05 oracles).** The `P05` droplet/PBM oracles verify `population.rs`, which the `F1` search screen does not call (`evolve.rs` has no `population::` use). Documentation now says so (DOC-03).
  re-framing, "qualification" → "screen") do not need the code fixes and land first. Name changes in code
  (TR-10, TR-14, IR-05, IR-10) are then mirrored in the docs as each merges.

## 6. Change log

- 2026-10-16T05:32Z: added DOC-F07 (random-walk audit upper bound), DOC-F08 (untraceable per-family table), DOC-F09 (GCI order clamp); DOC-06 done `dfe62c2`.
- 2026-10-16T05:23Z: ledger synced from board (TR-05/06 merged; TR-01/04, EV-01..05, DOC-01/02 done). Fixed the as-of stamp, which had been written ahead of real time.
- 2026-10-16T05:19Z: first version (DOC-02, `2d14371`; `ae22586` added DOC-F06 and G-09). Base `7353526`. All six reviews read in full. 127 findings mapped (121 review findings, 6 DOC-lane findings). Later additions are recorded in the entries above.
