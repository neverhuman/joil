# joil results and evidence detail

This page holds the detail moved out of the README: what the pipeline is and is not, the results by evidence
tier, the Wave-4 and Wave-3 screen tables, the figure plates and the closed-catalog audits. The
[WHITEPAPER](WHITEPAPER.md) gives the full account. `tools/claims_audit.py` traces every number on this page
to the receipt it cites.

## What this is and is not

Six external reviews ([verbatim inputs](review-inputs/2026-10-15/INDEX.md), [finding-by-finding synthesis](reviews/2026-10-15-SYNTHESIS.md)) found that the earlier README overstated what the evidence shows. The corrections are listed in the WHITEPAPER section "Read this first" ([`docs/WHITEPAPER.md`](WHITEPAPER.md)). Repairs are tracked in [`docs/WORKPLAN-2026-10-15.md`](WORKPLAN-2026-10-15.md).

**What this is.** A research pipeline with these parts:

- a bounded macro genome (`DesignProgram`: seven optional layers, eleven labels assigned from gene activation; see [`docs/GENOME.md`](GENOME.md));
- a reduced-order screen;
- a multi-island constrained NSGA-II + MAP-Elites search;
- OpenFOAM 14 carrier CFD on compiled surfaces;
- receipt and claims-audit tooling.

**What this is not, yet.**

- Not an open-ended invention engine: the genome cannot express structures outside its seven layers ([`docs/design/GENOME_V4.md`](design/GENOME_V4.md) is a proposal).
- Not evidence that any design separates oil better than `CT-35`. Every separation number ($C_{\text{out}}$, grade efficiency, `KPI_score`) comes from a screen driven by gene-derived mechanism descriptors.
- Not physically validated: `F2-Oil` full residence, `F3` and `F4` have not been run.


## Results by evidence tier


- **`F2` carrier CFD on compiled shells (numerical only).** Three-grid OpenFOAM 14 refinement (`G0` `~140k`, `G1` `~305k`, `G2` `~552k` cells) with `LRR`/`SSG` Reynolds-stress cross-checks. At 4 m³/h the recorded `CT-35` case has `29.47 kPa` static pressure drop. The compiled candidate shells reach `18.70 kPa` on `G2` (label `trifurcated-6jet-vortex-head`; `18.12 kPa` `LRR` / `18.17 kPa` `SSG`), `18.72 kPa` (label `cylindrical-gyroid-coalescer`) and `18.63 kPa` (label `octa-branch-helical-manifold`). A `CT-35` case compiled, meshed and solved by the same pipeline gives `18.63 kPa` on `G2` ([`docs/results/wave4-gci-v1.json`](results/wave4-gci-v1.json), workplan INT-06), so these shells show no detectable pressure-drop change; the `29.47 kPa` case came from a different pipeline and outlet extension. The first Wave-4 design simulated with its split inlet ducts was not grid-converged ([WHITEPAPER §4.6](WHITEPAPER.md)). Caveats for these runs:
  - every candidate surface in these runs has `CT-35`'s two inlet ducts (cases built since workplan fix IR-R1 contain the split ducts; one Wave-4 design has since been solved, not grid-converged);
  - branch, ramp and skimmer genes are absent from that CFD geometry;
  - the `CT-35` case was built with a different geometry pipeline and outlet extension.
- **`F1` reduced screen.** `119,235` screen evaluations in a three-node campaign ([`docs/results/rust-evolution-qualification-v1.json`](results/rust-evolution-qualification-v1.json)), plus the 10-seed Wave-2, Wave-3 and Wave-4 search comparisons below. They measure how well each search arm optimizes the screen, not physical performance. On repaired code (Wave-4), uniform random search reaches a higher best screen score than the evolutionary arm `B3`.
- **Software contract.** `220 / 220` closed-catalog validity samples and an operator random walk with `85` label changes ([`docs/receipts/wave2-genome-and-10seed-ablation-20261015.json`](receipts/wave2-genome-and-10seed-ablation-20261015.json)). Both are upper bounds, because failed operators silently returned `CT-35` before workplan item TR-01.


## Wave-4: the same study on repaired operators (`F1` screen, sandbox, workplan INT-04)

Source: [`docs/results/wave4-evolution-proof-v1.json`](results/wave4-evolution-proof-v1.json) (`SHA-256: 2e7192e2aff6ceceadc1f1efc03682ac602d784138cde05fc9dc265efdbd0885`). The design matches Wave-3: `10` seeds, `450` charged proposals per seed and arm, `22,500` screen evaluations and `72` Monte Carlo samples per candidate. It ran after the truth repairs. Every proposal is now charged to the budget, a failed variation no longer becomes `CT-35`, and `B3`'s acquisition no longer picks from extra uncharged proposals. The result reverses the Wave-3 comparison with random search:

