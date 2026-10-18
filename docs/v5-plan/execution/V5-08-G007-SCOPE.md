# G007 geometry path reconciliation

The direct G007 assignment names `crates/joil-geometry` for the implicit kernel; the frozen plan and parallel lanes name `crucible-geom`. There will be one implementation of shared field types in joil-geometry and a compatibility crucible-geom crate that re-exports those exact types. Observer/mesh leaf modules remain in crucible-geom, owned by their assigned lanes. This avoids incompatible duplicate Point3/FieldProgram/ImplicitEval authorities. Integrator owns both manifests, roots, lockfile and registration. Lane scaffold manifests/roots are draft inputs, reconciled during integration.

V5-08 gains the requested field/CSG/interval/chamber-wall files, explicit budgets and tests. Its five original acceptance criteria and negative controls remain unchanged: pinned candidate comparison, disjoint ownership, computed bounds, frozen interfaces and traceable conformance. New file names and unit tests alone cannot close those criteria. ChamberWallProposal is absent from the current tracked Rust source; the new type will be an explicit checked input contract with source-bound legacy conversion, never an assumed existing API.

Negative controls cover invalid DAGs/coordinates, depleted budgets, unsupported arithmetic, backend disagreement, overlapping/gapped ownership, ambiguous interval cells, wrong frozen interfaces and identity/conformance drift. No physical campaign or distance/topology claim follows from a sampled level set. Any unmet criterion remains review/blocked with its precise limitation.

Addendum 2 permits downstream code against this draft API. It does not grant task acceptance before integrated tests and receipts. No concurrent writer may modify the integrator-owned shared roots or kernel implementation in this checkout.
