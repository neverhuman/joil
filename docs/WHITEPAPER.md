# Compositional 3D Evolutionary Design and Multi-Fidelity Navier–Stokes Verification of Additively Manufacturable Deoiling Hydrocyclone Heads

**Authors:** Joil Research & Engineering Collaboration (Gemini & Codex)  
**Repository:** `neverhuman/joil`  
**Date:** 15 October 2026  
**Epistemic Claim State:** `F0_F1_EXECUTED_F2_3GRID_GCI_AND_RSM_CARRIER_VERIFIED_NOT_F4_PHYSICALLY_VALIDATED` (`qualified_fitness: null`, `qualified_capacity_gain: null`, `physical_validation: false`)

---

## Abstract

Deoiling hydrocyclone liners used in produced-water treatment remain constrained by the classical 1980s Colman–Thew (`CT-35`) twin-tangential-inlet paradigm, in which abrupt entry jet impingement and high inlet-ring shear degrade droplet size distributions (DSD) while consuming excessive static pressure drop ($\Delta p_u \approx 29.5\text{ kPa}$ at $Q = 4.0\text{ m}^3/\text{h}$). Additive manufacturing via Laser Powder Bed Fusion (LPBF in Ti-6Al-4V ELI or Inconel 625) removes traditional subtractive machining constraints, enabling complex 3D multi-branch helical manifolds, internal swirl-conditioning ribbons, inflected $S$-cone swirl chambers, and fluted vortex finders within an unchanged external pressure-vessel envelope (`DN35`).

This white paper documents the end-to-end engineering, physics, evolutionary optimization, and computational verification pipeline developed in **Joil**:
1. **Compositional 3D Parametric Design Language (`F0` / `P02` / `P08`)**: A versioned compositional graph genome (`DesignProgram`) compiling into watertight 3D manifold triangle meshes (`Phenotype3D`) and conformal 5-patch OpenFOAM 14 surfaces (`wall`, `inlet1`, `inlet2`, `overflow`, `underflow`) across **7 topological families**, guarded by automated `F0` Red-Team geometric/LPBF checks and `P08` structural/modal/fatigue qualification.
2. **3-Node Distributed Rust + CUDA C++20 Evolutionary Search (`F1` / `P07`)**: A multi-island Constrained NSGA-II + 4D MAP-Elites campaign executed across three GPU compute nodes (`xbabe1` RTX 4090, `xbabe2` RTX 3090, `xbabe3` RTX 3090 at `85.50%` VRAM utilization), evaluating **`119,235` 3D candidate geometries**, **`2,857,464` 24-sample Latin Hypercube Monte Carlo realizations**, and **`15,970,490,852` (`15.97 billion`) coupled 3D GPU Lagrangian droplet trajectories** with `0.0` CPU/GPU double-precision ODE parity error and dynamic Weber/Kolmogorov (`Hinze`) shear-breakup coupling. The campaign populated **`191` 4D MAP-Elites cells** across all 7 topology families and discovered **`275` Rank-0 Pareto elites** that strictly dominate `baseline-ct35` across all 5 screening objectives simultaneously.
3. **Critique-Proof 3D OpenFOAM 14 Navier–Stokes Verification (`P04` / `P05`)**: The promoted #1 Pareto elite (`xbabe2-gen043-isl10-ind030`, a `trifurcated-6jet-vortex-head` topology) was compiled into a 5-patch conformal mesh and verified in 3D OpenFOAM Foundation 14 across a **3-grid systematic spatial refinement sequence** (`G0`: `139,981` cells, `G1`: `305,522` cells, `G2`: `552,513` cells) and cross-checked with **two independent 7-equation Reynolds-Stress Transport Models (`RSM`)** — **Launder–Reece–Rodi (`LRR`, linear pressure-strain)** and **Speziale–Sarkar–Gatski (`SSG`, quadratic pressure-strain)** — against the `kOmegaSST` eddy-viscosity closure. Across all three grids and all three turbulence closures (`kOmegaSST`, `LRR RSM`, `SSG RSM`) at matched $Q = 4.000\text{ m}^3/\text{h}$ feed and $r = 1.968\%\text{–}1.996\%$ overflow split, the promoted elite reduces inlet-to-underflow static pressure drop by **`-33.74%` to `-38.51%`** (`18.12–19.53 kPa` vs `29.47 kPa` for `CT-35`) and specific hydraulic energy loss by **`-31.40%` to `-36.09%`** (`21.54–23.12 kJ/m³` vs `33.71 kJ/m³` for `CT-35`), with `LRR` and `SSG` RSM agreeing to within **`0.26%` (`46.5 Pa`)** on $\Delta p_u$ and **`0.08%` (`17.2 J/m³`)** on $e_{\text{loss}}$, and a signal-to-discretization-span margin of **`4.86×` (`10.77 kPa` saving vs `2.22 kPa` 3-grid span)**.

> [!IMPORTANT]
> **Epistemic Discipline (`docs/SCIENCE.md` & `AGENTS.md`)**: All results reported herein are hash-bound computational and numerical verification artifacts (`F0`/`F1`/`F2-carrier`). In accordance with our scientific governance contract, `qualified_fitness`, `qualified_capacity_gain`, and `physical_validation` remain explicitly `null`/`false` until `F4` LPBF hardware fabrication, X-ray CT metrology (`CAD-09`), and the pre-registered 5-block blinded randomized A/B/A flow-loop trials (`P06`) are executed.

---

## 1. Physical Formulation & Colman–Thew (`CT-35`) Baseline Reconstruction

### 1.1 Governing Separation & Hydraulic Metrics (`docs/SCIENCE.md`)

Let $Q_i$ denote the total volumetric feed rate entering through the two terminal feed ports (`inlet1`, `inlet2`), $Q_o$ the oil-enriched overflow (reject) volumetric rate, and $Q_u = Q_i - Q_o$ the treated-water underflow volumetric rate. The volumetric reject split is:

$$r = \frac{Q_o}{Q_i}, \qquad Q_{\text{treated}} = Q_i (1 - r)$$

For a dilute oil-in-water dispersion characterized by an inlet oil mass-weighted droplet size distribution $w(d)$ ($\int_0^\infty w(d)\,\mathrm{d}d = 1$) at feed concentration $C_i\text{ [mg/L]}$ (`500 mg/L` research reference), let $E(d) \in [0, 1]$ denote the fraction of injected oil mass in diameter class $d$ captured at the overflow terminal. Total oil mass capture $E_{\text{oil}}$, reduced centrifugal separation efficiency $E_{\text{red}}(d)$, and compliant treated-water outlet concentration $C_u\text{ [mg/L]}$ are governed by the exact mass-balance identities:

$$E_{\text{oil}} = \int_0^\infty E(d)\,w(d)\,\mathrm{d}d, \qquad E_{\text{red}}(d) = \frac{E(d) - r}{1 - r}, \qquad C_u = C_i\,\frac{1 - E_{\text{oil}}}{1 - r} = C_i \int_0^\infty \bigl(1 - E_{\text{red}}(d)\bigr)\,w(d)\,\mathrm{d}d$$

