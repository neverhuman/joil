# Chemical engineering process networks

This is an implementation annex to [the master plan](../MASTER_PLAN.md), covering
the first executable foundations for discovering different processes inside a
cartridge. It adds genuine three-dimensional channel volumes and conservative
composition of fixed stage responses. The geometry and transport APIs are
independent building blocks: this change does not connect them to the production
genome, mesh exporter, hydraulic solver, or physical fitness pipeline.

The most useful early architecture experiments are gentle inlet conditioning,
protected oil collection, real coalescing structures, unequal stages in series,
and parallel cells sharing inlet/product/reject manifolds. Useful architecture
must survive the full chain from geometry to droplet transport and experiment.

## Run the synthetic examples

All commands run from the repository root and write diagnostics to stdout. The
fixtures contain invented response coefficients and research geometry. They are
not measured devices, fitted CFD models, CT-35 variants, or performance claims.

```bash
cargo run --locked -p pack-hydrocyclone-judges --example process_network < tests/fixtures/process-network/series.json
cargo run --locked -p pack-hydrocyclone-judges --example process_network < tests/fixtures/process-network/recycle.json
cargo run --locked -p joil-geometry --example channel_network < tests/fixtures/process-network/branch-rejoin-geometry.json
```

The transport example exits 0 when traversal completes or reaches its declared
tail tolerance, 2 when the work budget is exhausted, and 1 for invalid input or
arithmetic failure. Exit 0 is software completion, never physical qualification.
Every report retains an unresolved physical verdict. The geometry example emits
a recipe identity, field identity and the actual shared implicit-field program;
it grants no mesh, manufacturing or scientific admission.

The examples cap input bytes. Library validation separately bounds dimensions,
stage/route counts, matrix terms, geometry segments, field nodes and traversal.

## Why stage composition matters

For stage-relative liquid reject fractions r1 and r2, with the first product
feeding the second:

```text
product volume = fresh volume * (1-r1) * (1-r2)
external reject fraction = 1 - (1-r1) * (1-r2)

For unchanged size classes:
remaining oil in class d = input_mass(d) * (1-E1(d)) * (1-E2(d))
product concentration [mg/L] = 1000 * product_oil_mass [kg] / product_volume [m3]
```

Two 2% stage-relative splits produce 3.96% total reject. If both synthetic stages
capture 80% of the oil they receive, a 500 mg/L feed gives approximately
20.825 mg/L product. These numbers only illustrate accounting. Each real stage
must share the total pressure and reject budgets; it cannot receive an extra
pressure allowance because another chamber was inserted.

The second stage receives the surviving distribution. For two equally weighted
classes and repeated captures of 10% and 90%, the remaining oil fraction is
0.5*(0.9^2+0.1^2)=0.41. Multiplying overall average remainders would incorrectly
give 0.25. A size-changing stage also makes ordering important.

## Fixed response transport contract

`pack_hydrocyclone_judges::process_network` represents a directed network of
stages with multiple output routes, multiple fresh feeds, shared product/reject
terminals, and retained/wall inventory. Physical edges may form cycles.

All stages use the same increasing diameter edges in metres. Oil is represented
as mass in each diameter interval. Each route has:

- A fraction of the incoming bulk liquid volume, supplied by an external source.
- A matrix whose row is an inlet size class and column an output size class.
- A destination: another stage, product, reject, retained inventory, or wall.

Matrix entries are unconditional fractions of inlet oil mass. They are not
multiplied again by the liquid fraction. Summing over every route and output
class must conserve each input class's mass. Off-diagonal entries represent a
fixed size transformation, for example a prescribed breakup response. They do
not predict breakup or concentration-dependent coalescence.

The source digest binds the supplied response identity; it does not validate
its values. Only synthetic-fixture and unqualified-linear-response evidence
classes are accepted. A caller cannot request a qualified variant. Unknown wire
fields, incompatible dimensions, negative/nonfinite coefficients, unconserved
kernels, unreachable stages, and unbounded budgets fail explicitly.

Splits are prescribed inputs to this accounting layer. They must eventually be
obtained from the connected pressure/flow problem. Internal ports cannot become
free pressure reservoirs, independently adjustable suction boundaries, or
locations that reset swirl, turbulence or droplet size. This scalar cohort API
does not preserve a spatially resolved interface state and therefore is not a
replacement for full-liner CFD or the existing head/tail coupling contract.

## Recycle and unresolved inventory

Evaluation propagates a finite fresh cohort through first-passage routes.
Product/reject cohorts stop at their first terminal destination. Each additional
stage visit is recorded and charged, so internal reprocessing cannot be counted
as additional fresh-feed capacity.

Traversal stops when no circulating state remains, both oil and volume tails
are within their relative tolerances, or a work budget is reached. It always
retains the actual circulating volume and oil. It never renormalizes terminal
outputs to hide an unfinished tail. A fully closed loop exhausts its budget
with all fresh inventory still present.

Retained and wall oil are not captured oil. Product concentration bounds assign
circulating, retained and wall inventory adversely: the lower bound allows all
unknown liquid to become product water, and the upper bound allows all unknown
oil to become product oil. No observed product volume means no finite reported
concentration upper bound. External reject also includes an inventory interval.
These are algebraic allocation bounds, not interval-arithmetic certificates.
Signed floating-point balance errors remain in the report; accumulated drift
beyond the numerical accounting tolerance is an error, never silently repaired.

`complete` means all live traversal reached a terminal or explicit inventory
state. It does not mean retained oil vanished, a steady flow was reached, or a
candidate passed. The fixed response model has no residence-time, pressure,
thermal, gas, fouling or time-dependent storage model. Loop/retention behavior
outside this contract requires a coupled dynamic calculation.

