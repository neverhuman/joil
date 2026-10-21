# Forge physics: what the screen computes, and how far to trust it

The forge judges (`crates/joil-forge-judges`) score a *physical description* of a plant:
hydrocyclone cells, lattice coalescer beds and flow splitters wired into a directed
network. The judges never see a genome, a family label or a design history. Every number
they return carries the evidence class `unvalidated_reduced_order_screen`. It is a
search screen with declared assumptions, not CFD, not a measurement, and not physical
validation. The repository's evidence ladder in [`docs/SCIENCE.md`](../SCIENCE.md) still
governs every claim.

This page lists every equation and closure the screen uses, the numerical controls that
check it, and the places where it is known to disagree with other evidence.

## 1. Stage hydraulics: swirl that decays for a physical reason

For one cell with feed `Q`, overflow split `r` and a piecewise-linear wall `R(z)`:

| Quantity | Model |
|---|---|
| Inlet jet velocity | `v_in = Q / A_in` |
| Initial circulation | `Γ₀ = α v_in r_inj`, with `α = 1 − k_α (b/R₀)^½` (slot-inlet form; involute entries use `0.7 k_α`) |
| Forward flow | `Q_f(z) = Q − rQ z/L` (the reverse core is fed uniformly along the cell) |
| Swirl decay | angular-momentum balance with wall shear `τ = ½ c_f ρ (Γ/R)²`: `dΓ/dz = −π c_f Γ² / Q_f` |
| Wall friction | `c_f` = Haaland Darcy factor / 4 at `Re = 2Γ/ν` and printed roughness 20 µm, times a closure multiplier |
| Vortex | Burgers profile `v_t(r) = Γ/r · (1 − e^{−r²/r_c²})`, `r_c = κ R` |

The decay law has no free rate constant. Narrow passages, rough walls and long tails all
lose swirl through the same friction term, and a contraction spins the flow up because
`Γ` is conserved while `R` falls. The step is integrated exactly for frozen `c_f` and
`Q_f` (`Γ ← Γ / (1 + π c_f Γ Δz / Q_f)`), so it cannot overshoot.

**Pressure is an energy budget, not a fitted Euler number.**

* Underflow: `Δp_u = ½ρv_in²[K_c + (1−α)²] + ∫ π c_f ρ Γ³/R² dz / Q_f + axial friction + ½ρ⟨v_t²⟩_exit + ½ρv_z,exit²`.
  The swirl and axial kinetic energy that leave through the underflow are dissipated
  downstream, so they count as losses.
* Overflow: `Δp_o = ½ρv_in²[K_c + (1−α)²] + ½ρ(Γ₀/R₀)² + ρΓ₀²/(2r_c²)·∫₀^{R₀²/r_c²}(1−e^{−u})²/u² du + ½ρv_o²`.
  The third term is the Burgers core pressure deficit (it tends to `ρΓ²ln2/r_c²`); the
  last is the discharged orifice jet.

## 2. Separation: drift–diffusion in the flow-fraction coordinate

The forward flow occupies `λR ≤ r ≤ R`; drops that reach the capture surface `r = λR`
join the reverse core and report to the overflow. The march uses the flow fraction
`s = (r² − r_cap²)/(R² − r_cap²) ∈ [0, 1]` and the oil mass flux `n(s, z)` of each size
class:

```text
∂n/∂z = −∂/∂s[(a_f + a_d) n] + ∂/∂s[K ∂n/∂s]
a_f = −(1 − s) q'/Q_f                    fluid drift that feeds the overflow, q' = rQ/L
a_d = −2π r v_slip / Q_f                  drop slip (oil is lighter, so inward)
K   = 4π r² D_t / ((R² − r_cap²) Q_f)     turbulent diffusion, D_t = c_D Γ
v_slip = Δρ d² (v_t²/r) / (18 μ) · m(κ) / (1 + 0.15 Re_p^0.687)
```

`m(κ)` blends a rigid interface (surfactant-loaded produced water) with the
Hadamard–Rybczynski mobile limit. The core boundary is an advective outflow and the wall
has zero flux. Implicit upwind finite volumes conserve each class to round-off.

Two properties follow by construction and are tested:

* **A passive tracer reports exactly the split.** With implicit forward flow,
  `n_{k+1} = n_k Q_f(z_{k+1})/Q_f(z_k)` is exact, so `E(d → 0) = r` to machine precision.
  This is the SCIENCE.md oracle "a passive tracer has E = r and Cu = Ci".
* **No negative or created mass.** The matrix is an M-matrix with unit column sums except
  at the outflow.

The reduced cut size `d50′` (50 % of `(E − r)/(1 − r)`) is reported per stage.

## 3. Drop breakup wherever energy is dissipated

Maximum stable diameter (Davies 1985, Hinze–Kolmogorov with dispersed-phase viscosity):

