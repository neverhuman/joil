# joil

Evidence-driven evolutionary design of deoiling hydrocyclone cartridges.

Goal: more compliant treated water per liner at a fixed pressure budget and reject fraction, with performance established through numerical verification and paired physical tests. Explore unconventional internal geometry without rewarding simulation errors.

**Current status: full-liner baseline + three promoted exotic 3D elites (`Wave-1` 6-jet `xbabe2-gen043-isl10-ind030`, `Wave-2` TPMS gyroid `s20261103-g008-i012`, and `Wave-2` 8-jet `wave2-arch-octa-branch-helical-manifold`) verified across 3-grid OpenFOAM 14 spatial refinement sequences (`G0`: `~140k` cells, `G1`: `~305k` cells, `G2`: `~552k` cells) with Roache `GCI` and dual 7-equation `LRR` (linear pressure-strain) and `SSG` (quadratic pressure-strain) Reynolds-Stress Model (`RSM`) cross-checks (`15` OpenFOAM 14 3D runs), alongside the 3-node (`xbabe1`/`xbabe2`/`xbabe3`) Rust/CUDA multi-island evolutionary campaign (`119,235` 3D evaluations, `15.97B` droplet trajectories) and the pre-registered 11-family Wave-2 10-seed `B0`–`B3` ablation (`5,400` 3D evals, Mann–Whitney $U = 100$, $p_{\text{Holm}} = 0.000314$, Cliff's $\delta = 1.000$, `D1`–`D4` held-out duties `PASS`); `F2-Oil` full-residence ($T \ge 4.0\text{ s}$) oil fitness and `F4` physical rig validation remain open.** The extended-outlet `CT-35` SST initializer reaches 4 m³/h and 1.986% reject with 29.47 kPa static pressure drop and 33.71 kJ/m³ device mechanical loss, while the promoted Wave-1 3D `trifurcated-6jet-vortex-head` (`xbabe2-gen043-isl10-ind030`) reaches 4.000 m³/h and 1.968%–1.996% reject at **18.70 kPa static pressure drop (`-36.54%`) and 22.05 kJ/m³ hydraulic loss (`-34.60%`) on the fine 553k-cell `G2` grid** (`18.12 kPa` / `-38.51%` under `LRR` RSM, `18.17 kPa` / `-38.35%` under `SSG` RSM, `0.26%` inter-RSM parity). The promoted Wave-2 `cylindrical-gyroid-coalescer` (`s20261103-g008-i012`) achieves **18.72 kPa (`-36.46%`) and 22.07 kJ/m³ (`-34.53%`) on `G2`** (`18.25 kPa` `LRR` / `18.15 kPa` `SSG`, `0.50%` inter-RSM parity, $\text{SNR}_{\text{span}} = 4.98\times$), and the Wave-2 `octa-branch-helical-manifold` achieves **18.63 kPa (`-36.78%`) and 21.97 kJ/m³ (`-34.82%`) on `G2`** (`18.33 kPa` `LRR` / `17.96 kPa` `SSG`, `2.02%` inter-RSM parity, $\text{SNR}_{\text{span}} = 4.75\times$). While `F1` vs. `CT-35` dominance survives across all 15 CFD runs, the fine-scale hydraulic ordering among the three elites reverses within `94.97 Pa` (`0.51%`), well inside the `4.57%`–`5.01%` Roache `GCI` band; furthermore, unextended ($L_{\text{ext}} = 0\text{ m}$) elite carriers fail the strict `F2-Oil` `< 1%` overflow reverse-flow gate (`255.8%`), and our Stage-0 10-bin Lagrangian checkpoint (`35.58 s` wall-clock for `0.01 s`, `0` parcel discrepancy) bounds the resumable 8-stage ($8 \times 0.50\text{ s} = 4.0\text{ s}$) full-residence run at `3.95 wall-hours` on 4 CPUs. Full-residence oil closure and independent physical validation gates remain open; these are not qualified industrial performance claims.

- **[Scientific White Paper (Process, Methods, 3D Topologies, GCI, LRR/SSG RSM, Hostile-Referee Audit & Figures)](docs/WHITEPAPER.md)**
  - [Figure Plate 1: 7-Topology Compositional 3D Hydrocyclone Atlas](docs/figures/rust-3d-topology-atlas.png)
  - [Figure Plate 2: Multi-Objective Pareto Front & 4D MAP-Elites Convergence](docs/figures/pareto-map-elites-convergence.png)
  - [Figure Plate 3: 3-Grid Roache `GCI` & Dual 7-Eq. `LRR`/`SSG` RSM Verification](docs/figures/elite-gci-and-rsm-verification.png)
  - [Figure Plate 4: 3D OpenFOAM 14 Internal Swirl-Head & Vortex-Physics Comparison](docs/figures/openfoam14-3d-vortex-physics-comparison.png)
  - [Figure Plate 5: Wave-2 Exotic 3D Genome Atlas (11 Families: TPMS Gyroid, Schwarz-P, Octa-Branch & Counter-Fluted)](docs/figures/wave2-exotic-3d-genome-atlas.png)
  - [Figure Plate 6: Wave-2 10-Seed Statistical Ablation, Bootstrap 95% CIs, Wilcoxon Tests & Held-Out Duties](docs/figures/wave2-multiseed-ablation-and-ci.png)
  - [Figure Plate 7: Wave-2 10-Bin Grade-Efficiency Curves $E_{\text{red}}(d)$ & Inlet-to-Outlet DSD Transfer](docs/figures/wave2-grade-efficiency-and-dsd-shift.png)
  - [Figure Plate 8: Wave-2 `P08` Structural Lamé Stress, Modal Lock-In & LPBF Overhang Qualification](docs/figures/wave2-structural-lame-and-modal-qualification.png)
  - [Figure Plate 9: Top 8 Exotic Valid 3D Phenotypes Gallery & 11-Family Genome Expressiveness Audit (`220/220` Pass)](docs/figures/wave2-8-exotic-phenotypes-gallery.png)
  - [Figure Plate 10: Side-by-Side Wave-1 vs. Wave-2 Elite OpenFOAM 14 CFD Fields & 5-Run GCI/RSM Audit](docs/figures/wave2-vs-wave1-openfoam14-cfd-comparison.png)
  - [Animation 1: 3D Helical Vortex Oil-Droplet Separation & 10-Seed QD Illumination (`GIF`)](docs/figures/wave2-vortex-and-qd-illumination.gif)
  - [Animation 2: Phylogenetic 3D Topology Morph (`CT-35` → `4-Jet` → `6-Jet` → `Cylindrical-Gyroid TPMS`, `GIF`)](docs/figures/wave2-topology-morph-and-lineage.gif)
- **[Wave-2 Execution Plan (Trans-Dimensional 3D Genome, Multi-Seed Baselines, GCI/RSM & Animations)](docs/WAVE2_PLAN.md)** · **[Pre-Registration Contract](experiments/wave2-preregistration-v1.json)** · **[10-Seed Ablation Dossier](docs/results/wave2-multiseed-ablation-and-holdout-v1.json)** · **[Wave-2 5-Run GCI/RSM + F2-Oil Dossier](docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v1.json)**
- [Master implementation plan](docs/MASTER_PLAN.md)
- [Physics, scoring and validation contract](docs/SCIENCE.md)
- [Representation and CAD contract](docs/GENOME.md)
- [Source review and corrections](docs/SOURCE_REVIEW.md)
- [Primary references](docs/REFERENCES.md)
- [Work packets](docs/WORK_PACKETS.json) · [Status](docs/STATUS.json)
- [Compute and installation runbook](docs/COMPUTE.md)
- [Actual baseline KPIs, failures and qualification limits](docs/BASELINE_RUN.md)
- [Reproduce the pinned pilot](docs/PILOT_RUNBOOK.md)

![CT-35 exact CAD cutaway and head fluid domain; geometry only](docs/media/overview.png)

The research baseline is CT-35: the Colman–Thew patent example scaled to D2 = 35 mm, with explicit reconstruction assumptions. It is not an exact commercial liner or a validated experimental apparatus. Geometry dimensions are in mm; solver adapters must convert to SI explicitly.

Quick start — evolutionary flexibility, 10-seed ablation, and verification (verified from a fresh `git clone`; omit `rtk` / `rtk proxy` if `rtk` is not installed):

```bash
rtk cargo test --workspace --locked
rtk cargo test -p joil-core --test regression genome_expressiveness_11family_sampling_and_exotic_3d_gallery_audit -- --nocapture
rtk cargo test -p joil-core --test regression operator_random_walk_20step_11family_watertight_stress_test -- --nocapture
rtk cargo run -p joil-cli -- baseline
rtk cargo run --release -p joil-cli -- export-3d-preview
rtk cargo run --release -p joil-cli -- wave2-benchmark
rtk proxy python3 tools/run_elite_3grid_gci_and_rsm.py --verify-existing
rtk proxy python3 -m unittest discover -s tests -v
rtk proxy python3 tools/check_public.py
rtk proxy python3 tools/claims_audit.py
rtk proxy sh -c 'export PATH="/home/ubuntu/joil/.private/node-runtime/bin:$PATH" && cd apps/explorer && npm ci && npm run build'
rtk proxy python3 tools/check.py
```

Evolutionary 3D genome flexibility (`crates/joil-core/src/program.rs`): `DesignProgram::audit_11family_genome_expressiveness(20, 20261116)` draws `N = 220` stratified random 3D genomes (`20` per canonical family across `axisymmetric-stepped`, `twin-tangential-volute`, `spiral-rifled-vortex-head`, `quad-entry-involute-head`, `bifurcated-quad-manifold`, `trifurcated-6jet-vortex-head`, `hybrid-omni-exotic`, `cylindrical-gyroid-coalescer`, `schwarz-p-rectifier-head`, `octa-branch-helical-manifold`, and `counter-fluted-recovery-vortex`) and verifies `220 / 220` (`100.0%`) pass `canonicalize_and_preflight` with watertight 3D manifold meshes (`0` open/non-manifold edges, min LPBF clearance `>= 1.24 mm`, min Jacobian `1.000`) versus `0 / 30` (`0.0%`) on unprojected out-of-bounds negative controls (rendered in [Figure Plate 9](docs/figures/wave2-8-exotic-phenotypes-gallery.png)). Beyond static sampling, `DesignProgram::audit_11family_operator_random_walk(20, 20261116)` chains `20` consecutive genetic operator mutations and single-point crossovers from each of the `11` canonical families (`220` chained offspring steps, `85` cross-family topological jumps across all `11` families) and verifies `220 / 220` (`100.0%`) intermediate and terminal offspring remain watertight 3D manifolds (`0` open/non-manifold edges, min clearance `>= 1.00 mm`).

Optional exact CAD, inside a project environment with **Python 3.12** (on macOS, `uv venv --python 3.12 .venv` can create it):

```bash
rtk proxy python3 -m venv .venv
rtk proxy .venv/bin/pip install -r requirements-cad.txt
rtk proxy python3 tools/check.py --cad
```

CAD outputs include canonical mm STEP, mm and metre STL previews, metre glTF, terminal patch definitions and an audit report. Each check run uses a fresh output directory. Labelled conformal CFD tessellation is a later acceptance gate. See the report before using geometry downstream; scaling STEP coordinates without changing declared units is not a unit conversion.

Stack: Rust (`joil-core`, `joil-cli`) compositional 3D geometry/mesh/B-rep compiler, constrained multi-island NSGA-II + 4D MAP-Elites search engine, GP surrogate with Constrained Expected Improvement (`P07`), structural/modal/LPBF qualification (`P08`), PBM & blinded A/B/A power verifiers (`P05`/`P06`); CUDA C++20 3D volumetric SDF & Lagrangian droplet tensor evaluator (`sm_86`/`sm_89`); Python/OpenCASCADE CAD and scientific tools; pinned OpenFOAM Foundation 14 CPU/MPI solver; Vite/TypeScript/React interactive 3D hydrocyclone & droplet-vortex explorer (`apps/explorer`).

No jarambit code or shared environment is modified or required at runtime. Raw supplied material and installation details stay in ignored private storage. No project software license has been selected yet; dependency licenses remain their own.