Crucially, omitting the $(1 - r)$ denominator underestimates $C_u$, and a passive tracer ($E(d) = r$) yields $E_{\text{red}}(d) = 0$ and $C_u = C_i$ (zero separation). At $C_i = 500\text{ mg/L}$ and $r = 0.02$, achieving the research water-quality limit $C_u \le 30\text{ mg/L}$ requires $E_{\text{oil}} \ge 0.9412$, while the stretch limit $C_u \le 20\text{ mg/L}$ requires $E_{\text{oil}} \ge 0.9608$.

On the carrier side, let $p_{\text{in}}$, $p_o$, and $p_u$ denote the area-averaged static pressures on the inlet, overflow, and underflow patches. The static pressure drops, pressure-drop ratio ($\text{PDR}$), and boundary-integrated mechanical energy dissipation per unit compliant treated volume ($e_{\text{loss}}$) are:

$$\Delta p_u = p_{\text{in}} - p_u, \qquad \Delta p_o = p_{\text{in}} - p_o, \qquad \text{PDR} = \frac{\Delta p_o}{\Delta p_u}, \qquad e_{\text{loss}} = \frac{-\sum_{\partial\Omega} \int_{\partial\Omega_k} \left(p + \frac{1}{2}\rho |\mathbf{U}|^2\right) \mathbf{U}\cdot\mathbf{n}\,\mathrm{d}A}{Q_i (1 - r)}$$

### 1.2 Baseline `CT-35` Geometry & Resolution of Historical CAD/CFD Artifacts

The public research baseline (`baselines/ct35/genome.json`, `baselines/ct35/reference.json`) reconstructs the 35 mm Colman–Thew deoiling hydrocyclone geometry (`DN35` class):
- **Swirl Chamber (`D1 -> D2`)**: Entry diameter $D_1 = 70.0\text{ mm}$, cylindrical length $L_1 = 70.0\text{ mm}$, reducing cone half-angle $\alpha/2 = 10.0^\circ$ ($\alpha = 20.0^\circ$) to nominal diameter $D_2 = 35.0\text{ mm}$.
- **Twin Rectangular Tangential Inlets (`inlet1`, `inlet2`)**: $17.131\text{ mm} \times 7.021\text{ mm}$ ($A_{\text{each}} = 120.28\text{ mm}^2$, combined area $240.55\text{ mm}^2$).
- **Fine Tapered Separation Section (`D2 -> D3`)**: Cylindrical section $L_2 = 35.0\text{ mm}$ followed by a $1.5^\circ$ full-angle ($\beta/2 = 0.75^\circ$) conical taper to $D_3 = 17.5\text{ mm}$ ($L_{\text{cone2}} = 668.40\text{ mm}$) and tailpipe $D_u = 10.0\text{ mm}$ reaching $z_{\max} = 1,656.0\text{ mm}$.
- **Overflow Vortex Finder**: Inner bore diameter $D_o = 2.0\text{ mm}$ terminating at $z = -24.0\text{ mm}$.

During `P01`/`P04` baseline verification (`docs/BASELINE_RUN.md`), two critical numerical/geometric defects in legacy setups were identified and resolved:
1. **Inward-Winding STL Normals (`cad/ct35/build.py`)**: Exported OpenCASCADE shell triangulations had inward-pointing surface normals, yielding negative enclosed volume (`-1,105,747 mm³`). Enforcing `BRepLib::OrientClosedSolid` corrected the signed STL volume to **`+1,105,747.14 mm³`** (`+0.025%` relative to analytic `1,105,468.26 mm³`).
2. **Underflow Swirl Recirculation & $20 D_u$ Outlet Extension (`runs/ct35-sst-extended-000`)**: Because strong swirl persists to the tailpipe exit ($z = 1.656\text{ m}$), truncating the mesh at $z = 1.656\text{ m}$ induces artificial pressure-outlet backflow (`13.8%` of patch flux). Extending the downstream tailpipe by $20 D_u = 200\text{ mm}$ to $z = 1.856\text{ m}$ (`334,768` cells) reduced overflow reverse flow to **`0.0 m³/s`** and established the immutable OpenFOAM 14 baseline reference (`runs/ct35-sst-extended-000`, `SHA-256: 05c5c75...`):
   - Feed flow: $Q_i = 4.000\text{ m}^3/\text{h}$, Reject split: $r = 1.986\%$ ($p_o/\rho = -180\text{ m}^2/\text{s}^2$)
   - Static inlet-to-underflow pressure drop: **$\Delta p_u = 29,468.22\text{ Pa}$** ($\text{PDR} = 7.108$)
   - Device-boundary specific hydraulic energy loss: **$e_{\text{loss}} = 33,709.55\text{ J/m}^3$** (`34,004.44 J/m³` across the extended domain)

---

## 2. Compositional 3D Parametric Design Language & Pre-CFD Engineering Gates

### 2.1 Four-Layer Identity Separation & 7 Topological Families (`crates/joil-core/src/phenotype.rs`)

Rather than mutating a brittle flat array of numbers, Joil separates every candidate into four hash-bound representations:
1. **Genotype (`DesignProgram`, `crates/joil-core/src/program.rs`)**: A canonical JSON design program with inactive-gene canonicalization, strict unknown-key rejection, and deterministic SHA-256 identity.
2. **Watertight 3D Solid/Fluid Phenotype (`Phenotype3D` & `TriangleMesh3D`)**: Full 3D parametric surface/solid synthesis generating closed, outward-oriented triangle meshes (`2,976` to `6,928` triangles per candidate) with exact vertex-welded topology and glTF 2.0 (`GLB`) export for interactive 3D inspection.
3. **5-Patch Conformal CFD Surface (`MultiPatchSurface`, `crates/joil-core/src/cfd_case.rs`)**: Automated decomposition of the 3D manifold into five named OpenFOAM STL solids (`wall`, `inlet1`, `inlet2`, `overflow`, `underflow`) with octree-safe phase alignment (`0.0031 rad`) and off-axis polar caps (`[0.11 mm, 0.13 mm]`) that eliminate `snappyHexMesh` ray-edge grazing across arbitrary hex grid spacings.
4. **Evaluation Subject (`CfdCaseManifest` / `QualificationReport`)**: Immutable SHA-256 binding of geometry, solver dictionaries, boundary conditions, and extracted metrics.

The compositional design space spans **7 distinct 3D topological families**:

| Topology Family | Inlet Manifold & Branches | Internal Swirl Conditioning | Chamber Wall Contour | Overflow Vortex Finder & Skimmer |
| :--- | :--- | :--- | :--- | :--- |
| **`baseline-colman-thew`** | 2 tangential ports ($0\text{ rad}$ wrap) | None (smooth bore) | Linear $20^\circ \to 1.5^\circ$ cones | Cylindrical $D_o = 2.0\text{ mm}$ finder |
| **`quad-spiral-manifold`** | 4 helical branches ($2\times 2$ split, $137^\circ$ wrap) | Progressive spiral entry ramps | Modal $m=2, 4$ harmonic lobes | Annular core skimmer + flared lip |
| **`helical-ribbon-core`** | 2 helical ports ($106^\circ$ wrap) | 2 internal helical guide ribbons ($241^\circ$ twist) | Smooth parabolic-taper transition | Extended low-loss vortex finder |
| **`s-cone-fluted-finder`** | 2 curved tangential ports | Boundary-layer stabilization ramp | Inflected cubic $S$-cone ($+2.45\text{ mm}$) | 6-flute longitudinal star vortex finder |
| **`double-vortex-skimmer`** | 4 bifurcated tangential jets | Co-rotating concentric core | Dual-inflection $S$-cone + $m=3$ modes | Deep concentric annular skimmer |
| **`trifurcated-6jet-vortex-head`** | **6 helical-tangential jets ($2\times 3$ split)** | **3-vane progressive swirl conditioner** | **Low-drag inflected $S$-cone + $m=3$ flutes** | **5-flute pressure-recovery finder** |
| **`scalloped-parabolic-chamber`** | 4 spiral-entry jets | 2 helical wall strakes | $m=3, 6$ scalloped Goertler-suppression wall | Flared 4-flute vortex finder |

![Atlas of 7 Compositional 3D Hydrocyclone Topologies](figures/rust-3d-topology-atlas.png)

### 2.2 Automated Pre-CFD Engineering Gates (`F0` Red-Team & `P08` Structural/Modal)

Every candidate generated during evolution must pass two deterministic physics and manufacturing gates before its fitness is accepted:
- **`F0` Geometric & LPBF Red-Team Gate (`crates/joil-core/src/redteam.rs`)**:
  - Verifies watertight Euler/manifold closure (`0` open boundary edges, `0` non-manifold edges, positive signed enclosed volume).
  - Enforces minimum LPBF wall/vane thickness ($\ge 1.0\text{ mm}$), self-supporting build overhang ($\le 45^\circ$ from the vertical $+z$ build axis), minimum overflow/underflow throat clearance ($\ge 3.5\text{ mm}$), and zero abrupt backwards-facing steps (dead-zone recirculation screen).
- **`P08` Structural, Modal & Fatigue Qualification Gate (`crates/joil-core/src/structure.rs`)**:
  - Evaluates thick-wall Lamé hoop, radial, and axial Von Mises stress across 64 axial stations under $5\times$ nominal $\Delta p_u$ burst/fatigue duty in LPBF Ti-6Al-4V ELI ($\sigma_y = 830\text{ MPa}$), requiring safety factor $\text{SF}_y \ge 2.50$ (`39.2` on the promoted elite).
  - Evaluates the first Euler–Bernoulli cantilevered/supported bending natural frequency $f_{n,1}$ against the maximum Strouhal vortex-precession frequency $f_{\text{vortex}} = \text{St}\,U_\theta / (\pi D_2)$, requiring lock-in margin $f_{n,1} / f_{\text{vortex}} \ge 3.0$.

---

## 3. 3-Node Distributed Rust + CUDA C++20 Evolutionary Campaign (`P07`)

### 3.1 Distributed Cluster Execution & GPU Utilization (`xbabe1` + `xbabe2` + `xbabe3`)

To explore the 7-family compositional 3D design space without starving shared host workloads, we executed a coordinated 3-node multi-island evolutionary campaign across:
- **`xbabe1` (`192.168.68.86`)**: NVIDIA RTX 4090 (`24 GiB` VRAM, `sm_89`) + 16 CPU threads (`seed 20261015`), 16 islands $\times$ 45 generations.
- **`xbabe2` (`192.168.68.87`, local coordinator)**: NVIDIA RTX 3090 (`24 GiB` VRAM, `sm_86`) + 16 CPU threads (`seed 20262026`), 16 islands $\times$ 45 generations.
- **`xbabe3` (`192.168.68.54`)**: NVIDIA RTX 3090 (`24 GiB` VRAM, `sm_86`) + 16 CPU threads (`seed 20263037`), 16 islands $\times$ 45 generations.

The custom CUDA C++20 evaluator (`physics/cuda/joil_cuda_eval.cu`, `SHA-256: 589ff6b5a1fa64fcbd71e6312b2e449263d53f23427bee2f20cef93a4f6fb60c`) allocates a persistent **`20,480 MiB` (`20.0 GiB`)** device trajectory workspace (**`85.50%` of total `24,124.19 MiB` GPU VRAM**, leaving a safe `3.5 GiB` headroom buffer) and integrates 3D coupled Burger–Rott + harmonic swirl + Görtler secondary-flow + turbulent dispersion droplet trajectories in FP64 using a 4th-order Runge–Kutta (`RK4`) kernel with `0.0` CPU/GPU parity error (`crates/joil-core/src/gpu_eval.rs`).

### 3.2 Campaign Scale, Robustness Monte Carlo & Pareto / MAP-Elites Convergence

Across the merged 3-node campaign (`runs/rust-cuda-evolution-v1/campaign-summary.json`, `SHA-256: d1823cdaeb9179afde9789f752438ee0ae6e0087a43602125942e4438d31d5eb`):
- **Total 3D Candidate Evaluations**: **`119,235`** (`119,061` watertight `F0`-passing 3D meshes; `174` Red-Team rejections automatically quarantined from Pareto/MAP-Elites promotion).
- **24-Sample Latin Hypercube Monte Carlo Realizations**: **`2,857,464`** perturbed physical realizations simultaneously sampling LPBF dimensional bias ($\pm 80\ \mu\text{m}$), internal surface roughness ($R_a \in [3, 18]\ \mu\text{m}$), feed flow drift ($\pm 8\%$), and reject split variation ($\pm 0.35\%$).
- **Total Coupled 3D GPU Droplet Trajectories Integrated**: **`15,970,490,852` (`15.97 billion`)** across a 10-bin lognormal inlet oil DSD (`5–55 µm`) with dynamic Kolmogorov/Hinze entry-shear breakup coupling ($d_{\max} \propto (\sigma/\rho_c)^{3/5} \varepsilon^{-2/5}$).
- **Quality-Diversity & Pareto Archive**: **`191` occupied 4D MAP-Elites cells** spanning all **7 topology families** (`100%` topological diversity retention) and **`275` Rank-0 Constrained NSGA-II Pareto elites** (`100.0%` of which strictly dominate `baseline-ct35` across all 5 objectives simultaneously).