```text
d_max = C [ (σ + 0.08 μ_d ε^{1/3} d_max^{1/3}) / ρ_c ]^{3/5} ε^{−2/5}
```

solved by fixed point, with a viscous-subrange branch below the Kolmogorov scale. Breakup
is an equal-volume split into the fewest stable fragments plus a satellite fraction at
`d_max/6`; mass moves between classes and is never created. The dissipation rate `ε` is
evaluated at every place that destroys mechanical energy:

| Location | `ε` |
|---|---|
| Cyclone | `max(ε_inlet, k_peak · ε_mean)`; `ε_inlet` = inlet loss power over a jet volume `c_jet n A_jet^1.5`; `ε_mean` = all dissipation over the cell volume |
| Channel | friction and minor losses over the channel volume |
| Throttle | the pressure a stream must shed to merge, dissipated in its own jet |
| Recycle pump | shaft power over an impeller volume |
| Coalescer bed | Ergun pressure drop × superficial velocity over pore volume |

This is why a throttle between two stages, a pump in a recycle, or a very fast inlet can
make a plant worse even though it looks better on paper.

## 4. Coalescer beds

Printed lattice beds with struts `d_f ≥ 1 mm`: Ergun pressure drop with equivalent particle
diameter `1.5 d_f`; capture by Kuwabara-flow interception `η_R = (1−α)/Ku · R²/(1+R)` and
inertial impaction `η_I = St²/(St+0.25)²`; bed efficiency `E = 1 − exp(−4αηL/(π(1−α)d_f))`.
Captured oil drains as drops of `c_rel d_f`, limited by the bed's own `d_max`. With
millimetre struts, interception of 10–20 µm drops is weak. That is a physical result the
search is allowed to discover, not a bias.

## 5. Networks: series, parallel, recycle, pumps

* **Flows.** `(I − A) Q = b` by dense elimination with pivoting. A closed loop that
  retains flow is a singular system and is reported as a hydraulic failure.
* **Pressures.** Each active unit needs `p_in(u) = max_k [Δp_k(u) + Δp_channel + p_req(dest_k)]`.
  Bellman–Ford relaxation finds the fixed point; an unpumped loop with positive losses never
  converges and is reported as a passive loop. A pumped link only needs its suction
  pressure. Surplus pressure on any stream is shed in a throttle, which then breaks drops.
* **Pressure budget.** The budget applies to the feed pressure plus pump shaft work per
  unit of feed volume. A booster between stages therefore cannot buy extra separation
  for free: every stage shares one budget, as the master plan requires.
* **Oil.** Gauss–Seidel propagation of size-class mass vectors through each port's
  transfer operator (breakup, then capture) and each link's breakup operator, in
  depth-first order. Convergence is checked against the feed mass; non-convergence is a
  numerical failure, never a physical verdict. The oil balance closes to 1e-8 or the
  evaluation fails.
* **Outlet concentration.** `Cu = 1000 m_product / Q_product` (mg/L), which reproduces
  `Cu = Ci(1 − E)/(1 − r)` for a single stage.

## 6. Capacity: the objective the repository already defined

`Q*` is the largest **tested** feed flow at which the plant meets the quality limit with
pressure inside the budget. The pressure-limited flow `Q_p` comes from bisection on the
hydraulic solve; quality is probed at `Q_p · 0.85^j` with no interpolation across failed
or untested gaps. Treated capacity is `Q*(1 − r_total)`. When nothing complies, capacity
is zero and the search ranks by the log shortfall `ln(Cu/limit)`.

**Off-design feeds.** Inlet drop sizes move with upstream shear, so a campaign lists
off-design feeds (default: the reference feed with an 8 µm oil-volume median). Each runs
once at the rated flow `Q*` under the nominal closure, and the margin
`m = min over scenarios of min(1, limit/Cu)` enters the objective as `ln m`: missing the
limit by a factor `k` costs as much as losing a factor `k` of capacity. A pass/fail form
of this term (capacity search on the scenario, credit or nothing) was tried first and
removed: search parked designs exactly on the limit, where a 0.5 % change in `Cu` between
resolutions flipped the verdict. The continuous margin has no such cliff.

## 7. Closures and their declared ranges

Every constant has a physical meaning and a range. Search scores each candidate under the
nominal closure plus Latin-hypercube samples that are shared by every candidate in a run
(common random numbers). A design is **feasible only if it complies under every sample**,
and its capacity is the minimum over samples. Anything that only wins for one convenient
closure value is not rewarded.

