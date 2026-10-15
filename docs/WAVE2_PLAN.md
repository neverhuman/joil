# Joil Wave-2 Execution Plan: Trans-Dimensional 3D Topologies, Critique-Proof Multi-Seed Science & Visual/Animation Suite

**Version:** 1.0 (`2026-10-15T21:12:00Z`)  
**Parent Authority:** Subordinate execution plan under [`docs/MASTER_PLAN.md`](MASTER_PLAN.md), [`docs/SCIENCE.md`](SCIENCE.md), [`docs/GENOME.md`](GENOME.md), and [`AGENTS.md`](../AGENTS.md). Does **not** fork or supersede `docs/MASTER_PLAN.md` or relax any frozen requirement in `requirements/RS-joil-research-v1.json`.  
**Active Branch & PR:** `feat/3d-pipeline-openfoam-elite` (**PR #1**: `https://github.com/neverhuman/joil/pull/1`, rebased on `origin/main` @ `59f5009`; do **not** stack a new PR and do **not** edit `.github/workflows/foundation.yml` or runner configurations).

---

## 1. Scope, Ordered Reads & Ownership Boundaries

### 1.1 Scope
Wave 1 proved the end-to-end 3-node (`xbabe1` + `xbabe2` + `xbabe3`) Rust + CUDA C++20 evolutionary pipeline (`119,235` 3D evaluations, `15.97B` droplet trajectories across 7 parameterized 3D families) and verified the promoted #1 elite (`xbabe2-gen043-isl10-ind030`, `trifurcated-6jet-vortex-head`) in 3D OpenFOAM Foundation 14 across a 3-grid spatial refinement sequence (`G0`: `139,981` cells, `G1`: `305,522` cells, `G2`: `552,513` cells) and two 7-equation Reynolds-Stress Transport Model (`LRR` and `SSG` RSM) closures (`-36.54%` to `-38.51%` reduction in $\Delta p_u$, `0.26%` inter-RSM parity).

**Wave 2** advances packets `P02`, `P04`, `P05`, `P07`, `P08`, and `P09` across three primary objectives:
1. **Truly Exotic, Valid-by-Construction 3D Genome (`ct35-exotic-head-v3`)**: Generalize the 7 fixed-template families into a trans-dimensional compositional 3D operator graph supporting hierarchical $2 \to K$ branching/rejoining inlet manifolds ($K \in \{2, 3, 4, 6, 8, 12\}$), variable-twist 3D helical guide-ribbon cascades, spatially graded Triply Periodic Minimal Surface (`TPMS` Gyroid / Schwarz-P / Diamond) peripheral coalescer/rectifier inserts, and non-axisymmetric $C^2$ Fourier-spline swirl chambers, equipped with Jacobian-guarded mutation and homologous graph crossover operators that guarantee watertight, self-intersection-free, `snappyHexMesh`-ready 5-patch manifolds.
2. **Critique-Proof Up-Front Scientific Design**: Pre-register all primary/secondary metrics (`experiments/wave2-preregistration-v1.json`), execute equal-budget comparisons against **`CT-35` (`B0`)**, **pure random/LHS search (`B1`)**, and an **optimized conventional axisymmetric baseline (`B2`)** across **$N_{\text{seeds}} = 10$ independent seeds** with BCa bootstrap 95% confidence intervals, evaluate on **held-out geometry families (`LOFO-CV`)** and **held-out turndown/upset operating duties**, and enforce **automated 3-grid Roache `GCI` (`G0`/`G1`/`G2`) + dual `LRR`/`SSG` RSM verification on every promoted Wave-2 elite**.
3. **Publication-Grade Visuals & Data-Driven 3D Animations**: Generate 6 static 300-DPI multi-panel figure plates and 4 source-bound 60-fps 3D animations / interactive glTF 2.0 field scenes directly from converged OpenFOAM 14 and multi-seed evolutionary trajectories.

### 1.2 Ordered Reads Before Editing
Every agent (`Gemini`, `Codex`, `Claude`) picking up a Wave-2 work item must read in order:
1. `AGENTS.md`, `GEMINI.md`, `COMMS.md`, and root `COORD.md`.
2. `docs/MASTER_PLAN.md`, `docs/SCIENCE.md`, `docs/GENOME.md`, `docs/WHITEPAPER.md`, and this file (`docs/WAVE2_PLAN.md`).
3. `docs/STATUS.json`, `docs/WORK_PACKETS.json`, and `requirements/RS-joil-research-v1.json`.
4. `crates/joil-core/src/{program,phenotype,cfd_case,redteam,structure,search,surrogate,droplet}.rs` and `physics/cuda/joil_cuda_eval.cu`.
5. `docs/results/elite-3grid-gci-and-rsm-verification-v1.json` and `tools/generate_gci_rsm_report_and_figures.py`.

### 1.3 Module & Path Ownership Boundaries
| Subsystem / Packet | Owned Paths | Key Functions & Types |
| :--- | :--- | :--- |
| **`P02` Wave-2 3D Genome & Mesh Compiler** | `crates/joil-core/src/program.rs`, `crates/joil-core/src/phenotype.rs`, `crates/joil-core/src/cfd_case.rs`, `crates/joil-core/src/redteam.rs`, `cad/program/` | `DesignProgram` (`v3`), `InletManifoldGraph3D`, `HelicalRibbonCascade3D`, `TpmsInsertField3D`, `FourierSplineWall3D`, `mutate_diffeomorphic_spline`, `mutate_branch_topology`, `mutate_tpms_grading`, `crossover_homologous_graph`, `compile_5patch_cfd_case` |
| **`P07` Multi-Seed Search, Baselines & Holdouts** | `crates/joil-core/src/search.rs`, `crates/joil-core/src/surrogate.rs`, `crates/joil-core/src/gpu_eval.rs`, `physics/cuda/joil_cuda_eval.cu`, `experiments/wave2-preregistration-v1.json` | `SearchBaselineMode` (`Ct35Fixed`, `RandomLhs`, `ConventionalAxisymmetricNsga2`, `Exotic3dQdCei`), `run_multiseed_benchmark`, `compute_bootstrap_ci`, `audit_rejected_cohort` |
| **`P04`/`P05` Automated Multi-Elite 3-Grid `GCI` & `RSM`** | `physics/openfoam/`, `tools/run_elite_3grid_gci_and_rsm.py`, `tools/generate_gci_rsm_report_and_figures.py`, `analysis/inventory.py` | `run_three_grid_refinement`, `run_dual_rsm_crosscheck` (`LRR` + `SSG`), `compute_roache_gci_triplet`, `verify_droplet_residence_inventory` |
| **`P09` Figures, Animations & Interactive Explorer** | `tools/generate_openfoam14_flowfield_plate.py`, `tools/generate_wave2_figures_and_animations.py`, `apps/explorer/src/`, `docs/figures/` | `render_wave2_figure_plates`, `render_vortex_droplet_animation`, `export_glb_field_bundle` |

---

## 2. Pillar 1 — Wave-2 Trans-Dimensional 3D Genome (`ct35-exotic-head-v3`) & Meshability-Preserving Operators

### 2.1 Immutable Hardware & CFD Interface Envelope (`docs/GENOME.md`)
All Wave-2 geometries strictly preserve the frozen external and downstream interfaces so every candidate is a drop-in cartridge replacement for `CT-35`:
- **Terminal Feed Inlets (`inlet1`, `inlet2`)**: Frozen at $x = \pm 60.0\text{ mm}$, outward normals $\mp\hat{\mathbf{x}}$, each $7.7545\text{ mm} \times 15.5090\text{ mm}$ ($A = 120.264\text{ mm}^2$ per port).
- **Overflow Terminal (`overflow`)**: Frozen at $z = -22.8\text{ mm}$, $D_o = 2.0\text{ mm}$ CFD bore (`1.4 mm` hardware reference).
- **Downstream Collar & Separation Tail (`underflow` + downstream `wall`)**: The final $0.5\,D_2$ reducing-cone collar ($z \in [151.75, 169.25]\text{ mm}$, $D_2 = 35.0\text{ mm}$), the $1.5^\circ$ fine tapered separation cone ($z \in [169.25, 939.25]\text{ mm}$, $D_3 = 17.5\text{ mm}$), and the extended tailpipe ($z \in [939.25, 1856.0]\text{ mm}$) are **100% frozen**.
- **Evolvable Interior Volume**: Only the upstream swirl head, inlet distributor manifold, internal swirl conditioner, vortex-finder sleeve, and reducing cone on $z \in [-20.0, 151.75]\text{ mm}$ evolve.

### 2.2 Five Compositional 3D Operator Layers (`crates/joil-core/src/program.rs`)

```rust
/// Wave-2 Compositional 3D Design Program (`schema_version = 3`).
/// Backward-compatible superset of `ct35-parametric-v1` and `ct35-exotic-head-v2`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Wave2DesignProgram {
    pub schema_version: u32,                // 3
    pub family: String,                     // "ct35-exotic-head-v3"
    pub topology_class: String,             // 11 classes (7 Wave-1 + 4 Wave-2 TPMS/branching classes)
    pub baseline_file_sha256: String,
    pub manifold_graph: InletManifoldGraph3D,
    pub ribbon_cascade: HelicalRibbonCascade3D,
    pub tpms_insert: TpmsInsertField3D,
    pub chamber_wall: FourierSplineWall3D,
    pub vortex_finder: VortexFinder3D,
}
```

1. **Layer A — Hierarchical Multi-Inlet Branching & Rejoining Manifold (`InletManifoldGraph3D`)**:
   - Generalizes `InletRamp3D` + `InletSplit3D` into a directed acyclic channel graph (`nodes: Vec<JunctionNode3D>`, `edges: Vec<BranchSweep3D>`) mapping the two fixed feed ports (`inlet1`, `inlet2`) into $K \in \{2, 3, 4, 6, 8, 12\}$ tangential-helical nozzle entries around the swirl chamber circumference.
   - Each `BranchSweep3D` is parameterized by a $C^2$ Bishop-framed cubic B-spline centerline $\mathbf{C}_k(s) \in \mathbb{R}^3$ ($s \in [0, 1]$) and a cross-sectional superellipse schedule $(a_k(s), b_k(s), n_k(s))$ subject to:
     - **Continuity of Cross-Sectional Area**: At every $1 \to m$ bifurcation node, $\sum_{j=1}^m A_{\text{child},j}(0) = \gamma_{\text{split}}\,A_{\text{parent}}(1)$ with $\gamma_{\text{split}} \in [0.92, 1.08]$ (preventing sudden expansion stalls or cavitation vena-contracta spikes).
     - **Bounded Diffuser Half-Angle**: $\frac{\mathrm{d}\sqrt{A_k(s)}}{\mathrm{d}s} \le \tan(7^\circ)$ everywhere along arc length $s$.
     - **Minimum Inter-Branch Wall Ligament**: Signed distance between any two non-sibling channel surfaces $d_{\text{sep}}(\mathcal{S}_i, \mathcal{S}_j) \ge 1.25\text{ mm}$.

2. **Layer B — Internal Helical Guide-Ribbon & Swirl-Conditioning Cascade (`HelicalRibbonCascade3D`)**:
   - Parameterizes $M \in \{0, 2, 3, 4, 6\}$ internal helical guide vanes (`vane_count: u32`, `axial_span_mm: [f64; 2]`, `radial_span_fraction: [f64; 2]`, `helical_twist_rad: f64`, `camber_mode_rad: [f64; 2]`, `thickness_mm: f64`).
   - Vanes are synthesized as smooth stream-surface shells with rounded leading/trailing edges ($r_{\text{LE}} = 0.5\,t_{\text{vane}} \ge 0.60\text{ mm}$) and $C^1$ root fillets onto the outer chamber wall, eliminating sharp corner vortices.

3. **Layer C — Spatially Graded Lattice / TPMS Peripheral Coalescer & Rectifier (`TpmsInsertField3D`)**:
   - Introduces an optional additively manufacturable Triply Periodic Minimal Surface (`TPMS`) porous shell localized strictly within the outer annular entry/rectification zone $r \in [R_{\text{inner}}(z), R_{\text{wall}}(z)]$, $z \in [z_1, z_2] \subset [15.0, 95.0]\text{ mm}$:
     - **Supported Basis Types (`TpmsBasis`)**:
       - `None` (open bore, recovering Wave-1 topologies),
       - `Gyroid`: $\phi_{\text{G}}(\mathbf{X}) = \sin X \cos Y + \sin Y \cos Z + \sin Z \cos X$,
       - `SchwarzP`: $\phi_{\text{P}}(\mathbf{X}) = \cos X + \cos Y + \cos Z$,
       - `Diamond`: $\phi_{\text{D}}(\mathbf{X}) = \sin X \sin Y \sin Z + \sin X \cos Y \cos Z + \cos X \sin Y \cos Z + \cos X \cos Y \sin Z$,
       - `CylindricalGyroid`: Cylindrical coordinates $(R, \Theta, Z) = (2\pi r / \lambda_r, m_{\text{tpms}}\theta, 2\pi z / \lambda_z)$ so cells align naturally with the swirling flow!
     - **Unobstructed Core Guarantee**: The inner boundary $R_{\text{inner}}(z) \ge 0.58\,R_{\text{wall}}(z)$ is modulated by a smooth $C^2$ radial window $W_{\text{tpms}}(r, z)$ so the central vortex core ($r < 0.58\,R_{\text{wall}}$) is **100% free of lattice struts**, allowing unimpeded reverse-flow oil core migration while the peripheral TPMS shell damps inlet jet turbulence and promotes gentle Stokes coalescence of sub-15 µm droplets.
     - **Printability & Meshability Bounds**: Wavelengths $\lambda_r, \lambda_z \in [6.0, 14.0]\text{ mm}$, sheet thickness $t_{\text{sheet}} \in [1.20, 2.20]\text{ mm}$, pore hydraulic diameter $d_{h,\text{pore}} \ge 3.50\text{ mm}$, and zero closed cavities (verified via 3D 6-connected flood fill on both fluid and solid sub-volumes).

4. **Layer D — Non-Axisymmetric $C^2$ Fourier-Spline Chamber & Inflected $S$-Cone (`FourierSplineWall3D`)**:
   - Parameterizes the chamber and reducing-cone wall radius $r(\theta, z)$ on $z \in [0, 151.75]\text{ mm}$ via:
     $$r(\theta, z) = R_{\text{CT35}}(z) + W_{\text{clamp}}(z)\left[\sum_{k=1}^{4} c_k \sin(k\pi \xi) + \sum_{m=2}^{6} \bigl(a_m(z)\cos(m\theta - \psi_m z) + b_m(z)\sin(m\theta - \psi_m z)\bigr)\right]$$
     where $\xi = (z - z_0)/(151.75 - z_0) \in [0, 1]$ and $W_{\text{clamp}}(\xi) = 16\,\xi^2(1 - \xi)^3$ enforces exact $C^2$ clamping (`value = 0`, `slope = 0`, `curvature = 0`) at the frozen downstream collar $z = 151.75\text{ mm}$.

5. **Layer E — Contoured & Fluted Pressure-Recovery Vortex Finder (`VortexFinder3D`)**:
   - Extends `Skimmer3D` with an internal helical de-swirl recovery vane or fluted outer skirt (`length_mm: [12.0, 48.0]`, `outer_radius_mm: [2.8, 5.8]`, `capture_flutes: 0..8`, `flute_helical_pitch_rad: [-1.2, 1.2]`) while keeping the $z = -22.8\text{ mm}$ terminal face frozen.

### 2.3 Validity- & Meshability-Preserving Genetic Operators (`crates/joil-core/src/program.rs`)

To prevent evolution from wasting compute on non-manifold, self-intersecting, or unmeshable geometries, every genetic operator acts as a **closed endomorphism on the manifold-valid subset**:

| Operator Name | Mathematical Mechanism | Guaranteed Invariants |
| :--- | :--- | :--- |
| **`mutate_diffeomorphic_spline`** | Perturbs Fourier-spline modes $(c_k, a_m, b_m)$ and B-spline control points $\mathbf{P}_i$ along a step vector $\Delta \theta$, then analytically bounds step size $\alpha \in (0, 1]$ such that the cylindrical surface metric Jacobian $\det g(\theta, z) > 0$ and radial slope $|\partial r / \partial z| \le \tan(42^\circ)$ (LPBF overhang limit) hold at all quadrature nodes | Zero surface fold-overs, zero self-intersections, $\le 42^\circ$ self-supporting overhang |
| **`mutate_branch_topology`** | Splits a parent inlet sweep into $(2 \text{ or } 3)$ child sweeps or merges adjacent sibling sweeps using an exact area-preserving Y-junction template with $C^2$ hyperbolic-blended saddle fillets ($r_{\text{fillet}} \ge 1.5\text{ mm}$) | Exact terminal port area conservation ($240.53\text{ mm}^2$), zero razor cusps, minimum ligament $\ge 1.25\text{ mm}$ |
| **`mutate_tpms_grading`** | Mutates `TpmsBasis`, axial/radial cell periods $(\lambda_r, \lambda_z)$, and sheet thickness $t_{\text{sheet}}$ strictly inside the pre-computed bi-continuous percolation window $|\tau(\mathbf{x})| \le \tau_{\max}(\text{Basis}) - \Delta\tau(t_{\min})$ | Single connected fluid domain (`1` component), single connected solid wall (`1` component), zero trapped powder pockets |
| **`crossover_homologous_graph`** | Aligns two parent `Wave2DesignProgram` trees by layer and `FeatureId`; applies Simulated Binary Crossover (`SBX`, $\eta_c = 15$) to homologous continuous genes and swaps complete, interface-compatible layer subtrees (`manifold_graph`, `ribbon_cascade`, `tpms_insert`, `chamber_wall`, `vortex_finder`) | Exact $C^2$ seam continuity across layer boundaries; deterministic SHA-256 lineage tracking |
| **`canonicalize_and_preflight`** | Zeroes all inactive genes (e.g., when `tpms_insert.basis == None` or `manifold_graph.branches_per_port == 1`), rounds floating-point genes to $10^{-6}$, triangulates the 5-patch surface, and verifies `open_boundary_edges == 0 && non_manifold_edges == 0 && min_dihedral_deg >= 22.0` | `100%` `snappyHexMesh` octree snapping yield across `G0`/`G1`/`G2` |

---

## 3. Pillar 2 — Critique-Proof Up-Front Scientific & Statistical Protocol

### 3.1 Pre-Registered Metrics & Decision Rules (`experiments/wave2-preregistration-v1.json`)
Before any Wave-2 evolutionary run starts, the following metrics, constraints, and decision rules are frozen in `experiments/wave2-preregistration-v1.json` (verified by SHA-256 in `tools/check.py`):
1. **Hard Feasibility Constraints (Must Pass at Every Tier)**:
   - `F0` Geometry/LPBF: Watertight 2-manifold (`0` open/non-manifold edges), positive enclosed volume, minimum wall/strut thickness $\ge 1.00\text{ mm}$ ($\ge 1.20\text{ mm}$ for TPMS sheets), overhang $\le 45^\circ$, minimum passage hydraulic diameter $\ge 3.50\text{ mm}$, `0` trapped voids.
   - `P08` Structural/Modal: Ti-6Al-4V ELI Von Mises yield safety factor $\text{SF}_y \ge 2.50$ at $5\times \Delta p_u$ burst duty; modal vortex-precession lock-in ratio $f_{n,1} / f_{\text{vortex}} \ge 3.0$.
   - Hydraulic & Water-Quality Compliance: Reject split $r \in [1.85\%, 2.15\%]$ at target backpressure; 95th-percentile treated-water oil concentration $C_{u,\text{p95}} \le 30.0\text{ mg/L}$ (`20.0 mg/L` stretch gate).
2. **Primary & Secondary Optimization Objectives (Evaluated Only on Feasible Candidates)**:
   - $f_1$ (**Minimize**): 95th-percentile robustness treated-water oil concentration $C_{u,\text{p95}}\text{ [mg/L]}$ across 24-sample Latin Hypercube perturbations (dimensional tolerance $\pm 80\ \mu\text{m}$, roughness $R_a \in [3, 18]\ \mu\text{m}$, flow drift $\pm 8\%$, split drift $\pm 0.35\%$).
   - $f_2$ (**Minimize**): Device specific mechanical energy loss per compliant treated volume $e_{\text{loss}}\text{ [J/m}^3\text{]}$ (and static pressure drop $\Delta p_u\text{ [Pa]}$ at $Q_i = 4.000\text{ m}^3/\text{h}$).
   - $f_3$ (**Maximize** $\leftrightarrow$ **Minimize** $1 - E_{20,\text{p05}}$): 5th-percentile $20\ \mu\text{m}$ grade capture efficiency $E_{\text{red},20,\text{p05}}$.
   - $f_4$ (**Minimize**): Cut size $d_{50}\text{ [}\mu\text{m]}$ derived from the 10-bin coupled Hinze-breakup + trajectory transfer kernel.
   - $f_5$ (**Maximize** $\leftrightarrow$ **Minimize** $-Q_{\text{treated,band}}$): Continuous compliant treated-water capacity $Q_{\text{treated}} = Q_i(1-r)\text{ [m}^3/\text{h]}$ across the intersection of required feed scenarios.

### 3.2 Equal-Budget Baselines & Ablation Matrix (`B0`, `B1`, `B2`, `B3`)
To prove that both **evolutionary search** and **exotic 3D topology** contribute statistically significant gains, Wave 2 runs four strictly controlled comparator arms at **identical per-seed evaluation budgets**:

| Comparator ID | Method Name | Allowed Design Space | Search Mechanism | Per-Seed Budget ($N_{\text{seeds}} = 10$ Seeds Each) | Scientific Purpose |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **`B0`** | `Baseline-CT35` | Fixed `ct35-parametric-v1` (`baselines/ct35/genome.json`) | Deterministic reference (1 geometry) | Full 24-LHS `F1` + 3-grid `G0`/`G1`/`G2` + `LRR`/`SSG` RSM | Historical Colman–Thew datum |
| **`B1`** | `Random-LHS-Exotic3D` | Full 11-family `Wave2DesignProgram` (`v3`) | Pure scrambled Sobol / Latin Hypercube sampling + `F0`/`P08` filter (no mutation/selection) | `25,000` `F1` 3D evals + top `10` promoted to OpenFOAM 14 `F2` per seed | Proves directed evolution significantly outperforms random valid 3D generation |
| **`B2`** | `Conventional-Axisym-NSGA2` | Restricted to axisymmetric 2-port cone/cylinder/finder (`M=0` modes, `K=2` ports, no helical vanes, no TPMS) | Constrained multi-island NSGA-II + GP surrogate | `25,000` `F1` 3D evals + top `10` promoted to OpenFOAM 14 `F2` per seed | Proves exotic 3D topology outperforms the best possible conventional axisymmetric liner |
| **`B3`** | `Exotic3D-QD-CEI` (Ours) | Full 11-family `Wave2DesignProgram` (`v3`: branching, helical, TPMS, Fourier-spline) | Constrained NSGA-II + 4D MAP-Elites + Matérn-5/2 GP `CEI` active learning | `25,000` `F1` 3D evals + top `10` promoted to OpenFOAM 14 `F2` per seed | Full Wave-2 method under test |

### 3.3 Multi-Seed Statistical Protocol ($N_{\text{seeds}} = 10$) & Confidence Intervals
- **Master Seeds**: `20261101` through `20261110` (deterministic `SplitMix64` streams per island and per LHS perturbation draw).
- **Reported Statistical Estimators (`docs/results/wave2-multiseed-statistics-v1.json`)**:
  - Across the $N_{\text{seeds}} = 10$ independent runs of `B1`, `B2`, and `B3`, compute:
    - Sample mean, standard deviation, median, and interquartile range (IQR) of 5-objective Pareto Hypervolume (`HV` relative to `B0` reference point) and each individual objective ($C_{u,\text{p95}}, \Delta p_u, e_{\text{loss}}, d_{50}, Q_{\text{treated}}$).
    - **Bias-Corrected and Accelerated (`BCa`) Bootstrap 95% Confidence Intervals** (`10,000` bootstrap resamples over the 10 seed outcomes).
    - **Paired Wilcoxon Signed-Rank & Mann–Whitney $U$ Tests** (`B3 vs B1` and `B3 vs B2`) with **Holm–Bonferroni step-down family-wise error rate (`FWER`) correction** at $\alpha = 0.025$ (one-sided superiority) and $\alpha = 0.05$ (two-sided), plus non-parametric **Cliff's delta ($\delta_{\text{Cliff}}$)** effect size.

### 3.4 Held-Out Geometry Families & Held-Out Operating Conditions
1. **Held-Out Topology Families (`LOFO-CV` Generalization Gate)**:
   - Hold out two entire Wave-2 topological families (`cylindrical-gyroid-coalescer` and `octa-branch-helical-manifold`) from initial GP surrogate fitting.
   - Require out-of-family Spearman rank correlation $\rho \ge 0.70$, empirical 95% confidence interval coverage $\in [80\%, 100\%]$, and execute a **mandatory 10% random audit of surrogate-rejected / low-ranked candidates** in OpenFOAM 14 to measure and bound the empirical false-omission / rank-reversal rate.
2. **Held-Out Operating Conditions (`OOD Duty Envelope Stress Test`)**:
   - Every elite promoted from nominal screening ($Q_i = 4.00\text{ m}^3/\text{h}$, $C_i = 500\text{ mg/L}$, $d_{32} = 22\ \mu\text{m}$, $\mu_c = 0.89\text{ mPa}\cdot\text{s}$) is evaluated without retuning on **four held-out stress duties**:
     - `D1_turndown`: Low-flow turndown $Q_i = 2.60\text{ m}^3/\text{h}$ ($r = 2.0\%$) — tests whether reduced entry velocity collapses the vortex core.
     - `D2_turnup`: High-flow turnup $Q_i = 5.40\text{ m}^3/\text{h}$ ($r = 2.0\%$) — tests whether elevated entry dissipation triggers Hinze droplet breakup ($d_{\max} < d_{90}$).
     - `D3_sheared_feed`: Upstream choke-valve sheared feed ($d_{32} = 13.5\ \mu\text{m}$, $C_i = 500\text{ mg/L}$).
     - `D4_cold_viscous_slug`: Cold high-water-cut slug ($T = 10^\circ\text{C}$, $\mu_c = 1.31\text{ mPa}\cdot\text{s}$, $C_i = 850\text{ mg/L}$, $\rho_d = 865\text{ kg/m}^3$).

### 3.5 Mandatory 3-Grid Roache `GCI` + Dual `LRR`/`SSG` RSM Gate on Every Promoted Elite
No candidate from any seed or topology family may be listed as a verified Wave-2 elite unless `tools/run_elite_3grid_gci_and_rsm.py` completes and records:
1. **3-Grid OpenFOAM 14 Sequence (`G0 ~140k`, `G1 ~305k`, `G2 ~550k` cells)** with `checkMesh` pass (`0` failed topology checks), water continuity closure $< 10^{-3}$, and matched reject split $r \in [1.90\%, 2.10\%]$.
2. **Roache `GCI` & Signal-to-Span Certification**: Computes $r_{21}, r_{10}, R = e_{21}/e_{10}, p_{\text{obs}}, \text{GCI}_{21}^{\text{std}}$, and full 3-grid extremum span $\Delta f_{\text{span}} = \max_k f_k - \min_k f_k$. Requires signal-to-span ratio $\text{SNR}_{\text{span}} = |\Delta p_{u,\text{CT35}} - \Delta p_{u,G2}| / \Delta f_{\text{span}} \ge 3.0$.
3. **Dual 7-Equation `LRR` + `SSG` RSM Cross-Check on `G1`**: Both linear (`LRR`) and quadratic (`SSG`) pressure-strain Reynolds-Stress Transport closures must converge with `100.0%` diagonal normal-stress realizability ($R_{xx}, R_{yy}, R_{zz} > 0$) and agree within `5.0%` on $\Delta p_u$ and $e_{\text{loss}}$.

---

## 4. Pillar 3 — Planned Publication Figures & High-Resolution Animations

Wave 2 will produce **6 static 300-DPI publication figure plates** (`docs/figures/*.png` + `.manifest.json`) and **4 data-driven 3D animations / interactive scenes** (`docs/media/*.mp4`, `docs/media/*.gif`, and `apps/explorer/public/scenes/*.glb`), all bound by SHA-256 provenance manifests:

### 4.1 Static Publication Figure Plates (`docs/figures/`)
1. **`docs/figures/wave2-exotic-3d-genome-atlas.png`** (*11-Archetype 3D Cutaway & Cross-Section Atlas*):
   - High-resolution 3D rendered cutaways and axial/azimuthal cross-sections of all 11 topology families (`2->4`, `2->6`, `2->8` helical manifolds, internal guide-ribbon cascades, peripheral Cylindrical-Gyroid TPMS coalescer shells, scalloped Görtler-suppression chambers, and fluted pressure-recovery vortex finders) with wall-thickness and LPBF overhang heatmaps.
2. **`docs/figures/wave2-multiseed-ablation-and-ci.png`** (*10-Seed Statistical Benchmark: `B0` vs `B1` vs `B2` vs `B3`*):
   - 4-panel statistical plate showing: (A) Hypervolume convergence trajectories (`mean ± 95% BCa bootstrap CI` across $N=10$ seeds) for `Random-LHS` (`B1`), `Conventional-Axisym-NSGA2` (`B2`), and `Exotic3D-QD-CEI` (`B3`); (B) Raincloud / violin + boxplot distributions of best $\Delta p_u$ and $C_{u,\text{p95}}$ with exact Wilcoxon $p$-values and Cliff's $\delta$; (C) `LOFO-CV` held-out topology calibration parity & 10% rejected-cohort audit; (D) Held-out operating-duty (`D1..D4`) robustness envelope.
3. **`docs/figures/elite-gci-and-rsm-verification.png`** (*Multi-Elite 3-Grid Roache `GCI` & Dual `LRR`/`SSG` RSM Matrix*):
   - Expanded from the Wave-1 single-elite plate to show `G0`/`G1`/`G2` spatial convergence, `LRR` vs `SSG` 7-equation RSM parity, and 305k-cell Lumley anisotropy invariants across the top promoted elites from each winning topology family.
4. **`docs/figures/openfoam14-3d-vortex-physics-comparison.png`** (*3D OpenFOAM 14 Internal Swirl-Head, Centripetal Pressure & Hinze Shear Physics*):
   - Direct cell-by-cell comparison of `CT-35` vs. promoted exotic elites: entry-plane tangential swirl $U_\theta(r)$, radial pressure gradient $\partial p/\partial r = \rho U_\theta^2/r$, axial core depression $p_{\text{core}}(z)$, and Kolmogorov–Hinze maximum stable droplet size $d_{\max}(\varepsilon)$.
5. **`docs/figures/wave2-grade-efficiency-and-dsd-shift.png`** (*10-Bin Coupled Grade-Efficiency Curves $E_{\text{red}}(d)$ & Inlet-to-Outlet DSD Transfer*):
   - Plots size-resolved reduced grade capture $E_{\text{red}}(d)$ across $d \in [3, 100]\ \mu\text{m}$ with and without dynamic Hinze entry shear breakup, demonstrating how multi-branch + TPMS entry conditioning prevents fine-droplet emulsification at high turnup flow ($Q_i = 5.4\text{ m}^3/\text{h}$).
6. **`docs/figures/wave2-structural-lame-and-modal-qualification.png`** (*`P08` 3D Pressure-Mapped Von Mises Stress, LPBF Overhang & Strouhal Lock-In Margin*):
   - Axial/azimuthal contour maps of structural Von Mises stress under $5\times \Delta p_u$ burst load, Euler–Bernoulli / FEA first bending mode shape vs. vortex precession frequency $f_{\text{vortex}}$, and powder-evacuation clearance maps.

### 4.2 Data-Driven 3D Animations & Interactive Scenes (`docs/media/` & `apps/explorer/`)
1. **`docs/media/wave2-vortex-core-and-droplet-separation.mp4` (and `.gif` preview)**:
   - Side-by-side synchronized 60-fps 3D animation (`CT-35` on left, promoted `Exotic3D` elite on right) rendering 5,000 color-coded oil droplet trajectories ($d_p \in \{10, 20, 35, 50\}\ \mu\text{m}$) spiraling inward into the reverse-flow oil core and exiting through the `overflow` vortex finder while treated water exits `underflow`.
2. **`docs/media/wave2-topology-morph-and-lineage.mp4`**:
   - Smooth 3D phylogenetic lineage animation tracing an evolutionary path from `Gen 0` (`baseline-ct35`) $\to$ `Gen 10` (bifurcated spiral ramp) $\to$ `Gen 25` (trifurcated 6-jet + inflected $S$-cone) $\to$ `Gen 45` (graded peripheral Cylindrical-Gyroid TPMS + 6-jet helical head), displaying live `F0`/`P08`/`F1`/`F2` telemetry overlays.
3. **`docs/media/wave2-4d-map-elites-illumination.mp4`**:
   - Time-lapse animation of the 4D MAP-Elites quality-diversity archive illuminating across generations `0..45` across all 10 seeds, alongside the advancing 5-objective Pareto frontier.
4. **Interactive Three.js / WebGL 3D Scene Bundle (`apps/explorer/public/scenes/wave2-elites.json` + `.glb`)**:
   - Upgrades `apps/explorer` so users can toggle between all 11 3D topology families, slice the 3D mesh interactively along $z$ or $\theta$, inspect OpenFOAM 14 `G2` / `LRR` / `SSG` velocity and Reynolds-stress profiles, and play back 3D Lagrangian droplet trajectories in real time.

---

## 5. Worker-Ready Step-by-Step Execution Plan, Edge Cases & Stop Conditions

### 5.1 Ordered Implementation Steps
- **Step `W2-S1` — Pre-Registration & Schema v3 Contract (`crates/joil-core/src/program.rs`, `experiments/wave2-preregistration-v1.json`)**:
  - Add `TpmsInsertField3D`, `TpmsBasis`, and 4 new Wave-2 topology classes (`cylindrical-gyroid-coalescer`, `schwarz-p-rectifier-head`, `octa-branch-helical-manifold`, `counter-fluted-recovery-vortex`) to `crates/joil-core/src/program.rs` and `crates/joil-core/src/phenotype.rs` while keeping `schema_version = 2` and `ct35-parametric-v1` 100% backward-compatible.
  - Implement `mutate_diffeomorphic_spline`, `mutate_branch_topology`, `mutate_tpms_grading`, and `crossover_homologous_graph` with unit tests verifying 1,000 random mutations/crossovers yield `0` open edges, `0` non-manifold edges, positive signed volume, and $\ge 22^\circ$ minimum dihedral angle.
- **Step `W2-S2` — Multi-Seed Comparator Engine (`B0`, `B1`, `B2`, `B3` in `crates/joil-core/src/search.rs`)**:
  - Implement `SearchBaselineMode::{Ct35Fixed, RandomLhs, ConventionalAxisymmetricNsga2, Exotic3dQdCei}` and the $N_{\text{seeds}} = 10$ runner with `BCa` bootstrap 95% confidence intervals, Wilcoxon/Mann–Whitney tests, Cliff's $\delta$, `LOFO-CV` held-out topology validation, and held-out operating duties (`D1..D4`).
- **Step `W2-S3` — Multi-Elite 3-Grid OpenFOAM 14 `GCI` & Dual `LRR`/`SSG` RSM Automation (`tools/run_elite_3grid_gci_and_rsm.py`)**:
  - Generalize our verified OpenFOAM 14 3-grid (`G0`/`G1`/`G2`) + dual `LRR`/`SSG` RSM pipeline so it can be invoked on any promoted elite candidate ID with a single deterministic command, emitting per-elite and comparative JSON dossiers.
- **Step `W2-S4` — Figure Plates (`1..6`) & Animation Generator (`tools/generate_wave2_figures_and_animations.py`)**:
  - Generate the 6 static 300-DPI figure plates and 4 animations / interactive scene bundles with `.manifest.json` SHA-256 sidecars.
- **Step `W2-S5` — White Paper (`docs/WHITEPAPER.md`), `README.md`, & `apps/explorer` Integration**:
  - Update `docs/WHITEPAPER.md` and `README.md` with the Wave-2 multi-seed statistical tables, held-out stress tests, and figure/animation links; verify `PATH="/home/ubuntu/joil/.private/node-runtime/bin:$PATH" python3 tools/check.py` exits `0`.

### 5.2 Numerical Edge Cases & Stop Conditions
1. **TPMS Sub-Voxel Disconnection / Trapped Cavities**: Never trust analytic TPMS inequalities alone near boundary blending zones ($r \approx R_{\text{inner}}(z)$); always run a 3D connected-components flood fill on the triangulated shell and reject any candidate with $> 1$ solid component or $> 1$ fluid component before `snappyHexMesh`.
2. **`snappyHexMesh` Ray-Grazing on Multi-Branch Saddles**: Preserve the verified `0.0031 rad` azimuthal phase offset and `[0.11 mm, 0.13 mm]` off-axis polar apex caps in `crates/joil-core/src/cfd_case.rs` so background hex split planes never graze branch bifurcation saddles.
3. **RSM Positivity & Continuation Stability**: Always initialize `LRR` and `SSG` 7-equation RSM runs from a converged `kOmegaSST` velocity/turbulence field with positive-definite diagonal initialization $R_{ii} = \frac{2}{3}k > 0$ (`tools/generate_gci_rsm_report_and_figures.py`), and stop/quarantine any run where `positive_definite_normal_stress_fraction < 1.0` or water continuity closure exceeds `1e-3`.
4. **Stop Conditions**: Immediately halt a campaign lane if: (a) `baseline_ct35()` round-trip hash changes, (b) `B3` fails to beat `B1` (`Random-LHS`) at $\alpha = 0.025$ across 10 seeds, (c) out-of-family `LOFO-CV` Spearman $\rho < 0.70$, or (d) fine-grid `G2` signal-to-span ratio $\text{SNR}_{\text{span}} < 3.0$.

---

## 6. Explicit Handoff & Finishing Polish Checklist for Codex / Claude

To divide labor cleanly on branch `feat/3d-pipeline-openfoam-elite` (**PR #1**) without merge conflicts or stacked PRs:

1. **Completed & Locked by Gemini (`6d8ef7c` + this commit)**:
   - Wave-1 3-node GPU campaign (`119,235` 3D evals, `15.97B` droplets), 3-grid OpenFOAM 14 Roache `GCI` (`G0`: `140k`, `G1`: `306k`, `G2`: `553k` cells), dual 7-equation `LRR` (`18,119.20 Pa`) and `SSG` (`18,165.71 Pa`, `0.26%` parity) RSM verification on `xbabe2-gen043-isl10-ind030`.
   - 4 publication figure plates (`rust-3d-topology-atlas.png`, `pareto-map-elites-convergence.png`, `elite-gci-and-rsm-verification.png`, `openfoam14-3d-vortex-physics-comparison.png`), `docs/WHITEPAPER.md`, and this worker-ready `docs/WAVE2_PLAN.md` specification.
2. **Recommended Finishing Polish & Next Pickup Tasks for Codex / Claude**:
   - **Task C1 (UI / `apps/explorer` Polish — `P09`)**: Bind `docs/results/elite-3grid-gci-and-rsm-verification-v1.json` and the 4 figure plates into a dedicated **"3-Grid GCI & Dual LRR/SSG RSM Verification"** tab in `apps/explorer/src/App.tsx` so interactive console users can inspect the `G0`/`G1`/`G2` convergence curves and Lumley anisotropy invariants directly in the browser.
   - **Task C2 (Pre-Registration JSON & Unit Tests — `P07`)**: Materialize `experiments/wave2-preregistration-v1.json` from Section 3.1 of this plan and add a schema/hash verification test in `tests/test_evidence.py`.
   - **Task C3 (Wave-2 `TpmsInsertField3D` + Multi-Seed `B1`/`B2`/`B3` Runner — `P02`/`P07`)**: Implement `Step W2-S1` and `Step W2-S2` in `crates/joil-core/src/{program,phenotype,search}.rs` following the exact struct and operator signatures in Section 2.
   - **Verification Command Before Every Commit on `feat/3d-pipeline-openfoam-elite`**:
     ```bash
     PATH="/home/ubuntu/joil/.private/node-runtime/bin:$PATH" python3 tools/check.py
     ```
     (Must exit `0` across all 8 stages: `fmt`, `clippy`, `rust-tests`, `baseline`, `python-tests`, `public-check`, `npm-ci`, `ui-build`).