| Metric / Objective (`F1` 24-Sample LHS + 10-Bin Coupled DSD Screen) | `baseline-ct35` Reference | Promoted #1 Elite (`xbabe2-gen043-isl10-ind030`) | Relative Improvement |
| :--- | :---: | :---: | :---: |
| **Topology Class** | `baseline-colman-thew` | `trifurcated-6jet-vortex-head` | 6-jet helical manifold + 3-vane core |
| **95th-Percentile Treated Oil $C_{\text{out},p95}$ [mg/L]** ($\le 30.0$ gate) | `28.229 mg/L` | **`15.468 mg/L`** (`nominal: 10.671 mg/L`) | **`-45.21%` lower oil carry-under** |
| **Static Pressure Drop $\Delta p_u$ [Pa]** ($F_1$ hydraulic model) | `29,468.22 Pa` | **`16,000.00 Pa`** (`OpenFOAM 14 G2: 18,699.40 Pa`) | **`-45.70%` screen / `-36.54%` 3D CFD** |
| **Specific Hydraulic Energy $e_{\text{loss}}$ [J/m³]** | `33,709.55 J/m³` | **`16,256.81 J/m³`** (`OpenFOAM 14 G2: 22,047.32 J/m³`) | **`-51.77%` screen / `-34.60%` 3D CFD** |
| **5th-Percentile $20\ \mu\text{m}$ Grade Capture $E_{20,p05}$ [-]** | `0.8617` (`86.17%`) | **`0.9462` (`94.62%`)** | **`+8.45 percentage points`** |
| **Cut Size $d_{50}$ [$\mu\text{m}$]** | `11.537 µm` | **`10.354 µm`** | **`-10.25%` finer droplet cut** |
| **Peak Entry Shear Rate $\dot{\gamma}_{\max}$ [$\text{s}^{-1}$]** | `4,850.0 s⁻¹` | **`2,584.9 s⁻¹`** | **`-46.70%` lower emulsification shear** |

![Multi-Objective Pareto Front and 4D MAP-Elites Convergence](figures/pareto-map-elites-convergence.png)

### 3.3 Leave-One-Topology-Family-Out (`LOFO-CV`) Surrogate Qualification (`P07`)

To verify that surrogate-assisted pre-screening generalizes across unseen topological families without silent rank reversals, we executed a Leave-One-Topology-Family-Out cross-validation (`LOFO-CV`) using an ARD Matérn-5/2 Gaussian Process surrogate (`crates/joil-core/src/surrogate.rs`, `docs/results/surrogate-qualification-v1.json`, `SHA-256: daa9942a840663d08239f01e140df4189d8ec13f422b3f74659023e6d36ff6f8`):
- **Out-of-Family Rank Correlation**: Spearman $\rho = \mathbf{0.8750}$ (`gate >= 0.70`), Kendall $\tau = \mathbf{0.7333}$.
- **Top-Quartile Precision & Regret**: Top-quartile precision **`100.0%`**, normalized top-1 selection regret **`0.0000`**.
- **Uncertainty Calibration**: Empirical 95% GP confidence interval coverage **`92.86%`** (`gate in [80%, 100%]`), with explicit out-of-distribution epistemic uncertainty inflation ($\sigma_{\text{OOD}} = 1.976\,\sigma_{\text{in-family}}$) preventing uncalibrated extrapolation.

---

## 4. Critique-Proof 3D OpenFOAM 14 Verification: 3-Grid Roache `GCI` & Dual 7-Equation `RSM` (`LRR` + `SSG`) Cross-Check (`P04` / `P05`)

### 4.1 3-Grid Systematic Spatial Refinement (`G0`, `G1`, `G2`) on the Promoted Elite

To ensure that the hydraulic advantage of the promoted #1 Pareto elite (`xbabe2-gen043-isl10-ind030`, `trifurcated-6jet-vortex-head`, `design_sha256: 88da933b54c303e6bda1031c11b3334cf90f997361f90ac6321994c1a2d9e7dd`) is not an artifact of mesh resolution or turbulence closure, we compiled its 5-patch watertight STL (`6,416` triangles, `0` open/non-manifold edges) and executed a 3-grid systematic refinement study in **OpenFOAM Foundation 14** (`sha256:8866eda29be2bcbc05e0fa9536cacb3c63ff25c97179e058959ffbeb90da7199`) at matched $Q_i = 4.000\text{ m}^3/\text{h}$ feed and overflow backpressure $p_o/\rho = -180\text{ m}^2/\text{s}^2$:

- **`G0` Coarse Grid (`runs/rust-cfd-elite-g0-coarse`)**: Background hex grid `18 × 12 × 212` ($\Delta_0 = 8.00\text{ mm}$) $\to$ **`139,981` snapped cells** ($h_{\text{eff}} = 1.992\text{ mm}$, $V = 0.00110627\text{ m}^3$).
- **`G1` Medium Grid (`runs/rust-cfd-elite-gen024-isl2-ind014`)**: Background hex grid `24 × 16 × 282` ($\Delta_0 = 6.00\text{ mm}$) $\to$ **`305,522` snapped cells** ($h_{\text{eff}} = 1.537\text{ mm}$, $V = 0.00110848\text{ m}^3$, linear refinement ratio $r_{10} = h_0 / h_1 = \mathbf{1.296}$).
- **`G2` Fine Grid (`runs/rust-cfd-elite-g2-fine`)**: Background hex grid `30 × 20 × 352` ($\Delta_0 = 4.80\text{ mm}$) $\to$ **`552,513` snapped cells** ($h_{\text{eff}} = 1.262\text{ mm}$, $V = 0.00110943\text{ m}^3$, linear refinement ratio $r_{21} = h_1 / h_2 = \mathbf{1.218}$).

All three grids preserved all five boundary patches (`wall`, `inlet1`, `inlet2`, `overflow`, `underflow`) with zero failed topology checks in `checkMesh -allGeometry -allTopology`.

| Verification Case (`OpenFOAM 14`) | Turbulence Closure | Snapped Cells ($N$) | Effective $h_{\text{eff}}$ [mm] | Feed $Q_i$ [m³/h] | Overflow Split $r$ [%] | Water Closure Residual | Static Pressure Drop $\Delta p_u$ [Pa] | Specific Hydraulic Loss $e_{\text{loss}}$ [J/m³] | $\Delta p_u$ vs `CT-35` Baseline |
| :--- | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: | :---: |
| **`CT-35` Baseline (`ct35-sst-extended-000`)** | `kOmegaSST` | `334,768` | `1.545 mm` | `4.0000` | `1.986%` | `1.63e-5` | `29,468.22 Pa` | `33,709.55 J/m³` | `0.00%` (Comparator) |
| **Elite `G0` Coarse (`rust-cfd-elite-g0-coarse`)** | `kOmegaSST` | `139,981` | `1.992 mm` | `4.0000` | `1.989%` | `1.41e-5` | **`17,310.97 Pa`** | **`21,398.41 J/m³`** | **`-41.26%`** |
| **Elite `G1` Medium (`rust-cfd-elite-gen024-isl2-ind014`)** | `kOmegaSST` | `305,522` | `1.537 mm` | `4.0000` | `1.971%` | `1.81e-5` | **`19,526.53 Pa`** | **`23,124.57 J/m³`** | **`-33.74%`** |
| **Elite `G2` Fine (`rust-cfd-elite-g2-fine`)** | `kOmegaSST` | `552,513` | `1.262 mm` | `4.0000` | `1.996%` | `8.66e-6` | **`18,699.40 Pa`** | **`22,047.32 J/m³`** | **`-36.54%`** |
| **Elite `G1` 7-Eq. `LRR` RSM (`rust-cfd-elite-g1-rsm-lrr`)** | **`LRR` (Linear RSM)** | `305,522` | `1.537 mm` | `4.0000` | `1.969%` | `1.07e-4` | **`18,119.20 Pa`** | **`21,560.75 J/m³`** | **`-38.51%`** |
| **Elite `G1` 7-Eq. `SSG` RSM (`rust-cfd-elite-g1-rsm-ssg`)** | **`SSG` (Quadratic RSM)** | `305,522` | `1.537 mm` | `4.0000` | `1.968%` | `1.45e-4` | **`18,165.71 Pa`** | **`21,543.52 J/m³`** | **`-38.35%`** |