| Closure | Nominal | Range | Meaning |
|---|---|---|---|
| `inlet_alpha_k` | 0.40 | 0.30–0.55 | inlet swirl loss |
| `inlet_contraction_k` | 0.25 | 0.10–0.50 | inlet contraction loss |
| `swirl_friction_mult` | 1.0 | 0.8–1.6 | swirling-wall friction enhancement |
| `core_radius_frac` | 0.27 | 0.20–0.34 | Burgers core radius / wall radius (anchored) |
| `capture_radius_frac` | 0.45 | 0.35–0.55 | reverse-flow capture surface / wall radius |
| `eddy_diffusivity_coeff` | 0.0008 | 0.0004–0.0016 | `D_t = c Γ` |
| `interface_mobility` | 0 | 0–0.5 | rigid to Hadamard–Rybczynski |
| `davies_c` | 0.66 | 0.55–0.85 | breakup constant |
| `peak_dissipation_ratio` | 15 | 5–40 | peak / mean dissipation in a cell |
| `jet_volume_factor` | 5 | 2–10 | jet mixing volume |
| `satellite_fraction` | 0.05 | 0–0.15 | satellites per breakup |
| `coalescer_capture_mult` | 1.0 | 0.3–1.5 | collector efficiency multiplier |
| `coalescer_release_factor` | 2 | 1–4 | released drop / strut diameter |
| `pump_dissipation_volume_m3` | 1e-4 | 3e-5–3e-4 | pump shear volume |

**Anchors.** `core_radius_frac` was chosen so that the CT-35 reconstruction reproduces the
committed carrier-CFD overflow drop (model 211 kPa vs the 208,358.5 Pa receipt at 4 m³/h,
within 2 %). `eddy_diffusivity_coeff` gives CT-35 a reduced cut size of about 10.7 µm,
inside the range usually reported for 35 mm deoiling liners. Both anchors are numerical
or literature anchors, not validation.

**Known disagreement.** The model's CT-35 underflow drop is about 2.3 times the carrier-CFD
static tap drop (68 kPa vs 29,468.22 Pa). The energy budget charges wall friction plus the
swirl and axial energy discharged at the underflow. The CFD itself differs between
pipelines (the same-pipeline control reads 18.63 kPa). The discrepancy is declared, kept
under a test (`ct35_overflow_drop_reproduces_carrier_cfd_anchor`), and left unresolved.
For CT-35 the feed pressure is set by the overflow side, so the capacity ranking is
driven mainly by the anchored branch.

## 8. Applicability envelope

The model refuses (error class `applicability`, never "infeasible") cells whose walls
converge faster than 25° or diverge faster than 8° half-angle, where separation, vortex
breakdown and contraction losses would dominate. The genome repairs segments into this
envelope (24.5° / 7.8°) during normalisation, and the repair is part of the canonical form.
Everything else in the repository's envelope (dilute oil, single-phase carrier, 2.6–5.4
m³/h reference band, LPBF surfaces) applies unchanged; the forge's capacity search may
probe flows outside the reference band, and those numbers are extrapolations.

## 9. Numerical controls

| Control | Result |
|---|---|
| Passive tracer = split (CT-35 and scaled cells, four splits) | error below 1e-12 |
| Per-class mass balance of the march | below 1e-12 |
| Grade efficiency monotone in diameter, bounded by `[r, 1]` | holds |
| Series composition uses the conditional (surviving) distribution, not averaged remainders | exact to 1e-9 |
| Splitter recycle loop: product concentration equals feed | exact to 1e-9 |
| Unpumped recycle detected as a passive loop | holds |
| Feed pressure scales close to `Q²` | ratio 3.3–4.4 for 2× flow |
| Search vs 16× finer march (CT-35, a 0.3-scale cell, and a 24.5° cone into a 5.8 mm throat) | capture within 0.5 % |
| Closure corners (all 0, all 1, all 0.5) evaluate | holds |

The throat case exists because an early search exploited an under-resolved axial march:
a design that looked robustly compliant at the search resolution was not compliant at a
finer one. The march now places a node at every wall station and limits each step to the
local diameter, and every robustly compliant candidate is re-scored at the report
resolution before it can count.

Run them with `cargo test -p joil-forge-judges`.

## 10. Same-model CT-35 control

Under this screen, at the reference duty (500 mg/L, oil-volume median 15 µm, GSD 1.6,
2 % split), the CT-35 liner reaches about 158 mg/L at 4 m³/h (capture 0.69, `d50′`
10.7 µm) and about 120 mg/L at its pressure-limited flow. It has **no compliant tested
flow** at the 30 mg/L research limit, so a capacity gain over CT-35 is undefined, exactly
as SCIENCE.md requires ("If no feasible baseline exists, gain is undefined"). Compare
designs on treated capacity, on `Cu` at matched flow, and on energy; do not divide by
zero. Reproduce with `joil-forge baseline`.

## 11. What is not modelled

Gas, solids, fouling, concentration-dependent coalescence in the bulk, time dependence,
vortex precession, short-circuit flow under the roof, structural loads and print
distortion. The layout's channel routing and plenum shapes enter the physics only through
channel length, diameter and losses. None of the forge's outputs closes any of the open
gates in `docs/STATUS.json`.
