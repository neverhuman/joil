# Genome v4 design: PROPOSED, not implemented

**Status: PROPOSED.** No type, function or crate named here exists in this repository, unless
[`docs/GENOME.md`](../GENOME.md) says otherwise. The implemented genome is the `DesignProgram` macro language
described there.

This design is a workplan §6 follow-on ([`docs/WORKPLAN-2026-10-15.md`](../WORKPLAN-2026-10-15.md)). The
current PR delivers only the Milestone-0 truth repairs and foundations:

- evidence kernel (EV);
- DesignIR v1 skeleton for the *current* features (IR-01);
- measured F0 (IR-08).

Sources: this file consolidates the genome proposals of the six 2026-10-15 reviews
([`docs/review-inputs/2026-10-15/`](../review-inputs/2026-10-15/INDEX.md)) and the earlier "future
representation" sketch that used to live in `docs/GENOME.md` (§5 below). Section references are to those
reviews:

| Key | Proposal name | Main sections |
|---|---|---|
| S1 | typed morphogenesis program (`DesignProgramV4`, `MorphExpr`, `FlowGraph`) | S1 §5–§7 |
| S2 | Alien Design Genome `joil-adg-v1` (typed program graph, field language, channel network) | S2 §5–§10 |
| S3 | Xenogen `GenomeV4` (typed fluid graph with innovation IDs, field language, developmental grammar) | S3 §4–§7 |
| S4 | TDFG, Typed Developmental Field Graph | S4 §3–§6 |
| S5 | `AlienGenome` v4 (graph, field, material, process and control programs; minimal-start complexification) | S5 §6–§9 |
| S6 | XenoGenome `GenomeEnvelope` (interface spine, typed topology graph, field programs, bounded grammar) | S6 §5–§7 |

## 1. Why a new genome

All six reviews say adding fields to `DesignProgram` cannot produce open-ended invention:

- The current type fixes the number and kind of mechanisms.
- The topology label is computed from which fields are active.
- Several genes affect the score without changing the simulated geometry (`docs/GENOME.md` §2).

The proposed change is a language whose structures the authors did not name in advance. Each structure
compiles into one exact physical subject and is judged by an evaluator that never sees the genes (S1 §19,
S4 §20, S5 §26).

## 2. Consensus core (all six reviews)

1. **Immutable interface contract.** Ports, frozen regions, keep-outs, envelope, units, frames and tolerances
   are one content-addressed artifact. They are never genes (S2 §7.8, S3 §4.2, S4 §3.3, S6 §5.2).
2. **Variable typed graph.** Modules, channels, junctions, chambers, collectors, porous and solid regions are
   connected through typed ports: domain, frame, section, continuity, units and conservation contract.
   - N→M branching, rejoin, loops and nesting are data, not enum variants (S1 §5.3, S3 §4.3, S4 §3.2, S6 §5.3).
   - Incompatible edges are rejected before any geometry is built.
3. **Bounded field expression DAG.** Coordinates, distances, Fourier/RBF/B-spline/TPMS bases, smooth Booleans,
   warps, band-limited noise and optional small CPPN nodes (S1 §5.4, S2 §7.5, S3 §4.5, S4 §3.4, S6 §5.5).
   - Every node declares units, range, smoothness, maximum frequency and a Lipschitz bound.
   - Network weights are content-addressed. No arbitrary executable code.
4. **Bounded developmental rewrite grammar.** Deterministic, with a canonical rule order and gas-metered
   budgets: nodes, edges, rule applications, field nodes, minimum feature scale and compiler cost. Exceeding a
   budget is an explicit rejection, never truncation (S3 §4.6, S4 §3.5–3.6, S5 §6.6, S6 §5.6).
5. **Material and process programs.** Explicit even while frozen. A manufacturing realization has a different
   identity from the nominal geometry (S1 §12, S3 §4.7, S6 §5.7).
6. **Strategy genes outside physical identity.** Self-adaptive step sizes and operator logits live in lineage
   state, so two identical designs do not get different subject hashes (S2 §7.1, S3 §4.8).
7. **Innovation IDs.** Homologous crossover aligns structure by ancestry and type. Non-homologous operators are
   labelled as such; speciation protects new structures for a tuning window (S3 §7.4, S5 §9.2, S6 §7.4).
8. **Layered identity.** Genotype hash ≠ development-trace hash ≠ DesignIR/phenotype hash ≠ backend artifact
   hashes ≠ mesh/case hashes. Latent or inactive genes change the genotype hash only (S2 §8.4, S3 §5.4,
   S5 §4.6).
9. **Exact numeric policy.** Exact binary64 serialization, or a declared fixed-point quantum per field. No
   silent rounding inside normalization (S2 §8.1, S4 §4.2).
10. **One compiler to one DesignIR.** The IR carries a semantic origin trace for every face, cell region and
    material region.
    - Every backend (B-rep, SDF/voxel, CFD, FEA, preview) consumes the IR and declares per-node coverage:
      `Exact`, `ConservativeApproximation`, `VisualizationOnly` or `Unsupported`.
    - An active node that is `Unsupported` blocks qualification (S1 §6, S3 §5, S5 §7, S6 §6).