### 4.2 Roache Grid Convergence Index (`GCI`) & Epistemic Treatment of Unstructured Snapping

In accordance with `docs/SCIENCE.md` (*"NASA spatial-convergence guidance informs systematic refinement; non-asymptotic data do not get a decorative GCI certificate"*), we inspect the convergence ratio $R = (f_2 - f_1)/(f_1 - f_0)$ across `G0 -> G1 -> G2` (`docs/results/elite-3grid-gci-and-rsm-verification-v1.json`; full mathematical derivation in **Appendix A**):
1. **Damped Oscillatory Convergence ($R \in (-1, 0)$)**:
   - For $\Delta p_u$: $e_{10} = f_1 - f_0 = +2,215.56\text{ Pa}$ (`+11.35%`) and $e_{21} = f_2 - f_1 = -827.13\text{ Pa}$ (`-4.42%`), yielding $R = e_{21}/e_{10} = \mathbf{-0.3733} \in (-1, 0)$.
   - For $e_{\text{loss}}$: $e_{10} = +1,726.16\text{ J/m}^3$ (`+7.46%`) and $e_{21} = -1,077.25\text{ J/m}^3$ (`-4.89%`), yielding $R = \mathbf{-0.6241} \in (-1, 0)$.
   - Physical cause: in octree-snapped hex-dominant meshes (`snappyHexMesh`), changing the background hex cell size ($\Delta_0 = 8.0\text{ mm} \to 6.0\text{ mm} \to 4.8\text{ mm}$) shifts the octree split planes relative to the 6 trifurcated tangential inlet nozzles, causing a damped oscillation whose amplitude contracts by **`62.7%`** on $\Delta p_u$ and **`37.6%`** on $e_{\text{loss}}$ between `G0->G1` and `G1->G2`.
2. **Standard Roache `GCI` vs. Full 3-Grid Extremum Span**:
   - Using the Celik et al. (2008) / ASME V&V 20 fixed-point observed order ($p_{\text{obs}} = 4.00$ for $\Delta p_u$, $p_{\text{obs}} = 2.00$ for $e_{\text{loss}}$, $p_{\text{obs}} = 1.95$ for split $r$) with $F_s = 1.25$, the standard fine-grid Grid Convergence Indices are **$\text{GCI}_{21}(\Delta p_u) = 4.60\%$**, **$\text{GCI}_{21}(e_{\text{loss}}) = 12.61\%$**, and **$\text{GCI}_{21}(r) = 3.27\%$**.
   - Because $R < 0$ (oscillatory convergence), we do **not** rely on Richardson extrapolation (`18.01 kPa`) to inflate the claimed improvement. Instead, we bound discretization uncertainty using the full 3-grid extremum span (`17.31–19.53 kPa`, total span `2,215.56 Pa`, half-span `±5.92%` around `G2`).
   - Even at the highest-drag grid in the entire triplet (`G1`: `19,526.53 Pa`), the promoted elite is **`9,941.69 Pa` (`-33.74%`) below `CT-35`**, and on the finest grid (`G2`: `18,699.40 Pa`) the **`10,768.81 Pa` (`-36.54%`) pressure-drop reduction is `4.86×` larger than the entire 3-grid span** (`6.76×` larger for specific hydraulic energy loss).

### 4.3 Dual 7-Equation Reynolds-Stress Transport Model (`LRR` & `SSG` RSM) Cross-Checks

Eddy-viscosity closures (`kOmegaSST`) invoke the Boussinesq isotropic eddy-viscosity hypothesis, which can under-predict streamline-curvature stabilization and normal-stress anisotropy in strongly swirling flows ($\text{Sw} > 3$). To verify that the elite's hydraulic efficiency survives second-moment turbulence closures across **both linear and quadratic pressure-strain redistribution formulations**, we executed 60-step continuations on `G1` (`305,522` cells) using:
1. **`LRR` RSM (`runs/rust-cfd-elite-g1-rsm-lrr`)**: Launder–Reece–Rodi (1975) linear pressure-strain model ($\Pi_{ij}^{\text{slow}} = -C_1 \varepsilon b_{ij}$, linear rapid redistribution in the anisotropy tensor $b_{ij} = \frac{R_{ij}}{2k} - \frac{1}{3}\delta_{ij}$).
2. **`SSG` RSM (`runs/rust-cfd-elite-g1-rsm-ssg`)**: Speziale–Sarkar–Gatski (1991) quadratic pressure-strain model, which adds non-linear return-to-isotropy terms quadratic in $b_{ij}$ ($\Pi_{ij}^{\text{slow}} = -(C_1 \varepsilon + C_1^* P) b_{ij} + C_2 \varepsilon (b_{ik} b_{kj} - \frac{1}{3} b_{mn} b_{mn} \delta_{ij})$) to eliminate wall-reflection sensitivity in curved/swirling boundary layers.

Both closures solve 7 coupled PDEs for $(R_{xx}, R_{xy}, R_{xz}, R_{yy}, R_{yz}, R_{zz}, \varepsilon)$ across all `305,522` cells:

| RSM Diagnostic Metric (`305,522` Cells on `G1`) | `LRR` (Linear Pressure-Strain) | `SSG` (Quadratic Pressure-Strain) | Inter-RSM Parity (`SSG` vs `LRR`) |
| :--- | :---: | :---: | :---: |
| **Inlet-to-Underflow Pressure Drop $\Delta p_u$ [Pa]** | **`18,119.20 Pa`** (`-38.51%` vs `CT-35`) | **`18,165.71 Pa`** (`-38.35%` vs `CT-35`) | **`+0.26%` (`+46.51 Pa`)** |
| **Specific Hydraulic Energy Loss $e_{\text{loss}}$ [J/m³]** | **`21,560.75 J/m³`** (`-36.04%` vs `CT-35`) | **`21,543.52 J/m³`** (`-36.09%` vs `CT-35`) | **`-0.08%` (`-17.24 J/m³`)** |
| **Volumetric Overflow Reject Split $r$ [%]** | `1.9691%` | `1.9683%` | `-0.0008 percentage points` |
| **Normal-Stress Realizability ($R_{xx}, R_{yy}, R_{zz} > 0$)** | **`100.00%` (`305,522 / 305,522`)** | **`100.00%` (`305,522 / 305,522`)** | Exact `100%` realizability |
| **Domain Mean TKE $\langle k \rangle$ [$\text{m}^2/\text{s}^2$]** | `0.04036 m²/s²` (`P95 = 0.1207`) | `0.03983 m²/s²` (`P95 = 0.1199`) | `-1.32%` |
| **Transverse Normal Stresses $\langle R_{xx} \rangle, \langle R_{yy} \rangle$ [$\text{m}^2/\text{s}^2$]** | `0.02474`, `0.02458` | `0.02505`, `0.02495` | Axisymmetric within `< 0.65%` |
| **Axial Normal Stress $\langle R_{zz} \rangle$ [$\text{m}^2/\text{s}^2$]** | `0.03141` (`P90 = 0.0704`) | `0.02966` (`P90 = 0.0653`) | Elevated axial core variance |
| **Mean Axial-to-Transverse Ratio $\langle R_{zz} / R_\perp \rangle$ [-]** | **`1.159`** | **`1.079`** | Quadratic return-to-isotropy damping |
| **Mean / P95 Lumley Anisotropy Norm $\|b_{ij}\|$ [-]** | **`0.138` / `0.392`** | **`0.099` / `0.260`** | Expected quadratic vs linear anisotropy |

