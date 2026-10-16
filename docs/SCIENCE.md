# Physics, metrics and qualification contract

Version 1. These rules govern implementation; the initial arithmetic library implements only a subset. No validated CFD or experimental dataset currently exists in this repository.

## Concentration and capture

Let `r=Qo/Qi` be overflow/feed volume split; `E(d)` be injected oil mass in inlet diameter class d that reaches the oil reject; and `w(d)` be inlet **oil mass** weights, equivalent to oil-volume weights only for equal oil density. A complete dilute cohort with all oil exiting has:

```
Eoil = integral E(d) w(d) dd
Cu_mg_L = Ci_mg_L * (1 - Eoil) / (1 - r)
Ereduced(d) = (E(d) - r) / (1 - r)
Cu_mg_L = Ci_mg_L * integral (1 - Ereduced(d)) w(d) dd
```

The last two forms are equivalent. Omitting `(1-r)` while using raw oil-mass capture underestimates outlet concentration. A passive tracer has E=r and Cu=Ci, not apparent oil treatment. At Ci=500 mg/L, E=.94 and r=.02, Cu=30.6122449 mg/L: it **fails** a 30 mg/L duty. At E=.95, Cu=25.5102041 mg/L. Required raw capture at 30 mg/L is at least .9412; at 20 mg/L at least .9608. These exact cases are regression oracles.

Do not clip negative reduced efficiencies: E<r means oil is preferentially leaving in the treated stream and is a meaningful poor result. Enforce 0≤E≤1, 0≤r<1, finite nonnegative concentrations. A DSD must declare number, area, volume or mass basis and normalized weights. Use measured quadrature/bin edges and verify tail mass and bin refinement; a lognormal volume median cannot be substituted for a number median. Diameter conversions must conserve oil volume.

For nonzero retained/in-domain oil, report `m_in=m_reject+m_treated+m_remaining+m_wall` plus explicit transfers and numerical discrepancy. Use measured treated oil mass / treated outlet volume for sampled transient concentration. Do not infer all non-reject mass is water exit unless cohorts close. Report lower/upper Cu and capture bounds by assigning unresolved mass to adverse outcomes. A steady inventory assumption needs evidence. Signed outlet recrossings do not create multiple captured masses.

Breakup-enabled transport uses a mass transfer matrix: inlet diameter bins -> reject/treated outlet diameter bins + retained/in-domain states. Each row conserves injected oil mass. It can be reweighted only when independent of feed concentration/distribution and coupling/coalescence, inside the tested linear envelope. Otherwise rerun the coupled population. A daughter is not a new injected mass. Reject boundary-bins that lose mass.

## Pressure, flow and energy

`PDR=(pin-po)/(pin-pu)` with positive inlet-to-water drop. Record tap coordinates, averaging, absolute/gauge/reduced/kinematic pressure and units. Convert incompressible `p/rho` to Pa by reading field dimensions and declared density. Restore hydrostatic/reference offsets before absolute-pressure/gas checks. Vertical elevation in energy is distinct from the downstream z coordinate if gravity is +z.

Set total feed and branch profiles, water outlet pressure, then iterate reject backpressure to measured target r. Freeze convergence policy and pressure bounds. A fixed split cannot be imposed alongside both independent outlet pressures. Pressure-matched comparison uses a bracketed measured/CFD relation; `Qguess=Qref*sqrt(dpbase/dpdesign)` is only a seed. Swirl, Reynolds number and breakup can change Euler number.

Hydraulic energy comes from integrated mechanical flux, including nonuniform velocity/pressure, both outlets and elevation. Report static tap drops separately. Specific energy objective uses compliant treated volume, not just feed volume. A fixed-flow loss proxy cannot claim complete rig energy consumption; pumps/valves/manifold are separate system boundaries.

## Duty and capacity

For every required scenario s, require at **the same Q**: upper-bound Cu≤limit; pressure/energy budget; split and PDR controllability; absolute-pressure margin; all geometry/material and numerical gates. Use lower capacity bounds and upper burden bounds where justified. Missing scenario or UNKNOWN => no qualified common point. Current `common_tested_capacity` returns only a sampled mathematical result, never evidence of continuous band performance or physical validation.

```
feasible(Q) = all_s passes(s,Q)
Qfeed* = max Q among tested common feasible operating points
Qtreated* = min_s Qfeed* * (1-r_s(Qfeed*))
gain = Qtreated_candidate* / Qtreated_baseline* - 1
```

