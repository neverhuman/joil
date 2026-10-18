# V5-09 observer contract and applicability

All fixtures are synthetic software checks. These modules measure accepted
artifact bytes, not the authored program. They import no kernel, IR, genome,
lineage or expected-delta API. A low-level function taking `Mesh` or `Volume` is a
diagnostic; `read_surface` and `read_volume` are the accepted entry points. They
require the unforgeable existing `VerifiedSubject`, requested semantic identity,
role, SI format, digest and exact length. Reports retain the artifact's input
approximation separately from represented geometry. They confer no scientific
or manufacturing acceptance.

V5-08's private preview supplies finite metre coordinates and a bounded field
DAG; it explicitly does not certify exact distances. These observers never call
that field evaluator. V5-08/10 must supply a reviewed single partition plus its
derived artifacts. The lane's crate manifest, root registration/lock entry and
`src/lib.rs` are **draft lane scaffolding**, to be dropped during integration.

`observer-cubical-volume/1` is an observer-owned **draft wire format** carried as
`NativeVolumeMesh`, not a claim that an arbitrary native tet mesh is JSON.
The format has uniform SI cubes, x-fastest ordering and one of `outside`, `fluid`,
`solid`, `unresolved` per complete cell. There are at most 4096 cells. Ports are
contract tags mapped to cell sets; no two-inlet coordinates are built in. The
reader checks seed material and transformed cell/contract-plane intersection.
The acceptance review still owns full port patch correspondence and coverage.
General V5-10 native meshes require an explicitly negotiated reader; unknown
encoding is Unsupported. `docs/design/V5_GEOMETRY.md` was absent at lane start.

Volume topology is computed twice over GF(2), by cubical boundary reduction and
by a conforming six-tet subdivision of every occupied cube. Both complexes
include complete closed cells. A ball is `(1,0,0)`; a solid torus `(1,1,0)`.
Boundary homology has a distinct result type: a boundary sphere is `(1,0,1)`.
At most 512 occupied cubes and 20 million sparse-reduction work units are
supported; exhaustion is Unsupported. Unresolved cells prohibit topology.
Point/edge contact may connect a closed complex while the independently
reported six-neighbour fluid reachability/solid attachment is disconnected.
Neither finite complex certifies subcell topology of nominal CAD.

Escape computes widest cell-centre paths after conservative finite-radius
erosion. Distances are to blocked boxes, unresolved cells and the domain edge;
half a pitch and input error are subtracted to cover full connecting edges.
Escape seeds need a represented exterior extension; a seed never opens unknown
space beyond the domain. Reports retain target bottlenecks, witness paths,
isolated components and unresolved cells. Any unresolved cell prevents an
Accessible gate. `inaccessible_site_volume_m3` measures inaccessible centre sites
weighted by cell volume; it is **not a prediction of residual powder mass**.
Real transport, process calibration and cleaning qualification remain external.
A nonpositive powder diameter is invalid.

Triangle-patch distance considers all supplied faces, not a radial proxy. The
nearest-feature estimate and independent normal-line intersections are floating
point diagnostics. Conservative bounds use directed-rounded projections and
feasible vertex distances, widened by the accepted displacement bound per
patch. Loose bounds stay loose: the estimate never upgrades a lower bound.
The request must include all governing opposing wall/ligament/passage faces;
selecting two favorable patches does not certify a whole part. The input
displacement bound must apply to those corresponding patches. Surface and
cubical integrals report represented volume, area, centroid and moments with
metre-based units. They are not nominal-CAD error certificates. Vertex links,
edge incidence, degeneracy and winding are checked; self-intersection absence
and correct nested-shell material orientation still require the artifact review.
Frozen cell-mask differences require aligned grids and the complete registered
region; cross-grid registration or an unprovided nominal region is Unsupported.

The enactment harness develops both signs of every supplied active parameter at
a step exceeding that parameter's representation scale. Geometry comparisons
contain no gene coefficients. A resolved changed centre must be farther than
the combined input bounds from phase boundaries. Missing capability, failed
development, Neutral and Changed remain distinct. Different/unresolved grids or
unresolved changes cannot earn Changed status. The caller must enumerate all
active parameters; this observer cannot inspect a genotype to invent that list.
Mass is integrated solely from the same accepted solid cells and a separately
identified density source. Density qualification is not inferred from its hash.
Physical deduplication hashes partition content and placement, excluding gene
names, lineage and byte formatting. It proves representation equality only;
cross-grid/mesh equivalence needs independent registration and comparison.

Validation: `cargo test --locked -p crucible-geom --test observers` and
`cargo clippy --locked -p crucible-geom --all-targets -- -D warnings`, under the
lane lease. Integration must negotiate the volume encoding and final exports,
then rerun the full repository gates on the integrated head before accepting
V5-09. No lane receipt marks canonical tasks done or promotes physical status.