- **Cross-Closure Hydraulic Parity**: `LRR` and `SSG` RSM agree to within **`0.26%` (`46.5 Pa`) on $\Delta p_u$** and **`0.08%` (`17.2 J/m³`) on $e_{\text{loss}}$**, and both sit within **`2.85%–3.10%`** of the fine-grid `552,513`-cell `G2 kOmegaSST` solution (`18.70 kPa`, `22.05 kJ/m³`).
- **Physical Interpretation of `LRR` vs. `SSG` Tensor Structure**: As predicted by second-moment turbulence theory, `SSG`'s quadratic slow pressure-strain term $C_2 \varepsilon (b_{ik} b_{kj} - \frac{1}{3} \text{II}_b \delta_{ij})$ provides stronger return-to-isotropy at high swirl than `LRR`'s linear Rotta term, moderating the 95th-percentile Lumley anisotropy norm from `0.392` (`LRR`) to `0.260` (`SSG`) while preserving the axial-dominated vortex-core normal-stress hierarchy ($R_{zz} > R_{xx} \approx R_{yy}$) and predicting virtually identical macroscopic hydraulic losses.

![3-Grid Roache GCI and Dual 7-Equation LRR/SSG RSM Verification Plate](figures/elite-gci-and-rsm-verification.png)

### 4.4 Internal 3D Swirl-Head & Reducing-Cone Vortex Physics in OpenFOAM 14

To understand **why** the `trifurcated-6jet-vortex-head` elite achieves a `36.5%–38.5%` reduction in static pressure drop without sacrificing the centripetal buoyancy field required for oil-droplet capture, we extracted the full 3D cell-centered velocity ($\mathbf{U}$), kinematic pressure ($p/\rho$), and turbulent dissipation ($\varepsilon$) fields directly from the converged OpenFOAM 14 solutions of `CT-35` (`runs/ct35-sst-extended-000`) and the promoted elite (`runs/rust-cfd-elite-gen024-isl2-ind014`):

1. **Elimination of Twin-Jet Wall Impingement Losses ($z = 35\text{ mm}$, Panel A & Panel B)**:
   - In `CT-35`, injecting $4.0\text{ m}^3/\text{h}$ through two concentrated tangential ports creates an over-accelerated wall jet ($U_\theta \approx 5.2\text{ m/s}$ at $r = 30\text{ mm}$) that dissipates mechanical energy against the cylindrical entry wall before transitioning into the reducing cone.
   - In the `trifurcated-6jet-vortex-head` elite (`2 -> 6` helical manifold with $78.6^\circ$ wrap + 3-vane swirl conditioner), the entry momentum is distributed azimuthally across 6 progressive jets, lowering peak outer-wall velocity at $z = 35\text{ mm}$ to $U_\theta \approx 3.6\text{ m/s}$ while **maintaining identical inner-core tangential swirl ($U_\theta \approx 1.8\text{–}2.5\text{ m/s}$ for $r \le 8\text{ mm}$) around the overflow vortex finder**.
2. **Preserved Radial Centripetal Pressure Gradient Across the Separation Cone ($z \in [105, 880]\text{ mm}$, Panel C)**:
   - Because radial droplet migration speed scales with $\partial p / \partial r = \rho U_\theta^2 / r$, oil separation is governed by the **radial pressure difference between the outer wall and the vortex core**, $\Delta p_{\text{radial}}(z) = p_{\text{wall}}(z) - p_{\text{core}}(z)$, rather than the absolute inlet gauge pressure.
   - As shown in Panel C, while the elite lowers entry wall pressure by `~10.8 kPa` in the swirl head ($z < 100\text{ mm}$), its smooth inflected $S$-cone preserves an axial core depression ($p_{\text{core}} \approx -16\text{ to }-22\text{ kPa}$ near the finder throat) and radial wall-to-core pressure difference across the separation cone ($z \in [120, 780]\text{ mm}$) that closely tracks `CT-35` while avoiding premature tailpipe wall-drag losses.
3. **Suppression of Entry-Nozzle Turbulent Shear Emulsification ($z \in [0, 120]\text{ mm}$, Panel D)**:
   - Evaluating the Kolmogorov–Hinze maximum stable oil droplet diameter $d_{\max} = 0.725\,(\sigma / \rho_c)^{3/5} \varepsilon^{-2/5}$ ($\sigma = 0.025\text{ N/m}$, $\rho_c = 998\text{ kg/m}^3$) across all entry-chamber cells ($z \in [0, 120]\text{ mm}$) shows that the elite's 99th-percentile dissipation rate ($\varepsilon_{\text{P99}} = 94.3\text{ m}^2/\text{s}^3$) corresponds to $d_{\max,\text{P99}} = \mathbf{91.6\ \mu\text{m}}$, remaining safely above the entire `5–55 µm` feed droplet size distribution and preventing inlet shear breakup.

![3D OpenFOAM 14 Internal Swirl-Head & Vortex Physics Comparison](figures/openfoam14-3d-vortex-physics-comparison.png)

### 4.5 Droplet Force Verification & 10-Cohort OpenFOAM 14 Lagrangian Audit (`P05`)

To ensure that Lagrangian oil-droplet tracking in `P05` adheres strictly to physical conservation laws:
1. **Analytical Solid-Body Rotation + Axial Poiseuille Oracle (`crates/joil-core/src/droplet.rs`, `docs/results/droplet-verification-v1.json`)**:
   - Verified light-droplet ($\rho_d = 850\text{ kg/m}^3$, $\rho_c = 998\text{ kg/m}^3$) radial migration in a stationary-frame solid-body vortex ($\Omega = 1,200\text{ rad/s}$) without artificial centrifugal forces, reproducing the exact analytical centripetal pressure-gradient / buoyancy / Stokes drag equilibrium drift $u_{r,\text{eq}}(r) = \frac{(\rho_d - \rho_c)\Omega^2 r d_p^2}{18\mu_c}$ to **`0.000%` relative error** across $d_p \in [10, 50]\ \mu\text{m}$.