If no feasible baseline exists, gain is undefined. Do not insert epsilon denominator. Inspect the measured operating map, agree a new duty revision before search if needed, and retain previous negative result. No monotonic interpolation across failed/unsampled gaps; require a continuous duty band and endpoint uncertainty before a turndown claim. Reference scenarios must share feed/temperature/property convention with comparator. A 30 mg/L research limit and 20 mg/L stretch are two named policies, not selectively switched thresholds.

Optimization minimizes `[-conservative common treated capacity, energy per compliant treated m3, sensitivity/regret]` after hard gates. Invalid numerics and unknown physics remain separate states. Novelty/compute acquisition is an exploration mechanism, not a performance objective. Optional screen log ratio needs finite positive resolved Cu for both comparator and candidate; censored values return an explicit unresolved score.

## Error and validation hierarchy

Record separately: temporal sampling, parcel sampling, spatial/time/substep/class errors, instrument uncertainty, uncertain inputs/manufacturing, and model-form discrepancy. Do not root-sum-square correlated or uncalibrated bounds. Statistical intervals require justified independent units or block correlation handling. Common random numbers improve pairing but do not make different meshes' trajectories identical. Raw parcel count is not effective sample size.

For each candidate decision, allocate an error budget in the observable's units before runs. An initial target is total uncertainty <one third of the gain being resolved, tightened when a quality threshold is nearby. Grade error must be propagated to mg/L, not accepted solely as percentage points. Finite solver residuals, time stability and mesh independence are different requirements.