| Arm | Gen-0 best `KPI_score` (10-seed mean) | Final best `KPI_score` mean (`95% CI`) | Final population median `KPI_score` (mean) | Best score vs `B3` |
| :--- | :---: | :---: | :---: | :--- |
| `B1` uniform random | `44.490288` | `45.936820` (`[45.615566, 46.265452]`) | `37.983620` | higher: `B3` > `B1` has $p_{\text{Holm}} = 1.0$, Cliff's $\delta = -0.68$ |
| `B3` QD + GP-CEI | `31.008835` | `45.131872` (`[44.778022, 45.452437]`) | `43.628366` | - |
| `A2` no selection | `31.008835` | `42.715589` (`[42.064348, 43.349521]`) | `37.856710` | lower ($p_{\text{Holm}} = 0.0012568$) |
| `B2` axisymmetric NSGA-II | `27.471889` | `36.236410` (`[36.047525, 36.416028]`) | `35.464588` | lower ($p_{\text{Holm}} = 0.0012568$) |
| `A1` crossover only | `31.008835` | `33.956765` (`[33.713883, 34.184385]`) | `33.640806` | lower ($p_{\text{Holm}} = 0.0012568$) |
| `CT-35` reference (one evaluation per seed) | - | `23.931038` | - | - |

Selection beats the no-selection and crossover-only ablations and the axisymmetric baseline. Uniform random search over the same authored space still finds the higher best score; `B3` leads it only on population median. These are prior-laden screen outputs without independent labels. The one Wave-4 design run through CFD is not grid-converged. Details and caveats: [WHITEPAPER §5.7](WHITEPAPER.md).

## Wave-3 Search-Efficiency Study on the `F1` Screen (`n = 10` Seeds, `450` Evals/Seed/Arm, Equal-Budget Baselines & Operator Ablations)

These are screen outputs from the pre-repair run. In it, selection and mutation scored higher than the ablations and than random search; the Wave-4 re-run above did not reproduce the advantage over random search. They do not show physical separation: the screen's descriptors reward the genes the search activates. Failed variations were silently replaced by `CT-35` and not charged to the budget. The Wave-4 re-run above, on repaired operators, supersedes this table. The `HV` column is the largest single dominated box plus an archive bonus, not Pareto hypervolume.

Every numerical entry below is a screen output computed by `joil-cli wave3-proof`. `python3 analysis/kpi_fitness.py --wave3-results docs/results/wave3-evolution-proof-v1.json` recomputes the KPI arithmetic and statistics (a consistency check, not an independent evaluation), and `python3 tools/claims_audit.py` checks them against the receipts:

$$\text{KPI}_{\text{score}}(\mathbf{g}) = \left(30.0 - C_{\text{out,p95}}(\mathbf{g})\right) + 25.0\,E_{\text{reduced},20\mu\text{m},\text{p05}}(\mathbf{g}) + 10.0\,\frac{\Delta p_{u,\text{CT35}} - \Delta p_u(\mathbf{g})}{\Delta p_{u,\text{CT35}}} - 500.0\,V(\mathbf{g})$$