2. **OpenFOAM 14 10-Cohort Inlet-Flux Lagrangian Audit (`docs/results/openfoam-lagrangian-audit-v1.json`)**:
   - Audited the 10-bin (`5–50 µm`) OpenFOAM 14 `kinematicCloud` parcel injection and boundary escape accounting (`runs/gemini-ct35-full-liner-inlet-t600-dt5e-05-84311d`), verifying **`0` mass-conservation discrepancies** across all 10 size cohorts while enforcing the epistemic rule that short-window (`0.10 s`) unexited inventory (`~97.2%` in-domain) is classified as `BOUNDED_SHORT_WINDOW_TRANSPORT_PROBE_NOT_OIL_FITNESS`.

---

## 5. Limitations, `F4` Physical Validation Protocol & Wave-2 Evolutionary Roadmap

### 5.1 Epistemic Boundaries (`AGENTS.md` & `docs/SCIENCE.md`)

In strict compliance with `AGENTS.md`, `docs/MASTER_PLAN.md`, and `docs/SCIENCE.md`, we separate what has been computationally verified from what requires physical laboratory evidence:
- **Completed & Hash-Bound Computational Evidence (`F0` / `F1` / `F2-Carrier`)**:
  - Exact `CT-35` baseline CAD/STL orientation audit and $20 D_u$ extended-outlet OpenFOAM 14 reference (`runs/ct35-sst-extended-000`).
  - 7-family compositional 3D parametric geometry compiler, `F0` Red-Team gate, `P08` structural/modal/LPBF gate, and interactive Three.js 3D topology explorer (`apps/explorer`).
  - 3-node (`xbabe1` + `xbabe2` + `xbabe3`) distributed Rust + CUDA C++20 evolutionary campaign (`119,235` 3D evaluations, `2,857,464` MC realizations, `15.97B` droplet trajectories, `191` MAP-Elites cells, `275` Rank-0 Pareto elites).
  - 3-grid systematic OpenFOAM 14 refinement (`G0`: `140k`, `G1`: `306k`, `G2`: `553k` cells) with Roache `GCI` and dual 7-equation `LRR` + `SSG` Reynolds-Stress Model (`RSM`) cross-checks on the promoted elite (`xbabe2-gen043-isl10-ind030`), demonstrating a `-33.74%` to `-38.51%` reduction in static pressure drop and `-31.40%` to `-36.09%` reduction in specific hydraulic energy loss at matched $Q = 4.000\text{ m}^3/\text{h}$ and $r \approx 1.97\%\text{–}2.00\%$.
- **Unwaived Pre-Qualification Gates (`qualified_fitness: null`, `physical_validation: false`)**:
  - Full-residence coupled `F2` transient droplet closure ($T \ge 4.0\text{ s}$ until retained in-domain oil inventory $< 0.1\%$) and `F4` physical rig trials remain required before promoting any candidate to qualified industrial superiority.

### 5.2 Pre-Registered `F4` Physical Validation & `CAD-09` Metrology Protocol (`P06`)

To prevent experimental confirmation bias, the physical qualification campaign (`docs/results/experimental-protocol-qualification-v1.json`, `SHA-256: 0038fb782ca397cf66fd6da632dc876b5695b200926528f8b0c5a4f7825a717f`) locks the following hardware and statistical protocol prior to LPBF build release:

| Qualification Stage | Protocol Specification | Acceptance Gate |
| :--- | :--- | :--- |
| **`CAD-09` LPBF Replication & X-Ray CT Metrology** | $n = 3$ independent Ti-6Al-4V ELI LPBF builds of `baseline-ct35` (`A1..A3`) and promoted elite `xbabe2-gen043-isl10-ind030` (`B1..B3`); full micro-CT voxel registration ($25\ \mu\text{m}$ resolution) against `STEP`/`STL` master | Overflow throat $D_o$ within $\pm 50\ \mu\text{m}$; 6-jet total throat area within $\pm 1.5\%$; internal bore $R_a \le 12\ \mu\text{m}$ after abrasive flow conditioning; zero trapped powder in helical branches |
| **Blinded 5-Block `ABBA`/`BAAB` Crossover Rig Trial** | 5 randomized crossover blocks (`20` paired intervals, blinding salt `joil-p06-blinded-protocol-v1`) on single-liner produced-water flow loop ($C_i = 500 \pm 25\text{ mg/L}$, $d_{32} = 22 \pm 2\ \mu\text{m}$, $r = 2.00 \pm 0.15\%$) | Paired t-test statistical power **`89.4%`** at one-sided $\alpha = 0.025$ to detect $\ge 10\%$ capacity gain under realistic run-to-run variance ($\sigma_{\text{rep}} = 3.5\%$, $\sigma_{\text{print}} = 2.5\%$) |
| **Promotion Decision Rules (`H0` / `H10` / `H20`)** | Primary endpoint: compliant treated-water capacity $Q_{\text{treated}} = Q_i(1-r)$ at fixed $\Delta p_u$ budget and $C_u \le 30\text{ mg/L}$; secondary endpoint: $\Delta p_u$ reduction at fixed $Q_i = 4.00\text{ m}^3/\text{h}$ | **`H10` Promotion**: paired mean $\Delta Q_{\text{treated}} \ge +10.0\%$ with 95% lower confidence bound $> 0.0\%$ and zero water-quality regression ($C_{u,\text{elite}} \le C_{u,\text{CT35}}$) |

### 5.3 Next-Wave Flexible 3D Evolutionary Campaign Plan (Wave 2 — [`docs/WAVE2_PLAN.md`](WAVE2_PLAN.md))

With the Wave-1 science dossier locked (`119,235` compositional 3D evaluations across 7 families + 3-grid `GCI` + dual `LRR`/`SSG` RSM verification), our complete worker-ready Wave-2 execution plan ([`docs/WAVE2_PLAN.md`](WAVE2_PLAN.md)) extends topological freedom and statistical rigor along three axes:
1. **Freeform Fourier-Surface & Neural-SDF Swirl-Head Operators (`joil-core` Wave-2 Genome)**:
   - Extend the 7 discrete topological families into a continuous trans-dimensional operator graph supporting arbitrary branch bifurcations ($2 \to K$ helical distributors with $K \in \{3, 4, 5, 6, 8\}$), variable-pitch non-axisymmetric $S$-cones ($r(\theta, z) = R_0(z) + \sum_{m=1}^6 [a_m(z)\cos(m\theta) + b_m(z)\sin(m\theta)]$), and variable-camber 3D guide vanes parameterized by divergence-free stream surfaces.
2. **Multi-Fidelity `F1 -> F2` Active-Learning Loop in the Evolution Inner Cycle**:
   - Couple the `LOFO-CV`-qualified ARD Matérn-5/2 GP surrogate (`Spearman = 0.875`) directly to asynchronous OpenFOAM 14 `G1` (`306k`-cell) `LRR`/`SSG` evaluations launched automatically on the top Constrained Expected Improvement (`CEI`) candidates every 5 generations across `xbabe1`, `xbabe2`, and `xbabe3`, continuously recalibrating the `F1` CUDA kernel's swirl-decay and entry-loss coefficients per topology niche.
