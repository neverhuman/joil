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