| Comparator / Ablation Arm | Evals / Seed | Gen-0 Best `KPI_score`, 10-Seed Mean `[-]` | Final Best `KPI_score` Mean $\pm$ SD (`95% CI`) `[-]` | Final Pop-Median `KPI_score` Mean (`95% CI`) `[-]` | Final Best $C_{\text{out,p95}}$ Mean (`95% CI`) `[mg/L]` | Final reported `HV` Mean (not Pareto HV) `[-]` | Pairwise Mann–Whitney $U$ vs. `B3` (`Best KPI` / `Pop-Median KPI`) |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **`B0`: `Baseline-CT35`** (Reference) | `1` | `26.486473` | `26.486473` | `26.486473` | `25.101952` | `0.000000` | Baseline (`+20.517319` mean KPI gain in `B3`) |
| **`A1`: `No-Mutation` (`Crossover-Only`)** | `450` | `31.645679` | `35.369726 ± 0.538398` (`[35.039542, 35.670494]`) | `34.918421` (`[34.557810, 35.307194]`) | `17.396069` (`[17.108863, 17.698581]`) | `0.121383` | $U = 100.0$, $p_{\text{Holm}} = 0.0012568$, $\delta = +1.000$ (**PASS**) |
| **`B2`: `Conventional-Axisym-NSGA2`** (2-Port) | `450` | `28.90497` | `37.572998 ± 0.576043` (`[37.250758, 37.927368]`) | `36.146265` (`[35.883722, 36.419626]`) | `15.802162` (`[15.459877, 16.110008]`) | `0.182217` | $U = 100.0$, $p_{\text{Holm}} = 0.0012568$, $\delta = +1.000$ (**PASS**) |
| **`A2`: `No-Selection` (`Random-Walk`)** | `450` | `31.645679` | `43.243541 ± 0.985805` (`[42.669254, 43.812082]`) | `38.341649` (`[36.775733, 39.803955]`) | `10.780283` (`[10.343154, 11.196935]`) | `0.186724` | $U = 100.0$, $p_{\text{Holm}} = 0.0012568$, $\delta = +1.000$ (**PASS**) |
| **`B1`: `Equal-Budget Random Search`** | `450` | `44.786326` | `46.289980 ± 0.515204` (`[46.002233, 46.601384]`) | `38.201035` (`[37.334773, 39.079959]`) | `8.473741` (`[8.210036, 8.727178]`) | `0.255694` | $U = 84.0$ ($p_{\text{Holm}} = 0.010165$, $\delta = +0.680$) / Pop-Med $U = 100.0$ ($\delta = +1.000$) (**PASS**) |
| **`B3`: `Wave3-Exotic3D-QD-CEI` (Full Evolution)** | **`450`** | **`31.645679`** | **`47.003792 ± 0.503020` (`[46.702993, 47.278324]`)** | **`45.956543` (`[45.349348, 46.433642]`)** | **`7.879303` (`[7.667346, 8.124948]`)** | **`0.265579`** | **Top screen elite `s20261110-g024-i005`: `KPI = 47.745321`, $C_{\text{out,p95}} = 7.438085\text{ mg/L}$ (`-70.37%`)** |

## Figure plates and documents

- **[Scientific White Paper (Process, Methods, 3D Topologies, GCI, LRR/SSG RSM, Hostile-Referee Audit & Figures)](WHITEPAPER.md)**
  - [Figure 0 (Hero): Wave-4 Screen Search Diagnostics on Repaired Operators](figures/wave4-readme-hero-kpi-and-elite.png)
  - [Figure Plate 0 (historical, pre-repair): Wave-3 Per-Generation Screen KPI Trajectory (`n = 10` Seeds) & 15-Run OpenFOAM 14 Carrier CFD](figures/wave3-readme-hero-kpi-and-elite.png)
  - [Figure Plate 1: 7-Topology Compositional 3D Hydrocyclone Atlas](figures/rust-3d-topology-atlas.png)
  - [Figure Plate 2: Multi-Objective Pareto Front & 4D MAP-Elites Convergence](figures/pareto-map-elites-convergence.png)
  - [Figure Plate 3: 3-Grid Roache `GCI` & Dual 7-Eq. `LRR`/`SSG` RSM Verification](figures/elite-gci-and-rsm-verification.png)
  - [Figure Plate 4: 3D OpenFOAM 14 Internal Swirl-Head & Vortex-Physics Comparison](figures/openfoam14-3d-vortex-physics-comparison.png)
  - [Figure Plate 5: Wave-2 Exotic 3D Genome Atlas (11 Families: TPMS Gyroid, Schwarz-P, Octa-Branch & Counter-Fluted)](figures/wave2-exotic-3d-genome-atlas.png)
  - [Figure Plate 6: Wave-2 10-Seed Statistical Ablation, Bootstrap 95% CIs, Wilcoxon Tests & Held-Out Duties](figures/wave2-multiseed-ablation-and-ci.png)
  - [Figure Plate 7: Wave-2 10-Bin Grade-Efficiency Curves $E_{\text{red}}(d)$ & Inlet-to-Outlet DSD Transfer](figures/wave2-grade-efficiency-and-dsd-shift.png)
  - [Figure Plate 8: Wave-2 `P08` Analytic Structural Screen: Lamé Stress, Modal Ratio & LPBF Overhang Estimate (not FEA)](figures/wave2-structural-lame-and-modal-qualification.png)
  - [Figure Plate 9: Gallery of 8 Rendered Genomes & Closed-Catalog Validity Audit (`220/220`; illustrative renderings)](figures/wave2-8-exotic-phenotypes-gallery.png)
  - [Figure Plate 10: Side-by-Side Wave-1 vs. Wave-2 Elite OpenFOAM 14 CFD Fields & 5-Run GCI/RSM Audit](figures/wave2-vs-wave1-openfoam14-cfd-comparison.png)
  - [Figure Plate 11 (historical, pre-repair): Wave-3 6-Arm Screen Comparison (`n = 10` Seeds), Lineage, Label Diversity & Label-Held-Out Check](figures/wave3-evolution-ablation-and-lineage-proof.png)
  - [Figure Plate 11 (Wave-4): Screen Comparison on Repaired Operators, Incumbent-Improvement Milestones, Label Diversity & Per-Label Screen Summary](figures/wave4-evolution-ablation-and-lineage-proof.png)
  - [Animation 1: Illustrative Vortex Rendering & 10-Seed QD Illumination (not simulated trajectories; `GIF`)](figures/wave2-vortex-and-qd-illumination.gif)
  - [Animation 2: Illustrative Label Lineage Morph (`CT-35` → `4-Jet` → `6-Jet` → `Cylindrical-Gyroid TPMS` labels; renderings, `GIF`)](figures/wave2-topology-morph-and-lineage.gif)