Validation follows variable- and validation-point-specific comparisons with uncertainty, consistent with [ASME V&V 20](https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-fluid-dynamics-and-heat-transfer). The planned programme is not a declaration of compliance with the standard. [NASA spatial-convergence guidance](https://www.grc.nasa.gov/www/wind/valid/tutorial/spatconv.html) informs systematic refinement; non-asymptotic data do not get a decorative GCI certificate.

Evidence progression: proposed -> geometry checked -> numerically verified -> experimentally validated within stated envelope -> independent finalist review. Each new topology/feature scale/regime can exceed the validated envelope. A result needs exact subject, requirement, physics/numerics, extraction and calibration identities; no caller-supplied boolean upgrades it.

## Physical superiority

Use paired randomized baseline/candidate blocks and independent preparations/builds. Fit capacity endpoints with error bounds; do not compare only two noisily thresholded grid points. Predeclare pressure and split control, continuous band, censored oil measurements and exclusions. Hold out geometry families and fluid conditions from calibration. Determine replication through power/variance analysis.

H10 is a hypothesis: point gain≥10% with paired 95% lower bound>0 and no operating-band/quality/durability regression. To claim lower bound≥10%, that bound must itself meet 10%. A literature curve from a different liner cannot satisfy this physical gate. A commercial claim additionally needs an appropriate current comparator and application-specific evidence.

## Declared applicability envelope

Every computational (`F0`–`F3`) and experimental (`F4`) result in this repository is bound to an explicit physical applicability envelope. Extrapolation outside this envelope invalidates surrogate, reduced-order, and CFD predictions and requires new physical calibration:

1. **In-envelope operating regime (covered by `RS-joil-research-v1` and `joil-wave2-preregistration-v1`)**:
   - **Cartridge envelope & frozen interfaces**: Single-liner `DN35` deoiling hydrocyclone geometry ($D_1 = 70.0\text{ mm}$ swirl head, $L_1 = 70.0\text{ mm}$, $D_2 = 35.0\text{ mm}$ nominal reference section after $L_c = 99.25\text{ mm}$ reducing cone, $D_3 = 17.5\text{ mm}$ fine taper end after $L_2 = 770.0\text{ mm}$ and tailpipe $L_3 = 700.0\text{ mm}$, $D_o = 1.40\text{ mm}$ overflow vortex finder, total axial length $1,639.25\text{ mm}$ plus $120\text{ mm}$ ($0.12\text{ m}$) numerical underflow extension).
   - **Carrier & dispersed phase properties**: Incompressible Newtonian produced-water carrier ($\rho_c \approx 998\text{ kg/m}^3$, kinematic viscosity factor $\mu/\mu_0 \in [0.85, 1.35]$) and dilute dispersed light crude/mineral oil ($\rho_d \in [820, 890]\text{ kg/m}^3$, interfacial tension $\sigma \in [15, 35]\text{ mN/m}$, inlet volume fraction $\alpha_{\text{oil}} \le 0.10\%$ corresponding to $C_i \in [100, 1000]\text{ mg/L}$).
   - **Hydraulic & droplet size regime**: Volumetric feed flow $Q_i \in [2.60, 5.40]\text{ m}^3/\text{h}$ (`4.00 m³/h` nominal design point; `D1_turndown` at `2.60 m³/h`, `D2_turnup` at `5.40 m³/h`), volumetric reject split $r = Q_o / Q_i \in [1.50\%, 3.00\%]$ (`2.00% ± 0.05%` nominal comparison point), and inlet droplet size distribution $d_p \in [5, 100]\ \mu\text{m}$ with volume-weighted median $d_{50,\text{feed}} \in [25, 50]\ \mu\text{m}$ (`D3_sheared_feed` down to $29.5\ \mu\text{m}$).
   - **Additive manufacturing & surface finish**: Clean metallic LPBF Ti-6Al-4V ELI or Inconel 625 internal walls with conditioned sand-grain roughness $k_s \le 35\ \mu\text{m}$ ($R_a \le 12\ \mu\text{m}$), minimum wall thickness $\ge 1.0\text{ mm}$, and minimum internal passage hydraulic diameter $\ge 1.4\text{ mm}$ (with peripheral `TPMS` inserts restricted strictly to $r \ge 0.62\,R_{\text{wall}}(z)$ so the inner reverse-flowing vortex core remains unobstructed).
2. **Out-of-envelope regimes (explicit exclusions; no performance claim permitted)**:
   - **Three-phase gas-liquid-liquid flow ($\text{GVF} > 1.0\%$)**: Free gas cores, gas slugging, and dissolved-gas cavitation alter the radial pressure gradient and destabilize the overflow oil core; current `F1`/`F2` single-phase carrier models assume $\text{GVF} = 0\%$.
   - **Chemically stabilized micro-emulsions ($d_{50,\text{feed}} < 8\ \mu\text{m}$ or $\sigma < 5\text{ mN/m}$)**: Sub-`8 µm` surfactant-stabilized dispersions approach the turbulent dispersion diffusion limit in a `35 mm` liner and require chemical demulsification or coalescer pre-treatment.
   - **Concentrated oil slugs ($\alpha_{\text{oil}} > 1.0\%$)**: High dispersed-phase holdup violates one-way/dilute momentum coupling and requires fully coupled Eulerian–Eulerian multiphase + Population Balance (`PBM`) coalescence closure.
   - **Particulate sand erosion, scaling, and wax/asphaltene fouling**: Unfiltered solids ($d_{\text{solid}} > 150\ \mu\text{m}$) or paraffin/asphaltene deposition can bridge peripheral `TPMS` pores (`1.4–3.2 mm` cell pitch) or multi-branch manifolds and require dedicated `F4` endurance/fouling loop qualification.

## Multi-fidelity qualification ladder (`F0` -> `F4`) and unwaived physical gates

No computational milestone may be self-upgraded to a higher fidelity level or used to waive a downstream physical gate (`qualified_fitness`, `qualified_capacity_gain`, and `physical_validation` in [`docs/STATUS.json`](STATUS.json) remain `null`/`false` until `F4` closes):

| Fidelity Level | Governing Scope & Verification Method | Current Repository Status | Mandatory Pre-Registered Exit Gate to Next Level |
| :--- | :--- | :--- | :--- |
| **`F0`: Geometry, Manifold & `P08` Structural/LPBF** | Watertight 3D B-rep/mesh synthesis (`Phenotype3D`), `0` open/non-manifold edges, frozen `DN35` collar/tail invariance (`0.0 mm³` diff), Lamé burst/fatigue stress ($\text{SF}_y \ge 2.50$), Strouhal modal lock-in ($f_{n,1}/f_{\text{PVC}} \ge 2.20$), LPBF overhang $\le 45^\circ$ | **EXECUTED & VERIFIED** (`11` canonical families; `220/220 = 100.0%` stratified random genomes passing `canonicalize_and_preflight` vs `0/30` negative controls) | Pass `F0` Red-Team (`redteam.rs`) and `P08` structural/modal (`structure.rs`) checks with zero violations |
| **`F1`: 3D Reduced-Order Swirl + RK45 Droplet Screen & 10-Seed Ablation** | Coupled 3D Burger–Rott/harmonic swirl + 10-bin Lagrangian droplet trajectories + Hinze shear breakup across 12–24 LHS operating points; pre-registered 10-seed `B0`–`B3` ablation and `D1`–`D4` held-out duties | **EXECUTED & VERIFIED** (`119,235` 3-node GPU evals + `5,400`-eval 10-seed Wave-2 ablation; Mann–Whitney $U = 100$, $p_{\text{Holm}} = 0.000314$, `0%` rejected-cohort false omissions) | Strictly limited to search screening and `4D MAP-Elites` archive illumination; top elites must enter `F2` 3D Navier–Stokes verification |
| **`F2-Carrier`: 3D OpenFOAM 14 3-Grid `GCI` & Dual `LRR`/`SSG` RSM** | Conformal 5-patch `snappyHexMesh` 3-grid spatial refinement (`G0` `140k`, `G1` `306k`, `G2` `553k` cells) + `kOmegaSST` and dual 7-equation `LRR` (linear) & `SSG` (quadratic) Reynolds-Stress closures at matched $Q_i = 4.000\text{ m}^3/\text{h}$, $r = 2.00 \pm 0.10\%$ | **EXECUTED & VERIFIED** on `CT-35` baseline and promoted 6-jet elite (`-36.54%` `G2` $\Delta p_u$, `-38.51%`/`-38.35%` `LRR`/`SSG` RSM, `0.26%` inter-RSM parity, $\text{SNR}_{\text{span}} = 4.86\times \ge 3.0$) | Enforce `tools/run_elite_3grid_gci_and_rsm.py --verify-existing` on every CFD-promoted elite across all topology families |
| **`F2-Oil`: Full-Residence 3D Coupled Droplet Grade-Efficiency Closure** | 10-bin (`3–100 µm`) inlet-flux-weighted OpenFOAM 14 `kinematicCloud` tracking integrated over full physical residence ($T \ge 4.0\text{ s}$) until retained in-domain oil mass $m_{\text{rem}} / m_{\text{in}} < 0.1\%$ | **OPEN / UNWAIVED GATE** (short `0.01–0.10 s` runs are locked as `BOUNDED_SHORT_WINDOW_TRANSPORT_PROBE_NOT_OIL_FITNESS` with `97.2%` retained inventory) | Close 10-bin mass balance ($|m_{\text{in}} - m_{\text{out}} - m_{\text{rem}}| / m_{\text{in}} < 10^{-4}$ at $m_{\text{rem}}/m_{\text{in}} < 10^{-3}$) across `G1`/`G2` and propagate grade uncertainty to $C_u\text{ [mg/L]}$ |
| **`F3`: Scale-Resolving Transient `DES`/`LES` & Coupled `PBM`** | Transient `IDDES`/`LES` on a 4th prism-layer polyhedral mesh (`G3` $\ge 1.2\times 10^6$ cells) with coupled Population Balance (`PBM`) coalescence/breakup across `D1`–`D4` | **OPEN / UNWAIVED GATE** (pre-registered next computational step before hardware release) | Verify that transient vortex-core precession (`PVC`) and `TPMS`/vane wake turbulence do not re-entrain captured core oil |
| **`F4`: `CAD-09` LPBF Metrology & `P06` Blinded 5-Block `A/B/A` Rig Trial** | $n = 3$ independent LPBF Ti-6Al-4V builds (`A1..A3` `CT-35`, `B1..B3` elite), $25\ \mu\text{m}$ X-ray micro-CT inspection (`CAD-09`), and 5-block (`20`-interval) blinded randomized `ABBA`/`BAAB` flow-loop crossover trial (`P06`) | **OPEN / UNWAIVED PHYSICAL GATE** (`qualified_fitness: null`, `qualified_capacity_gain: null`, `physical_validation: false`) | Micro-CT throat error $\le \pm 50\ \mu\text{m}$, $R_a \le 12\ \mu\text{m}$, zero trapped powder, and paired 95% lower confidence bound $\text{LCB}_{95}(\Delta Q_{\text{treated}}) > 0$ (or $\ge +10.0\%$ for `H10`) at matched $r = 2.00 \pm 0.05\%$ |

