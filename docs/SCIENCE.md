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
