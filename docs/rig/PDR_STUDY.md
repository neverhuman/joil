# Pressure drop and split study (V5 draft)

## Frozen boundary contract

Set the total inlet volumetric flux and inlet profile, plus treated-water outlet pressure. Control the reject outlet backpressure to reach a preregistered reachable reject fraction. Do not independently prescribe both outlet pressures or select the result by changing the target after inspection. Report PDR as an observed outcome, with the tap coordinates and elevation recorded separately from the integrated control-volume mechanical energy flux.

Pressure fields must retain their OpenFOAM dimensions. For incompressible `p` with dimensions `[0 2 -2 0 0 0 0]`, multiply by the declared water density to obtain Pa. For pressure dimensions `[1 -1 -2 0 0 0 0]`, the field is already Pa. Reconstruct `p_rgh` with declared gravity and datum. Report gauge pressure when no absolute reference exists; absolute pressure remains unknown and blocks checks requiring it. Static, total and reduced pressure are distinct quantities.

## Controller and sampling

The reject-backpressure controller is bounded by a preregistered pressure range and a reachable split interval. Log time, measured split, command pressure, saturation and settling state for every update. Use anti-windup. An unreachable target, pressure-rail saturation, oscillating split or missing settled window blocks final statistics. Freeze sample count and interval before final sampling. Freeze outlet settings during sampling unless the duty explicitly studies a closed loop. PDR and treated capacity are outcomes.

## Numerical challenge matrix

For one immutable subject and duty, separately challenge: bounded second-order convection against the initializer; independent RSM initialization/continuation; prism-layer count, growth, wall treatment and y+ coverage; both terminal-domain extensions; timestep and pressure/velocity correctors; stress realizability; and stationarity across the frozen sampling interval. Preallocate observable-specific mesh, time, model and sampling error budgets before execution. Do not issue a Richardson estimate for oscillatory or non-asymptotic sequences. Short RSM continuation is not independent validation. A failed mesh-health gate, incomplete fields or unbound native identity produces no accepted result.

## Current evidence status

The adjacent V5 capability card records the pinned Foundation14 package identity and read-only compiler/MPI/build probes, plus SSG/LRR class symbols in the installed library. It does not establish model construction, a converged carrier case or interrupted native restart. The analytic fixtures are synthetic controls. No PDR value or CFD qualification is asserted here.
