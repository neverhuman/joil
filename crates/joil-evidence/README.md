# JOIL evidence kernel

Adapted from `neverhuman/jarambit@c988273`, not a dependency on its working tree.
`UPSTREAM.json` pins source paths, Git blobs and exact-byte SHA-256 digests.

`src/identity.rs` ports ContentRef validation, exact binary64 canonical wire,
strict bounded JSON decoding (duplicate/NFC-colliding keys rejected), scientific
identity and separate runtime metadata. The wire namespace changes to JOIL.
Finite scalar and observation/claim semantics are adapted from upstream
`assurance.rs`; JOIL units, geometry stages and claim dependencies are separate
contracts. Verdict states, schedule-independent samples and hash-chained events
retain upstream's fail-closed patterns. JOIL does not port blade requirements,
material allowables, search booleans, compressed JEV1 logs or release authority.

Content hashes prove identity, not truth or authorization. Trusted inventory and
review authority must be supplied by the application; incoming artifacts cannot
appoint themselves. Missing independent evidence remains incomplete.

`schemas/observation-v1.schema.json` is the shared strict interchange shape.
Rust uses checked serde decoding; `analysis.evidence.ingest_observation` enforces
the same schema and semantic invariants. Cross-language tests round-trip a measured
record and compare acceptance of planted unit/state/source/uncertainty mutants.
Non-measured variants carry reasons and cannot project to a performance number.

`campaign::Requirements` reads the frozen TOML. Its five objectives preserve the
original Wave-2 preregistration; runtime values pass through the declared transforms.
The current contract admits sandbox work only. Physical limits and validation
inputs are missing, so claim-bearing admission fails even if a caller supplies
passing P0 receipts. `tools/gen_status.py --check` runs in the existing Python CI
suite and rejects stale status, changed evidence, objective drift and mismatched
new `runs/*/campaign.json` receipts. Legacy status is historical evidence only.

`subject::CaseManifest` binds OpenFOAM/FEA to compiler, IR, geometry, both meshes,
case inputs, solver, model, scenario, requirements, objectives, material/process
and extraction context. The solver-case digest covers the solver input payload
excluding this enclosing manifest, avoiding a self-referential hash. Writers must
verify exact artifact bytes; a missing mesh or case identity fails closed.
