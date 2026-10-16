# joil

Evidence-driven evolutionary design of deoiling hydrocyclone cartridges.

Goal: more compliant treated water per liner at a fixed pressure budget and reject fraction, with performance established through numerical verification and paired physical tests. Explore unconventional internal geometry without rewarding simulation errors.

**Current status: full-liner baseline + promoted exotic 3D elite verified across a 3-grid OpenFOAM 14 spatial refinement sequence (`G0`: `139,981` cells, `G1`: `305,522` cells, `G2`: `552,513` cells) with Roache `GCI` and dual 7-equation `LRR` (linear pressure-strain) and `SSG` (quadratic pressure-strain) Reynolds-Stress Model (`RSM`) cross-checks, alongside the 3-node (`xbabe1`/`xbabe2`/`xbabe3`) Rust/CUDA multi-island evolutionary campaign (`119,235` 3D evaluations, `15.97B` droplet trajectories) and the pre-registered 11-family Wave-2 10-seed `B0`–`B3` ablation (`5,400` 3D evals, Mann–Whitney $U = 100$, $p_{\text{Holm}} = 0.000314$, Cliff's $\delta = 1.000$, `D1`–`D4` held-out duties `PASS`); `F2` full-residence oil fitness and `F4` physical rig validation remain open.** The extended-outlet `CT-35` SST initializer reaches 4 m³/h and 1.986% reject with 29.47 kPa static pressure drop and 33.71 kJ/m³ device mechanical loss, while the promoted 3D `trifurcated-6jet-vortex-head` (`xbabe2-gen043-isl10-ind030`) reaches 4.000 m³/h and 1.968%–1.996% reject at **18.70 kPa static pressure drop (`-36.54%`) and 22.05 kJ/m³ hydraulic loss (`-34.60%`) on the fine 553k-cell `G2` grid**, **18.12 kPa (`-38.51%`) and 21.56 kJ/m³ (`-36.04%`) under 7-equation `LRR` RSM**, and **18.17 kPa (`-38.35%`) and 21.54 kJ/m³ (`-36.09%`) under 7-equation `SSG` RSM** (`0.26%` inter-RSM parity). Full-residence oil closure and independent physical validation gates remain open; these are not qualified industrial performance claims.

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
  - [Animation 1: 3D Helical Vortex Oil-Droplet Separation & 10-Seed QD Illumination (`GIF`)](docs/figures/wave2-vortex-and-qd-illumination.gif)
  - [Animation 2: Phylogenetic 3D Topology Morph (`CT-35` → `4-Jet` → `6-Jet` → `Cylindrical-Gyroid TPMS`, `GIF`)](docs/figures/wave2-topology-morph-and-lineage.gif)
- **[Wave-2 Execution Plan (Trans-Dimensional 3D Genome, Multi-Seed Baselines, GCI/RSM & Animations)](docs/WAVE2_PLAN.md)** · **[Pre-Registration Contract](experiments/wave2-preregistration-v1.json)** · **[10-Seed Ablation Dossier](docs/results/wave2-multiseed-ablation-and-holdout-v1.json)**
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

Evolutionary 3D genome flexibility (`crates/joil-core/src/program.rs`): `DesignProgram::audit_11family_genome_expressiveness(20, 20261116)` draws `N = 220` stratified random 3D genomes (`20` per canonical family across `axisymmetric-stepped`, `twin-tangential-volute`, `spiral-rifled-vortex-head`, `quad-entry-involute-head`, `bifurcated-quad-manifold`, `trifurcated-6jet-vortex-head`, `hybrid-omni-exotic`, `cylindrical-gyroid-coalescer`, `schwarz-p-rectifier-head`, `octa-branch-helical-manifold`, and `counter-fluted-recovery-vortex`) and verifies `220 / 220` (`100.0%`) pass `canonicalize_and_preflight` with watertight 3D manifold meshes (`0` open/non-manifold edges, min LPBF clearance `>= 1.24 mm`, min Jacobian `1.000`) versus `0 / 30` (`0.0%`) on unprojected out-of-bounds negative controls (rendered in [Figure Plate 9](docs/figures/wave2-8-exotic-phenotypes-gallery.png)).

Optional exact CAD, inside a project environment with **Python 3.12** (on macOS, `uv venv --python 3.12 .venv` can create it):

```bash
rtk proxy python3 -m venv .venv
rtk proxy .venv/bin/pip install -r requirements-cad.txt
rtk proxy python3 tools/check.py --cad
```

CAD outputs include canonical mm STEP, mm and metre STL previews, metre glTF, terminal patch definitions and an audit report. Each check run uses a fresh output directory. Labelled conformal CFD tessellation is a later acceptance gate. See the report before using geometry downstream; scaling STEP coordinates without changing declared units is not a unit conversion.

Stack: Rust (`joil-core`, `joil-cli`) compositional 3D geometry/mesh/B-rep compiler, constrained multi-island NSGA-II + 4D MAP-Elites search engine, GP surrogate with Constrained Expected Improvement (`P07`), structural/modal/LPBF qualification (`P08`), PBM & blinded A/B/A power verifiers (`P05`/`P06`); CUDA C++20 3D volumetric SDF & Lagrangian droplet tensor evaluator (`sm_86`/`sm_89`); Python/OpenCASCADE CAD and scientific tools; pinned OpenFOAM Foundation 14 CPU/MPI solver; Vite/TypeScript/React interactive 3D hydrocyclone & droplet-vortex explorer (`apps/explorer`).

No jarambit code or shared environment is modified or required at runtime. Raw supplied material and installation details stay in ignored private storage. No project software license has been selected yet; dependency licenses remain their own.