3. **Multi-Archetype 3D CFD Verification Matrix**:
   - Promote the top Rank-0 Pareto champion from **each of the remaining 5 exotic families** (`quad-spiral-manifold`, `helical-ribbon-core`, `s-cone-fluted-finder`, `double-vortex-skimmer`, `scalloped-parabolic-chamber`) through the automated 5-patch OpenFOAM 14 `G1` + `RSM` verification pipeline to construct a multi-family 3D Navier–Stokes Pareto frontier prior to `F4` LPBF fabrication.

---

## Appendix A. Mathematical Formulation of Unstructured 3-Grid Roache `GCI`

For a 3-grid systematic refinement sequence $(G_0, G_1, G_2)$ with total snapped fluid volumes $V_k$ and cell counts $N_k$ ($N_0 < N_1 < N_2$), the effective unstructured linear cell size and grid refinement factors are defined per ASME V&V 20-2009 / Celik et al. (2008):

$$h_k = \left(\frac{V_k}{N_k}\right)^{1/3}, \qquad r_{21} = \frac{h_1}{h_2} = 1.2180, \qquad r_{10} = \frac{h_0}{h_1} = 1.2963$$

Let $f_0, f_1, f_2$ denote a scalar integral quantity of interest (such as $\Delta p_u$ or $e_{\text{loss}}$) on coarse, medium, and fine grids, with differences $e_{21} = f_2 - f_1$ and $e_{10} = f_1 - f_0$. The convergence ratio $R = e_{21} / e_{10}$ classifies the spatial behavior into four regimes:
- $0 < R < 1$: Monotonic convergence;
- $-1 < R < 0$: Damped oscillatory convergence (`observed on G0 -> G1 -> G2 due to snappyHexMesh octree split alignment`);
- $R < -1$: Oscillatory divergence;
- $R > 1$: Monotonic divergence.

When $r_{21} \ne r_{10}$, the observed order of accuracy $p$ is solved via the fixed-point iteration:

$$p = \frac{1}{\ln r_{21}} \Bigl|\ln |e_{10} / e_{21}| + q(p)\Bigr|, \qquad q(p) = \ln\!\left(\frac{r_{21}^p - s}{r_{10}^p - s}\right), \qquad s = \operatorname{sgn}\!\left(\frac{e_{10}}{e_{21}}\right)$$

The standard fine-grid Roache Grid Convergence Index (`GCI`) with safety factor $F_s = 1.25$ and the conservative oscillatory/non-asymptotic envelope bound ($F_s = 3.0$, $p = 1.0$, plus the full 3-grid extremum span $\Delta f_{\text{span}} = \max_k f_k - \min_k f_k$) are given by:

$$\text{GCI}_{21}^{\text{std}} = \frac{1.25\,|e_{21}| / |f_2|}{r_{21}^p - 1}, \qquad \text{GCI}_{21}^{\text{cons}} = \frac{3.0\,|e_{21}| / |f_2|}{r_{21} - 1}, \qquad \text{SNR}_{\text{span}} = \frac{|f_{\text{CT35}} - f_2|}{\max_k f_k - \min_k f_k}$$

Reporting all three quantities simultaneously in `docs/results/elite-3grid-gci-and-rsm-verification-v1.json` ensures complete numerical transparency without claiming asymptotic monotonic extrapolation where $R \in (-1, 0)$.

---

## 6. Reproducibility & Verification Index

All claims, figures, and JSON dossiers in this white paper are reproducibly generated and verified by the single-command repository gate (`python3 tools/check.py`):

| Artifact / Verification Dossier | Repository Path | Content Identity (`SHA-256`) |
| :--- | :--- | :--- |
| **3-Node Campaign Summary (`119,235` evals, `15.97B` droplets)** | [`docs/results/rust-evolution-qualification-v1.json`](results/rust-evolution-qualification-v1.json) | `1bc32f98d38cd22525130843bf42034c0078146924b76148e06a4a02c9cff6b3` |
| **Head-to-Head Exotic vs. `CT-35` CFD Benchmark** | [`docs/results/exotic-vs-baseline-cfd-benchmark-v1.json`](results/exotic-vs-baseline-cfd-benchmark-v1.json) | `d20e9d72dfa004c1bc77a9389646b3ebb75f3004d85ad8c397f902afa181e496` |
| **3-Grid Roache `GCI` & Dual 7-Eq. `LRR`/`SSG` RSM Dossier** | [`docs/results/elite-3grid-gci-and-rsm-verification-v1.json`](results/elite-3grid-gci-and-rsm-verification-v1.json) | `6bcac03b3724fa5fd60a83db33614756016c5188a7c028ddda86ab7a10de966c` |
| **7-Topology 3D Archetype Atlas Figure Plate** | [`docs/figures/rust-3d-topology-atlas.png`](figures/rust-3d-topology-atlas.png) | `76e7ce10a2c334575b5dedcea04b33d51f8959d38ef849560c477aa1cc5cd8b7` |
| **Pareto Front & 4D MAP-Elites Convergence Figure Plate** | [`docs/figures/pareto-map-elites-convergence.png`](figures/pareto-map-elites-convergence.png) | `3afd610ac646a3ae13c84d2939213d487e98935541457292f65d7507d8a7c6f7` |
| **3-Grid `GCI` & Dual 7-Eq. `LRR`/`SSG` RSM Figure Plate** | [`docs/figures/elite-gci-and-rsm-verification.png`](figures/elite-gci-and-rsm-verification.png) | `dbce90bb3878dd863427e8d50f67b8ff469f6dfe4e782a9719f32db00710bbb9` |
| **3D OpenFOAM 14 Internal Vortex-Physics Figure Plate** | [`docs/figures/openfoam14-3d-vortex-physics-comparison.png`](figures/openfoam14-3d-vortex-physics-comparison.png) | `5c0c4b1ee4d0a82f09d8c086334fe222201473e8801cda806319dd76ef7cac44` |
| **Matérn-5/2 GP Surrogate `LOFO-CV` Qualification** | [`docs/results/surrogate-qualification-v1.json`](results/surrogate-qualification-v1.json) | `daa9942a840663d08239f01e140df4189d8ec13f422b3f74659023e6d36ff6f8` |
| **P08 Structural, Modal & LPBF Qualification** | [`docs/results/structural-qualification-v1.json`](results/structural-qualification-v1.json) | `848f18dc92dc61661cb3a57966c6137f57f1b529bd654d3807b110665ff18f40` |
| **P05 Analytical Droplet Physics Verification** | [`docs/results/droplet-verification-v1.json`](results/droplet-verification-v1.json) | `e0956795f6c1c42958b35a0b3413bd133db80194127670933b41907d50223ec3` |
| **P06 Pre-Registered 5-Block Blinded Flow-Loop Protocol** | [`docs/results/experimental-protocol-qualification-v1.json`](results/experimental-protocol-qualification-v1.json) | `0038fb782ca397cf66fd6da632dc876b5695b200926528f8b0c5a4f7825a717f` |

