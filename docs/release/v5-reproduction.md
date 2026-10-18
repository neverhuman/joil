# V5 reproduction and review bundle policy

## Offline public bundle

The allowlisted public bundle consists of this document, `experiments/v5-study/` protocol descriptors, `paper/` query definitions and synthetic fixtures, and the `crucible-paper` source with its locked workspace dependency graph. Reproduce software checks offline with:

```sh
CARGO_INCREMENTAL=0 RAYON_NUM_THREADS=1 cargo test --offline --locked -p crucible-paper
CARGO_INCREMENTAL=0 RAYON_NUM_THREADS=1 cargo run --offline --locked -p crucible-paper --bin paper-build
```

The paper builder supports a negative result; successful build means only that every typed claim resolves to a current matching observation. It does not imply a scientific PASS.

## Private governing evidence

Retain raw measurements, customer/site adapters, full campaign ledgers, rejected finalist evidence, licensed source material and review correspondence in the private evidence store. Public bundles contain only explicitly allowlisted, disclosure-reviewed derived artifacts with hashes and license records. Never stage raw private evidence to produce a complete paper.

## Dependency, license and prior-design inventory

Record every direct/transitive package version and license from Cargo.lock and the applicable upstream license files; record solver/render tool build identities separately. For each candidate, inventory prior-design sources, their dates, accessions and geometry/claim overlap. This engineering inventory supports review only. It makes no legal novelty, freedom-to-operate, commercial-superiority, service-qualification or patentability determination. Descriptor-hull escape is computational novelty only.

## Scientific boundary

Every output carries exact subject, scenario, estimand, unit, denominator, evidence tier and interpretation. Independent n is the preparation/build or independent search seed specified by the estimand; parcels, frames, meshes and generations are not independent replicates. Missing or stale tiers fail closed. Negative results and all attempted finalists remain in the family ledger.