11. **Blind judge.** Evaluators receive the subject bundle, scenario and evaluator configuration only. They see
    no gene names, topology labels, mechanism descriptors or generator identity. Two genomes with the same
    phenotype get the same result (S1 §9.1, S4 §2.3, S6 §9.1).
12. **Named mechanisms become macros.** Ramp, split, ribbon, TPMS, skimmer, chamber and cone lower to generic
    primitives. The eleven legacy labels become seed recipes and regression fixtures (S2 §7.3, S4 §3.2, §18).

## 3. Points where the reviews differ (decisions not yet taken)

| Question | Positions | Recorded default |
|---|---|---|
| Which geometry is authoritative | S6: B-rep is primary and voxel/SDF audits it. S3: neither alone; qualification needs agreement. S1/S2: promotion needs agreement within tolerance. | none; decide before v4 compiler work |
| Domain-neutral core | S5/S6: domain-neutral engine with a hydrocyclone plugin. S1/S3/S4: hydrocyclone-first. | none |
| Single "alien" number | S6 allows a reporting-only `AlienIndex` (geometric mean × evidence-survival factor). S1, S3, S4, S5 reject any blended score. | vector only (workplan §0 item 11) |
| Fidelity ladder labels | F−1…F5 (S4), F0–F6 (S5), E-2…E4 (S3), E0–E6 evidence plus N0–N5 novelty plus U0–U4 utility (S5) | F−1…F5 (workplan §0 item 11) |
| Evidence code from Jarambit | S6: shared domain-neutral crate. Workplan: copy into `crates/joil-evidence` with provenance. | copy (workplan §5 decision 3) |
| LLM proposals | S1/S2/S3/S6: allowed only as typed patches through the same compiler, with model identity in lineage; never a judge. | allowed under those limits; judge is never an LLM |

## 4. Acceptance tests a v4 implementation must pass

From the reviews' test catalogues. None runs today.

- Exact CT-35 zero-mutation round trip through genome, development and DesignIR (S3 §16-1, S6 §15.1).
- New-family challenge: a viable topology outside every seed label with **no source edits**
  (S5 G-010, S2 ADG-EXP-010, S4 GEN-009).
- A legacy six-jet design compiles to six distinct, connected, inspectable jet passages in the CFD domain
  (S4 §18 Phase 4, S1 §13.2).
- Every active node maps to at least one IR entity or is removed canonically; a gene that changes only a label
  fails (S1 §13.2).
- B-rep versus SDF agreement:
  - volume ≤ 0.2 %, area ≤ 0.5 %;
  - Hausdorff ≤ min(0.05 mm, 5 % of minimum feature);
  - exact agreement on component count and topology invariants (S2 §18.2).
- Unknown node or operator fails closed with its ID and registry version. Development always terminates within
  budget (S5 G-004, G-006).
- Zero invalid-to-baseline substitutions across ≥ 100,000 operator applications, with every outcome accounted
  for (S6 §15.3).

## 5. Earlier sketch (moved from `docs/GENOME.md`, unchanged in intent)

```rust
struct DesignProgram {
    schema: u32,
    interface_sha256: Digest,
    fluid_recipe: FluidRecipe,
    solid_recipe: SolidRecipe,
    manufacturing: ManufacturingVariant,
}
enum FluidRecipe {
    Ct35Reference(Ct35Ratios),
    ChannelGraph { nodes: Vec<Junction>, edges: Vec<Sweep>, chamber: WallField },
    Implicit { domain: Box3, basis: FieldBasis, coefficients: Vec<f64> },
}
struct Sweep {
    stable_id: FeatureId,
    endpoints: [JunctionId; 2],
    centerline: BSpline3,
    sections: Vec<Section>, // area, shape, orientation along arc length
}
enum Section { RoundedRectangle { width: f64, height: f64, radius: f64 },
               Superellipse { a: f64, b: f64, exponent: f64 }, Circle { radius: f64 } }
```

This sketch is guidance only. Graph branch count is variable. Bound complexity by measured manufacturability,
numerical resolution and budget, not by a fixed 128-vector ceiling, and raise those limits through a versioned
campaign. B-spline, Fourier and RBF wall fields use low-frequency coefficients first and refinement operators
later. The search domain needs a minimum resolved feature scale to prevent exploitable aliasing.

The first v4 kernel must:

1. Reproduce the baseline.
2. Reproduce conventional controls.
3. Only then compile branch/rejoin and freeform cases with the same fixed interface.

A deterministic seed plus a canonical serialization binds every candidate. The resulting B-rep/surface and its
tolerances are hashed too, because genotype identity alone cannot bind a changed CAD kernel.

## 6. Migration path (summary)

Freeze the current macro language as a legacy lane. TR-05 already provides `ct35-legacy-macro-v4` as an
explicit envelope migration with a receipt. Then lower each legacy layer to v4 primitives:

| Legacy layer | v4 primitive (S4 §18 Phase 3) |
|---|---|
| ramp | sweep/warp macro |
| split | true channel graph |
| ribbon | guide-surface module |
| TPMS | resolved or homogenized porous region |
| chamber modes | Fourier field |
| cone profile | spline field |
| skimmer | internal solid module |

Preserve the legacy source hash in the migration receipt and assign a new v4 phenotype hash. Keep the v3/v2
search as the comparator in an equal-budget head-to-head before claiming the redesign helps (S5 §24).