## Genuine 3D interior geometry

`joil_geometry::channel_network` builds a fluid field from spherical, box and
capped-cylinder chambers connected by arbitrary-axis circular sweeps. A channel
can have multiple waypoints, and the network can contain unequal serial
chambers, parallel passages, branches, rejoining paths and closed returns.
Every segment produces an actual capsule volume with spherical endcaps. Changes
to radius and path change the field itself; display names do not affect shape.

The field is consumable by the existing `MaterialPartition`, which owns the
fluid/solid distinction. It is not restricted to one wall radius R(theta,z).
The diamond fixture keeps solid material between the two paths, and the tests
check this independently of the authored channel count.

This is a piecewise-linear centreline sweep, not a smooth curvature-controlled
duct or general lattice compiler. Chamber and channel overlaps are represented
by their real set union. Crossing channels may therefore connect even when their
names are different, and disconnected volumes are representable. Independent
topology, gap, thickness, powder-removal, frozen-interface and mesh observers
must evaluate the resulting artifact before any admitted downstream use. The
compiler does not report authored connectivity as measured connectivity.

Capsules have exact-distance semantics in real arithmetic and a Lipschitz bound
of one. Their unions have exact set semantics but are not generally exact
signed-distance fields. Pointwise floating evaluation is still subject to the
kernel's error and operation-budget rules; a real-arithmetic formula is not a
floating-point geometric certificate. Finite sampling tests prove only their
stated numerical fixtures.

## Representation and physics work that follows

| Chemical engineering concept | Required follow-through |
|---|---|
| Gentle inlet and gradual swirl | Smooth sweeps, real vane geometry, measured droplet stress histories and oil-grade transport. |
| Progressive oil extraction | Separate collection channels, pickup apertures and one shared global reject/pressure problem. |
| Coalescence before separation | Real contactor sheets or lattices, characterized surface behavior, concentration-dependent population transport and downstream survival. |
| Unequal serial stages | Conditional inlet populations, coupled interstage flow and joint pressure allocation. |
| Parallel numbering up | Geometry and hydraulic co-optimization of distributor, cells, product and reject manifolds. |
| Thin annular passages | Independent minimum-gap and printability observations, actual shorter migration paths and wall-loss accounting. |
| Selective recycle or bypass | Physically resolved enrichment and return-pressure paths; count all internal processing. |
| Countercurrent routing | Protected reverse-core paths and bidirectional interface exchange; axial counterflow is different from opposite swirl rotation. |
| RTD and core stabilization | Size-specific first-passage histories, short-circuiting, stagnant inventory and vortex precession. |
| Pressure recovery | Mechanical-energy flux at both terminals; static pressure recovery cannot restore irreversible losses. |
| Wettability, finish and grading | Measured surface/material passports, as-built geometry, fouling and cleaning challenges. |
| Acoustic/electrical assistance | Separate multiphysics and power budgets; distinguish oil-in-water from water-in-oil evidence. |
| Heating or chemical conditioning | New feed and energy/consumable boundaries; equally conditioned comparator. |
| Flotation, membranes or sorption | Explicit gas/permeate/regeneration streams and separate qualified model scopes. |

Retain the existing head-retrofit contract and create a separately identified
full-cartridge scope before changing the frozen tail or external interfaces.
Most monolithic multistage architectures need that second scope. Hybrid systems
with power, chemicals or extra ports need a separately declared system boundary.

Add coordinated genome operations for inserting a fluid stage, splitting a
path into parallel cells, moving a collector, introducing a return and merging
regions into a continuous field. These operations must lower to geometry and
be audited before an evolutionary run can claim to search that capability.
Construction references stay acyclic while physical transport may recirculate.

Use chemical engineering to propose candidates. Score them using observed
physics and hold-out comparisons, preserving opportunities for unfamiliar
topologies. Neither a named coalescer nor a named TPMS layer should receive an
automatic efficiency bonus.

## Verification and CI

The existing Foundation `rust-tests` suite uses `cargo test --workspace --locked`
and discovers both new integration test targets. No extra workflow or weakened
gate is needed. The examples compile as part of Cargo's normal test build.

```bash
cargo test --locked -p joil-geometry --test channel_network
cargo test --locked -p pack-hydrocyclone-judges --test process_network
cargo clippy --locked -p joil-geometry -p pack-hydrocyclone-judges --all-targets -- -D warnings
```

Controls cover independent segment distance, rigid transforms and scale,
branch/rejoin solid ownership, unequal chambers and actual return geometry;
conditional populations, size changes, weighted mixing, passive tracers,
bounded recycle, wall inventory, invalid kernels and strict input formats.
These are software and analytic numerical controls. The original physical
qualification, capacity and manufacturing gates remain in force.

## Research references

- [Al-Shimmery et al., 2019: real printed TPMS contactors and droplet coalescence](https://doi.org/10.1016/j.memsci.2019.117274).
- [Associated Bath contactor dataset](https://doi.org/10.15125/BATH-00641), a future benchmark acquisition task; not imported or fitted here.
- [Zhang et al., 2025: hydrocyclone coupled with fiber coalescence](https://doi.org/10.1016/j.cjche.2025.02.008); the mechanism can favor growth or breakup depending on the feed.
- [Zhan et al., 2021: experimental axial separator with multiple outlets](https://doi.org/10.3389/fenrg.2020.608586); its concentration regime differs from JOIL's dilute envelope.
- [Bram et al., 2020: control-oriented modeling and experimental validation](https://doi.org/10.3390/pr8091010).

The broad coalescer/hydrocyclone combination is established prior work. Any
novelty claim should identify the discovered geometry, coupled architecture and
independently demonstrated benefit rather than the generic combination.