- **[Wave-3 Search-Efficiency Dossier (`F1` screen)](results/wave3-evolution-proof-v1.json)** · **[Wave-2 Execution Plan](WAVE2_PLAN.md)** · **[Pre-Registration Contract](../experiments/wave2-preregistration-v1.json)** · **[10-Seed Ablation Dossier](results/wave2-multiseed-ablation-and-holdout-v1.json)** · **[Wave-2 5-Run GCI/RSM + F2-Oil Dossier](results/wave2-tpms-elite-gci-rsm-and-f2-oil-v1.json)**
- [Master implementation plan](MASTER_PLAN.md)
- [Physics, scoring and validation contract](SCIENCE.md)
- [Representation and CAD contract](GENOME.md)
- [Source review and corrections](SOURCE_REVIEW.md)
- [Primary references](REFERENCES.md)
- [Work packets](WORK_PACKETS.json) · [Status](STATUS.json)
- [Compute and installation runbook](COMPUTE.md)
- [Actual baseline KPIs, failures and qualification limits](BASELINE_RUN.md)
- [Reproduce the pinned pilot](PILOT_RUNBOOK.md)

## The CT-35 research baseline

![CT-35 exact CAD cutaway and head fluid domain; geometry only](media/overview.png)

The research baseline is CT-35: the Colman–Thew patent example scaled to D2 = 35 mm ([`baselines/ct35/genome.json`](../baselines/ct35/genome.json)), with explicit reconstruction assumptions. It is not an exact commercial liner or a validated experimental apparatus. Geometry dimensions are in mm; solver adapters must convert to SI explicitly.

## Closed-catalog validity audits

Closed-catalog validity audits (`crates/joil-core/src/program.rs`; software contract, not open-ended expressiveness). `DesignProgram::audit_11family_genome_expressiveness` (since renamed `closed_catalog_validity`; the historical seed is not recorded, and the current regression calls `closed_catalog_validity(20, 20261115)`) draws `N = 220` samples (`20` per label across the eleven labels listed in [`docs/GENOME.md`](GENOME.md) §4) and finds `220 / 220` (`100.0%`) pass `canonicalize_and_preflight`, with closed ring surfaces (`0` open/non-manifold edges). The receipt records a `0.0%` pass rate for out-of-bounds negative controls, but no sample count; the current regression codes three such controls, and all fail parameter bounds. `DesignProgram::audit_11family_operator_random_walk` (since renamed `operator_chain_validity`; the historical seed is not recorded, and the current regression calls `operator_chain_validity(20, 20261116)`) chains `20` operator steps from each of the `11` labels (`220` steps, `85` label changes) and finds `220 / 220` (`100.0%`) passing steps. Because the ring surface is closed by construction and failed operators silently returned `CT-35` before TR-01, these counts are upper bounds on validity and say nothing about new topology (renderings in [Figure Plate 9](figures/wave2-8-exotic-phenotypes-gallery.png)).

## Software stack

Stack: Rust (`joil-core`, `joil-cli`) macro-genome ring-surface compiler, constrained multi-island NSGA-II + 4D MAP-Elites search engine, GP surrogate with Constrained Expected Improvement (`P07`), analytic structural screen (`P08`, not FEA), PBM & blinded A/B/A power-calculation software (`P05`/`P06`, synthetic oracles only); CUDA C++20 closed-form throughput evaluator (`sm_86`/`sm_89`); Python/OpenCASCADE CAD (schema v2 only) and scientific tools; pinned OpenFOAM Foundation 14 CPU/MPI solver; Vite/TypeScript/React research explorer (`apps/explorer`; renders gene values and screen outputs).
