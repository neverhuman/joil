# JOIL v4 — Open-World Scientific Invention Engine

## Engineering specification for a genome capable of alien design, graded by hard science

**Review basis:** `neverhuman/joil` main tree `d1ac285d7cde6b5c8b0f2f3c9cd20965e90e0ec1` and `neverhuman/jarambit` main tree `c9882733f2d23a69abe82876fc65b597cb4a4e95`, reviewed 2026-10-15.

---

## 0. Executive verdict

JOIL is already the more ambitious invention system. Its current `DesignProgram`, constrained NSGA-II, MAP-Elites archive, reduced-order physics, OpenFOAM promotion, structural screens, Monte Carlo evaluation, CUDA path, and explicit scientific caveats are far beyond a conventional scalar optimizer.

However, JOIL is not yet an open-ended alien invention engine. It is a powerful optimizer inside a hand-written vocabulary of known mechanisms. The present genome can combine ramps, splitters, harmonic walls, cone modes, skimmers, ribbons, and a small TPMS catalog, but it cannot invent an entirely new geometric language without a programmer first adding the new language to the Rust enum, normalization code, compiler, classifier, descriptors, visualizer, and evaluators.

Jarambit reaches the opposite balance. Its main genome is substantially less expressive, and even its Crescent lane is a fixed design envelope. Its major advantage is the surrounding scientific operating system: explicit requirement semantics, evidence identities, deterministic uncertainty sampling, prohibited evidence transfers, release gates, claim mutation tests, and a strong distinction between execution, numerical verification, and physical validation.

The correct synthesis is therefore:

> **Copy Jarambit’s scientific operating system, not its chromosome. Replace JOIL’s closed feature catalog with a typed developmental graph and make every evaluation backend compile from one truthful design intermediate representation.**

The highest-priority issue is not a lack of exotic genes. It is semantic drift among JOIL’s genome, fast Rust phenotype, exact Python CAD, CFD surface, visual presentation, and scoring model. A design should have one scientific identity, one semantic geometry graph, and multiple explicitly qualified representations. No backend may silently ignore, approximate, downcast, repair, or replace an active feature.

---

## 1. What JOIL should become

JOIL v4 should be a domain-general **open-world scientific invention engine** with the hydrocyclone as its first domain adapter. Its core product is not a particular geometry. Its core product is a reproducible chain:

```text
RequirementSet + WorldSpec
          ↓
Typed developmental genome
          ↓
Deterministic development trace
          ↓
Single semantic DesignIR
          ↓
Independent geometry / CFD / FEA / manufacturing backends
          ↓
Applicability, convergence, uncertainty, and adversarial audits
          ↓
Content-addressed Evidence DAG
          ↓
Uncertainty-aware quality-diversity archive
          ↓
Promoted mechanisms, not merely high-scoring shapes
```

Three statements should govern the architecture:

1. **Novelty without evidence is a hypothesis.**
2. **Performance from an evaluator that shares the genome author’s assumptions is a prior, not a discovery.**
3. **Validity across eleven predeclared families is catalog coverage, not open-ended invention.**

JOIL should optimize four distinct things without collapsing them into one score:

- **Utility:** robust task performance under declared operating worlds.
- **Novelty:** structural, behavioral, mechanistic, developmental, and material distance from known designs.
- **Evidence:** strength and independence of the supporting scientific chain.
- **Evolvability:** the ability of a design lineage to generate diverse viable descendants.

---

## 2. Current architecture audit

### 2.1 What JOIL already does exceptionally well

The following foundations should be preserved:

- Exact baseline identity and a frozen interface.
- Canonical serialization and SHA-256 identities.
- Hard geometry and interface preflight before performance ranking.
- A multi-objective evolutionary engine rather than a single weighted sum.
- MAP-Elites coverage in addition to Pareto optimization.
- Monte Carlo robustness summaries.
- A fidelity ladder that distinguishes reduced-order screens, CFD, and physical validation.
- Roache GCI, turbulence-model cross-checks, and explicit open validation gates.
- A strong written doctrine that unknown oil physics must not become a fabricated score.
- A repository culture that records caveats and qualification limits rather than treating a completed run as validation.
- A future genome document that already calls for variable branch count, graph CAD, independent B-rep/SDF measurement, a minimum resolved feature scale, and a challenge corpus.

These are unusually strong foundations. The redesign should not throw them away. It should make the implementation live up to them.

### 2.2 Critical P0 defects

#### P0.1 — Silent fallback to the baseline

Several variation and sampling paths normalize a candidate and, on failure, return `baseline_ct35()`. This is unacceptable for a scientific evolutionary system.

It creates at least six forms of corruption:

1. An invalid variation is recorded as a valid baseline descendant.
2. Operator reliability is overstated.
3. Lineage is false because the returned phenotype may have no meaningful relation to the attempted edit.
4. Archive occupancy is biased toward baseline-like designs.
5. Random-walk validity tests can pass because failures are converted into valid designs.
6. Search budget accounting loses the distinction between invalid proposal, repaired proposal, and genuine evaluation.

**Required change:** every operator returns a structured `VariationOutcome`. No operator may substitute a different design on error.

```rust
pub struct VariationOutcome {
    pub child: Option<AlienGenome>,
    pub edits: Vec<GenomeEdit>,
    pub repairs: Vec<RepairRecord>,
    pub failure: Option<VariationFailure>,
    pub locality: LocalityMetrics,
    pub rng_span: RngSpan,
}
```

A failed edit consumes proposal budget, enters operator statistics, and remains visible. It does not become a baseline.

#### P0.2 — Schema identity is not preserved

The current normalizer accepts v2 or v3 and then emits a v2 family. A validator may canonicalize values, but it must never silently downcast a scientific subject.

**Required change:**

- `validate()` checks without mutation.
- `canonicalize()` preserves the declared schema.
- `migrate_v2_to_v4()` is an explicit operation that emits a migration receipt.
- No schema migration occurs inside mutation, crossover, evaluation, loading, or hashing.

#### P0.3 — The genome and exact CAD do not share one feature set

The Rust genome supports Wave-2 ribbons and TPMS fields. The Python exact-CAD path is still a v2 compiler and does not implement those active features. Meanwhile, the fast Rust phenotype often models internal structures through radial surface perturbation or aggregate volume, clearance, and porosity formulas.

This creates multiple subjects that can accidentally share one design label:

- the authored genome;
- the Rust search mesh;
- the Python B-rep;
- the CFD mesh;
- the preview geometry;
- the reduced-order mechanism descriptor.

**Required change:** introduce a single semantic `DesignIR`. Every backend declares exact support for every active IR node. Missing support is a hard `UnsupportedFeature` result, never an ignored field.

#### P0.4 — Proxy geometry is being asked to certify actual topology

A watertight enclosing fluid surface does not prove that an internal TPMS, branch network, ribbon cascade, cavity, or powder-removal path exists as claimed. Aggregate volume subtraction does not establish topology, local feature size, connectedness, accessibility, or meshability.

**Required change:** hard geometry qualification must operate on the compiled active geometry, including all internal solids and voids. `trapped_cavities` must be measured evidence, not a default zero.

#### P0.5 — Human priors leak directly into the score

Current mechanism descriptors are computed from genome fields using hand-written formulas and then used in reduced-order performance formulas and archive descriptors. The same theory that proposes a feature therefore helps certify that the feature works.

This is useful as a design prior, but it is not independent evidence.

**Required change:** split descriptors into two namespaces:

```rust
pub struct PriorDescriptors { /* calculated from genome */ }
pub struct ObservedDescriptors { /* calculated from fields/measurements */ }
```

`PriorDescriptors` may steer proposal generation. They may not support performance claims, mechanism claims, archive replacement at a qualified tier, or physical promotion.

#### P0.6 — Topology is a hard-coded label catalog

The current `topology_class` is both authored and derived through a fixed classifier. The eleven families are useful experiment strata, but they are not an open ontology.

**Required change:** remove authored `topology_class` from the scientific subject. Derive a post hoc `PhenotypeTaxon` from actual geometry and behavior. Unknown clusters must be allowed and expected.

#### P0.7 — Surrogate defaults can hide invalidity and novelty

When descriptor or phenotype extraction fails, the current surrogate feature path can substitute baseline-like/default values. This turns an unknown or invalid design into an apparently ordinary point.

**Required change:** no imputation with physical defaults. Use explicit missingness, a separate failure model, and mandatory abstention for out-of-distribution candidates.

#### P0.8 — “Expressiveness” is presently a closed-catalog validity test

The 220/220 audits are valuable regression tests: they show that samples from the known family generators survive the known compiler and gates. They do not establish open-ended expressiveness.

**Required wording:**

- Keep the tests.
- Rename them `closed_catalog_validity` and `operator_chain_validity`.
- Add a separate open-world challenge whose oracle requires a new graph/homology/behavior class without source-code changes.

---

## 3. JOIL versus Jarambit

| Dimension | JOIL today | Jarambit today | JOIL v4 decision |
|---|---|---|---|
| Genome breadth | Composable 3D mechanism fields, but fixed struct and finite catalog | Mainly fixed scalar vectors and a bounded spline/feature lane | Keep JOIL’s ambition; replace fixed struct with typed graph and developmental programs |
| Topology mutation | Named branch count, TPMS basis, ribbon and harmonic mutations | Mostly parameter mutation inside a recognizable object envelope | Introduce genuine node/edge insertion, deletion, rewiring, duplication, grafting, and rewrite rules |
| Crossover | Scalar interpolation and whole-feature swaps described as graph crossover | Uniform gene crossover | Use innovation IDs, typed graph matching, module grafting, and explicit unmatched-gene policy |
| Geometry truth | Multiple partially divergent geometry paths | More tightly object-specific but still bounded | One DesignIR, compiler certificates, and backend semantic coverage |
| Search | NSGA-II, islands, MAP-Elites, surrogate | NSGA-II/QD variants, extensive campaign machinery | Build hierarchical uncertainty-aware QD and preserve stepping stones |
| Scientific identity | Good hashes and manifests, but fragmented across code and Python | Strong subject/requirements/build/config/evaluator/scenario/dependency identity | Port and generalize Jarambit’s evidence identity into typed JOIL core |
| Requirement semantics | Draft research set; many open fields | Hard/soft/record-only/provisional distinctions are explicit | Adopt the distinction everywhere, including UI and archive status |
| Monte Carlo | Robustness summaries, but domain model remains compact | Sample-index deterministic and thread-count invariant | Adopt sample-identity discipline and correlated uncertainty models |
| Claims governance | Strong prose caveats and audits | Strong release policy, prohibited transfers, mutation tests | Make claims a machine-checked DAG, not documentation convention |
| Physical validation | Correctly left open | Explicitly blocks release when evidence is absent | Preserve fail-closed policy and add lab-loop interfaces |

### 3.1 What to copy from Jarambit

1. **Scientific identity records** binding subject, requirements, source, build, lockfile, config, evaluator, scenario, objective, descriptor, material, and dependencies.
2. **Three independent state axes:** execution succeeded, numerical result verified, physical validation status.
3. **Requirement semantics:** hard, soft, record-only, provisional, and explicitly out-of-scope.
4. **Prohibited transfers:** no reuse of evidence across changed geometry, material, process, solver scope, or specimen without an approved transfer argument.
5. **Deterministic sample identity:** sample `i` must be invariant to worker count and scheduling.
6. **Release policy:** incomplete evidence remains incomplete; a successful pipeline does not authorize a claim.
7. **Adversarial test inventory:** deletion, stale cache, subject mismatch, claim mutation, missing control, unit mismatch, nonfinite data, and evidence tampering tests.
8. **Phenotype-centered red-team gates:** constraints are measured on the built subject, not inferred only from gene boxes.
9. **Human authorization:** no automatic final promotion.

### 3.2 What not to copy from Jarambit

1. A flat, fixed-length chromosome as the universal representation.
2. Euclidean distance in genotype space as the primary novelty metric.
3. A single recognizable object family as the definition of validity.
4. Repair-heavy operators that hide the true viability of proposals.
5. Aesthetic anchoring that prevents departure from the reference family.

The concise design rule is:

> **Jarambit supplies the assurance shell. JOIL supplies the seed of the open search engine. JOIL v4 must combine them without inheriting Jarambit’s representational ceiling.**

---

## 4. Non-negotiable design principles

### 4.1 One subject, many representations

Every artifact must identify the same semantic subject or explicitly declare a transformation. A preview mesh, CFD mesh, B-rep, SDF, FEA mesh, and manufactured specimen are not interchangeable simply because they share a name.

### 4.2 Open ontology

The engine must be able to create a viable design that cannot be assigned to any seed family without changing source code. Taxonomy is an analysis output, not a genome field.

### 4.3 Independent judgment

An evaluator must consume compiled geometry, fields, materials, boundary conditions, and scenarios. It must not infer success from the names or values of genes that were designed to encode a hypothesized mechanism.

### 4.4 Unknown is a first-class result

Every scientific datum is one of:

- known pass;
- known fail;
- unknown because not evaluated;
- invalid because the evaluation was numerically or procedurally untrustworthy;
- not applicable with a reason.

Unknown must never be zero, baseline, false, or an empty array.

### 4.5 Complexity must be earned

Start genomes minimal and allow complexification. Penalize unsupported or unresolved complexity, not complexity itself. A complex module survives because it creates robust utility, a useful stepping stone, or evolvability.

### 4.6 Neutrality is useful, but identity must be dual

Inactive or latent genes can preserve evolutionary memory and create neutral networks. They should not alter the compiled subject hash.

Maintain at least:

- `genotype_hash`: all active and latent hereditary information;
- `development_trace_hash`: exact deterministic expansion;
- `phenotype_hash`: semantic compiled geometry/material/process subject;
- `artifact_hashes`: exact backend bytes.

### 4.7 Fidelity is evidence, not merely compute cost

A low-fidelity result cannot evict a high-fidelity result merely because its point estimate is better. Archive comparison must account for uncertainty and evidence tier.

### 4.8 A new mechanism requires intervention

A candidate is not credited with a novel mechanism because it looks unusual or correlates with performance. The claimed module must survive deletion, replacement, sensitivity, and cross-world tests.

---

## 5. Proposed repository architecture

```text
crates/
  joil-schema/            Canonical wire formats, units, content references
  joil-requirements/      Requirement sets, scopes, hard/soft/record-only semantics
  joil-world/             Operating scenarios, uncertainty models, coevolved challenges
  joil-genome/            Typed hereditary graph, developmental programs, lineage
  joil-grammar/           Rewrite systems, module libraries, typed generative rules
  joil-design-ir/         Single semantic geometry/material/process representation
  joil-compiler/          Deterministic lowering and compiler certificates
  joil-geometry-audit/    Independent B-rep/SDF/topology/manufacturing audits
  joil-evaluators/        Common evaluator protocol and fidelity contracts
  joil-hydrocyclone/      Domain adapter: fluids, PBM, operating metrics, interfaces
  joil-structure/         Structural screens and promoted FEA adapters
  joil-search/            QD archives, emitters, speciation, stepping-stone retention
  joil-surrogate/         Failure, feasibility, performance, OOD, acquisition models
  joil-evidence/          Evidence DAG, claim registry, receipts, transfer rules
  joil-lab/               Specimen passports, paired tests, calibration and replication
  joil-cli/               Stable user and agent interface
apps/
  explorer/               Geometry, lineage, evidence, archive, mechanism cards
backends/
  brep/
  sdf/
  cfd/
  fea/
  manufacturing/
  preview/
requirements/
worlds/
claims/
policies/
experiments/
```

The hydrocyclone-specific implementation must become a domain plugin. Core graph, compiler, evidence, archive, and uncertainty types must not contain CT-35 constants.

---

## 6. AlienGenome v4

### 6.1 Top-level data model

```rust
pub struct AlienGenome {
    pub schema: SchemaId,
    pub interface_contract: ContentRef,
    pub roots: Vec<FeatureId>,
    pub graph: StableDiGraph<GeneNode, TypedConnection>,
    pub field_programs: Vec<FieldProgram>,
    pub material_program: MaterialProgram,
    pub process_program: ProcessProgram,
    pub control_program: Option<ControlProgram>,
    pub latent_modules: Vec<LatentModule>,
    pub operator_state: SelfAdaptiveOperatorState,
    pub lineage: LineageRecord,
    pub complexity_budget: ComplexityBudget,
}
```

The genome is not itself CAD. It is a typed hereditary program that deterministically develops into a semantic `DesignIR`.

### 6.2 Gene node library

```rust
pub enum GeneNode {
    Interface(InterfaceSpec),
    Primitive(PrimitiveSpec),
    Sweep(SweepSpec),
    Loft(LoftSpec),
    Shell(ShellSpec),
    Boolean(BooleanSpec),
    Branch(BranchSpec),
    Merge(MergeSpec),
    Repeat(RepeatSpec),
    Symmetry(SymmetrySpec),
    Transform(TransformSpec),
    Constraint(ConstraintSpec),

    ImplicitField(ImplicitFieldSpec),
    RbfField(RbfFieldSpec),
    FourierField(FourierFieldSpec),
    TpmsField(TpmsFieldSpec),
    CppnField(CppnFieldSpec),
    ReactionDiffusion(ReactionDiffusionSpec),
    CellularDevelopment(CellularDevelopmentSpec),
    DensityField(DensityFieldSpec),

    MaterialField(MaterialFieldSpec),
    MicrostructureField(MicrostructureFieldSpec),
    ProcessFeature(ProcessFeatureSpec),

    Sensor(SensorSpec),
    Actuator(ActuatorSpec),
    Controller(ControllerSpec),

    ModuleCall(ModuleCallSpec),
    RewriteRule(RewriteRuleSpec),
}
```

This library should remain extensible through versioned operator and node registries. A candidate containing an unknown node is rejected explicitly; it is never partially loaded.

### 6.3 Typed ports and connections

Every node exposes typed ports. Connections are legal only when types, units, frames, dimensionality, multiplicity, and semantic role are compatible.

```rust
pub struct PortType {
    pub domain: PhysicalDomain,       // Solid, Fluid, Thermal, EM, Control, Process
    pub topology_dim: TopologyDim,    // Point, Curve, Surface, Volume, Field
    pub quantity: QuantityKind,       // Length, Area, Flow, Material, Transform, etc.
    pub unit: UnitExpr,
    pub frame: FrameId,
    pub orientation: OrientationRule,
    pub multiplicity: Multiplicity,
}

pub struct TypedConnection {
    pub innovation_id: InnovationId,
    pub from: PortRef,
    pub to: PortRef,
    pub transform: Option<ConnectionTransform>,
    pub enabled: bool,
}
```

This prevents nonsensical graph edits before geometry compilation and makes cross-domain invention possible without untyped JSON conventions.

### 6.4 Representation portfolio

No single representation is sufficient for alien invention. JOIL should support a portfolio whose members can compose in one graph.

#### Direct parametric modules

Best for interfaces, manufacturing features, and local tuning. They offer stable semantics and high feasibility.

#### Skeleton/channel graphs

Best for branches, rejoins, manifolds, vascular systems, flow networks, frames, and load paths. Nodes and edges can vary in count and connectivity.

#### Exact CSG and B-rep operations

Best for terminal manufacturable geometry and precise interface control.

#### Implicit fields and SDFs

Best for freeform blends, continuous topology changes, graded porosity, and shapes difficult to express through a fixed feature history.

#### CPPN or coordinate-network fields

Best for compact developmental regularities such as repetition, symmetry, quasi-periodicity, self-similarity, and coordinated spatial patterning. These are useful indirect encodings, not evidence of performance.

#### Rewrite grammars and L-systems

Best for repeated structural motifs, branching systems, recursive flow networks, and modular complexification.

#### Reaction-diffusion and cellular development

Best for emergent spatial organization. Compilation must be deterministic, bounded, resolution-aware, and auditable.

#### Density/topology fields

Best for local load-path or flow-path adaptation, especially when gradients or adjoints are available. They must eventually lower to a manufacturable representation.

#### Material and process programs

Geometry alone is not sufficient. Alien invention may come from spatial material allocation, anisotropy, graded microstructure, surface treatment, build orientation, assembly order, or postprocessing.

#### Control programs

For active systems, morphology and controller should coevolve, but evaluation must prevent a controller from exploiting unmodeled simulator behavior.

### 6.5 Minimal-start complexification

The default initial genome should contain only:

- the immutable interface contract;
- a minimal connected fluid/solid path;
- one material and process declaration;
- no optional internal mechanism.

Complexity enters through structural mutations. Speciation and archive protection give new structures time to tune. This is preferable to starting every candidate with every possible layer and many inactive switches.

### 6.6 Development bounds

Every developmental program declares:

- maximum expansion steps;
- maximum node and edge count;
- minimum resolved feature scale;
- maximum field bandwidth;
- maximum recursion depth;
- deterministic seed policy;
- memory and compute budget;
- termination proof or watchdog contract.

Exceeding a budget yields `DevelopmentBudgetExceeded`, not truncated geometry.

---

## 7. Deterministic development and DesignIR

### 7.1 Compiler phases

1. **Parse and schema verification.** Reject unknown or duplicate fields.
2. **Type, unit, frame, and interface verification.** No geometry is built yet.
3. **Developmental expansion.** Execute rewrite rules and field programs with exact trace capture.
4. **Semantic lowering.** Produce bodies, voids, interfaces, fields, materials, process intent, and constraints.
5. **Topological preflight.** Check graph reachability and declared invariants before expensive CAD.
6. **DesignIR canonicalization.** Stable ordering and exact content hash.
7. **Backend compilation.** B-rep, SDF, CFD, FEA, manufacturing, and preview derive from the same IR.
8. **Independent audits.** Compare backends and measure invariants.
9. **Compiler certificate emission.** Bind all inputs, outputs, tolerances, support declarations, and warnings.

### 7.2 DesignIR

```rust
pub struct DesignIR {
    pub schema: SchemaId,
    pub coordinate_system: CoordinateSystem,
    pub bodies: Vec<SolidBody>,
    pub fluid_domains: Vec<FluidDomain>,
    pub interfaces: Vec<BoundaryPatch>,
    pub material_fields: Vec<MaterialField>,
    pub process_plan: ProcessPlan,
    pub semantic_map: FeatureGeometryMap,
    pub invariants: Vec<DesignAssertion>,
    pub source_trace: DevelopmentTraceRef,
}
```

The IR must explicitly represent internal solids and fluid voids. It cannot merely adjust an enclosing radius and keep aggregate volume accounts.

### 7.3 Semantic geometry map

Every face, edge, cell region, implicit component, and material region should be traceable to its originating genome node and developmental event.

This enables:

- module ablation;
- load and boundary transfer;
- failure localization;
- operator credit assignment;
- exact evidence identity;
- comparison across CAD, CFD, FEA, and manufactured specimens.

### 7.4 Compiler certificate

```rust
pub struct CompilerCertificate {
    pub genome: ContentRef,
    pub interface_contract: ContentRef,
    pub compiler: ContentRef,
    pub design_ir: ContentRef,
    pub tolerance_policy: ContentRef,
    pub active_feature_coverage: Vec<FeatureCoverage>,
    pub artifacts: Vec<ArtifactRecord>,
    pub topology_invariants: TopologySignature,
    pub semantic_map: ContentRef,
    pub warnings: Vec<CompilerWarning>,
    pub status: CompileStatus,
}
```

`FeatureCoverage` is mandatory for every active node and every claimed backend:

```rust
pub enum CoverageStatus {
    Exact,
    ConservativeApproximation { bound: BoundRef },
    VisualizationOnly,
    Unsupported,
}
```

A backend with an `Unsupported` active node cannot support qualification for that candidate.

---

## 8. Backend truth and equivalence

### 8.1 Required backends

- Exact or tolerance-controlled B-rep.
- Adaptive SDF/voxel representation.
- Surface and volume meshes for CFD.
- Structural mesh with material orientation and contacts.
- Manufacturing geometry and process plan.
- Lightweight preview mesh.

### 8.2 Independent agreement tests

For any promoted design, at least two independent geometry paths should agree within predeclared tolerances on:

- volume and surface area;
- connected components;
- Euler characteristic and Betti numbers;
- terminal positions, areas, normals, and semantic IDs;
- minimum wall thickness and passage size;
- port-to-port connectivity;
- trapped cavities and drainage paths;
- mass and material distribution;
- bidirectional Hausdorff distance;
- local feature-size distribution;
- semantic patch areas.

Disagreement does not average out. It quarantines the candidate.

### 8.3 No visual geometry in scientific scoring

Presentation meshes may smooth, decimate, color, animate, or simplify. Their manifest must say `visualization_only`, and their hashes must never substitute for the scientific subject.

### 8.4 Resolution applicability

Before any discretized backend evaluates a design, it must prove that:

- every performance-relevant feature has sufficient cells/elements/voxels;
- the topology is stable under one level of refinement;
- tiny passages are not numerically sealed or opened;
- field bandwidth is below the discretization Nyquist limit with safety margin;
- unresolved features produce `OutOfApplicability`, not a misleading score.

---

## 9. Variation operators for real topology invention

### 9.1 Operator families

#### Parametric operators

- bounded or log-space scalar perturbation;
- correlated parameter mutation;
- local spline control mutation;
- field coefficient add/drop/refine;
- self-adaptive step size.

#### Graph operators

- insert/delete node;
- insert/delete typed edge;
- split an edge into a subgraph;
- split one branch into N children;
- merge compatible branches;
- rewire a connection;
- create or delete a loop;
- duplicate a module;
- mirror/rotate/repeat a module;
- break symmetry;
- graft a donor subgraph;
- exapt a module into a different semantic role;
- replace a subgraph with a grammar expansion;
- collapse a low-value subgraph.

#### Developmental operators

- mutate rewrite precondition;
- mutate production rule;
- mutate diffusion constants or source fields;
- add/remove a developmental stage;
- change a module’s coordinate frame or growth field;
- refine/coarsen field resolution without changing intended phenotype.

#### Material/process operators

- insert graded material region;
- rotate anisotropy field;
- change build orientation;
- alter support/removal channels;
- add assembly interface;
- mutate heat treatment or surface process subject to a validated process catalog.

#### Controller operators

- add/remove sensor;
- add/remove actuation path;
- mutate control graph;
- coarsen or refine policy complexity.

### 9.2 Innovation IDs and speciation

Every structural gene receives an innovation ID. Crossover aligns homologous structures by ancestry and type, not by vector position or field name.

Species distance should combine:

- unmatched innovation count;
- graph edit distance;
- representation class distance;
- phenotype topology distance;
- observed behavior distance;
- developmental trace distance.

New structural species receive a protected tuning window before competing directly with mature lineages.

### 9.3 Repair policy

Repairs are permitted only when they are explicit mathematical projections with bounded semantic effect.

Each repair records:

- violated invariant;
- pre-repair and post-repair hashes;
- projection method;
- distance moved;
- affected nodes;
- whether lineage credit is reduced;
- whether the result remains eligible for science promotion.

Silent clamping, branch substitution, downcasting, and baseline fallback are prohibited.

### 9.4 Operator quality metrics

Track per operator and per species:

- proposal count;
- parse/type validity;
- compile validity;
- hard-gate validity;
- unique phenotype yield;
- novelty gain;
- utility gain;
- archive contribution;
- evidence progression;
- locality distribution;
- repair magnitude;
- compute consumed;
- descendant evolvability.

Operator scheduling should use a contextual bandit over these outcomes rather than fixed mutation probabilities.

---

## 10. Search architecture

### 10.1 Hierarchical quality-diversity archive

A single flat MAP-Elites key is too restrictive for open-world invention. Use a hierarchy:

```text
Level 0: representation/developmental family
Level 1: measured topology and homology
Level 2: learned behavioral embedding
Level 3: operating world / duty envelope
Level 4: evidence tier and uncertainty class
```

Cells store a small nondominated set, not one scalar elite. Each entry carries a probability distribution, evidence state, and fidelity.

### 10.2 Archive partitions

Maintain distinct partitions:

- `invalid`: parse, type, compile, or numerical invalidity;
- `unknown`: not yet evaluated or outside evaluator applicability;
- `near_feasible`: useful boundary and stepping-stone candidates;
- `feasible_low_fidelity`;
- `verified_simulation`;
- `physically_validated`;
- `independently_replicated`.

A low-fidelity candidate cannot displace a higher-tier candidate solely through a lower point estimate. It can coexist as a hypothesis with high value of information.

### 10.3 Descriptor strategy

Use three descriptor classes:

1. **Structural descriptors:** measured from compiled phenotype, such as graph cycle rank, connected-component signature, Betti numbers, branching entropy, genus, porosity spectrum, and material phase count.
2. **Behavioral descriptors:** measured from simulations or experiments, such as flow topology, vorticity spectrum, residence-time distribution, shear distribution, pressure recovery, robustness, and failure mode.
3. **Learned descriptors:** unsupervised embeddings trained on field observations or trajectories, with versioned encoder identity and stability audits.

Gene-derived descriptors are proposal priors only.

### 10.4 Emitter portfolio

Run multiple proposal processes concurrently:

- random grammar emitter;
- structural graph emitter;
- novelty emitter;
- constraint-boundary emitter;
- continuous CMA-style emitter;
- differentiable/adjoint emitter where valid;
- uncertainty/value-of-information emitter;
- exaptation/module-transfer emitter;
- world-transfer emitter;
- archive-repair emitter;
- language-model AST proposer constrained by the typed schema.

The scheduler rewards verified archive coverage, hypervolume, new viable topology, mechanism evidence, and evidence progression—not just cheap proxy improvement.

### 10.5 Stepping-stone preservation

Candidates may be retained because they:

- solve a different world;
- unlock a new region of topology space;
- create high descendant diversity;
- supply a reusable module;
- cross a feasibility boundary;
- expose a model failure;
- falsify a human prior.

This prevents premature convergence on the first successful hydrocyclone idiom.

### 10.6 Coevolving worlds

Define `WorldSpec` as a versioned scientific subject containing duty, boundary conditions, uncertain variables, failure modes, and evaluator applicability.

World mutations can vary:

- flow and pressure environment;
- inlet distributions;
- fluid properties;
- contaminant size distributions;
- transient disturbances;
- fouling and erosion conditions;
- manufacturing tolerance distributions;
- structural and thermal loads;
- sensor degradation;
- maintenance constraints.

Admit a new world only when it is solvable by at least one current design and meaningfully differentiates designs. Transfer designs and modules across worlds. This creates an open-ended arms race without rewarding impossible challenges.

---

## 11. Scientific grading of alien invention

### 11.1 Never use a single “alien score”

Report a certificate with independent axes.

#### Novelty level N0–N5

- **N0 — Known:** baseline, direct copy, or trivial parameter adjustment.
- **N1 — Recombinant:** new combination of known modules, no new structural or behavioral class.
- **N2 — Structural:** new graph, homology, developmental motif, material layout, or process architecture absent from the seed and reference corpus.
- **N3 — Behavioral:** occupies a new stable measured behavior region while meeting minimum criteria.
- **N4 — Mechanistic:** a new causal mechanism survives ablation, sensitivity, cross-solver, and held-out world tests.
- **N5 — Replicated invention:** the mechanism transfers beyond the discovery condition and is independently physically replicated.

#### Evidence level E0–E6

- **E0:** parses; no geometry claim.
- **E1:** deterministic compiled subject with independent geometry agreement.
- **E2:** verified analytic/reduced-order evaluation inside declared applicability.
- **E3:** converged high-fidelity simulation with conservation and uncertainty closure.
- **E4:** cross-model or cross-solver confirmation and robust scenario evidence.
- **E5:** paired physical validation on identified specimens.
- **E6:** independent replication or external laboratory confirmation.

#### Utility level U0–U4

- **U0:** infeasible or below minimal criterion.
- **U1:** feasible stepping stone.
- **U2:** nondominated in at least one declared world.
- **U3:** robustly dominates the baseline or accepted comparator on a preregistered vector.
- **U4:** validated transfer across multiple worlds or scales.

A design can honestly be reported as `N4 / E3 / U2`. This is far more informative than calling it “alien” or “best.”

### 11.2 Novelty vector

Store novelty as a vector before any display aggregation:

```rust
pub struct NoveltyVector {
    pub representation: f64,
    pub topology: f64,
    pub geometry: f64,
    pub behavior: f64,
    pub mechanism: f64,
    pub material_process: f64,
    pub developmental: f64,
    pub reference_corpus_distance: f64,
    pub transfer_breadth: f64,
}
```

Distance metrics must be versioned and tested against rigid transforms, unit changes, mesh density, representation synonyms, and superficial decoration.

### 11.3 Mechanism cards

Every N4 candidate must have a machine-readable `MechanismCard`:

```rust
pub struct MechanismCard {
    pub subject: ContentRef,
    pub claim: String,
    pub causal_chain: Vec<CausalStep>,
    pub required_observables: Vec<ObservableRef>,
    pub module_interventions: Vec<InterventionResult>,
    pub interaction_tests: Vec<InteractionResult>,
    pub sensitivity: Vec<SensitivityResult>,
    pub falsification_tests: Vec<TestRef>,
    pub applicability: ApplicabilityEnvelope,
    pub alternative_explanations: Vec<String>,
    pub unresolved_questions: Vec<String>,
    pub evidence_root: ContentRef,
}
```

Minimum intervention suite:

- delete the claimed module;
- replace it with a volume/mass-matched neutral control;
- perturb its key parameters locally;
- break its symmetry or phase relation;
- test interactions with neighboring modules;
- replay across held-out worlds;
- rerun with an independent evaluator.

If the performance survives removal of the supposed mechanism, the mechanism claim is rejected even if the design remains useful.

---

## 12. Utility, constraints, and robustness

### 12.1 Hydrocyclone utility vector

Do not compress the domain into a single score. Suggested outputs include:

- treated flow at compliant quality;
- treated outlet concentration with uncertainty;
- grade-efficiency curve;
- pressure drop and specific energy;
- reject fraction and stability;
- short-circuit and reverse-flow measures;
- residence-time distribution;
- droplet breakup and coalescence closure;
- fouling sensitivity;
- erosion/wear proxy and later measurement;
- structural and modal margins;
- mass, cost, process time, and yield;
- maintainability and blockage recovery;
- manufacturing robustness;
- world-transfer performance.

Hard constraints and soft objectives remain separate. A weighted presentation score may be generated for a specific decision, but it never replaces the underlying vector.

### 12.2 Robust decision statistics

Use predeclared statistics according to the risk:

- lower confidence bound for benefits;
- upper confidence bound for harms;
- probability of satisfying each hard requirement;
- joint probability of satisfying all hard requirements;
- CVaR for severe-tail failure;
- p05/p50/p95 summaries;
- worst case over finite certification scenarios;
- distributionally robust bounds when the scenario distribution is uncertain.

Report marginal and joint outcomes. Independent marginal pass rates do not imply joint reliability.

### 12.3 Uncertainty decomposition

Track separately:

- manufacturing geometry;
- material properties and anisotropy;
- operating conditions;
- feed distributions;
- numerical discretization;
- iterative/convergence error;
- model-form uncertainty;
- surrogate epistemic uncertainty;
- experimental measurement uncertainty;
- specimen-to-specimen variation.

Do not merge these into one unlabeled standard deviation.

---

## 13. Fidelity ladder and promotion

### F0 — Semantic and geometric validity

Requirements:

- schema, type, unit, and interface validity;
- deterministic development;
- complete active-feature support;
- independent geometry agreement;
- topology, clearance, connectivity, drainage, and process preflight.

Allowed claim: “This is a well-defined manufacturable or explicitly research-only candidate geometry.”

### F1 — Analytic and reduced-order screen

Requirements:

- evaluator applicability demonstrated;
- conservation checks;
- benchmark or analytic oracle where available;
- uncertainty bounds;
- no gene-derived mechanism values treated as observations.

Allowed claim: “Promising under the declared screen.”

### F2 — Coarse high-dimensional solver

Requirements:

- mesh and time-step adequacy screen;
- field conservation;
- resolved critical features;
- numerical failure separation.

Allowed claim: “High-fidelity candidate, not yet converged.”

### F3 — Converged multiphysics simulation

Requirements:

- grid/time/parcel or population convergence;
- model-form checks;
- full-domain closure;
- robustness scenarios;
- stable semantic mapping from geometry to mesh.

Allowed claim: “Numerically verified within the modeled scope.”

### F4 — Independent model or solver confirmation

Requirements:

- alternative turbulence, multiphase, structural, or numerical formulation;
- blinded or held-out configuration;
- predeclared agreement criteria.

Allowed claim: “Cross-model confirmed within the modeled scope.”

### F5 — Paired physical validation

Requirements:

- specimen passports;
- measured material/process state;
- calibrated instrumentation;
- paired or blocked design;
- preregistered analysis;
- uncertainty and missing-data handling.

Allowed claim: “Physically validated in the tested apparatus and duty.”

### F6 — Independent replication

Requirements:

- independent operator or laboratory;
- independently prepared specimen or apparatus;
- subject identity and transfer justification;
- successful replication against preregistered criteria.

Allowed claim: “Independently replicated.”

---

## 14. Surrogate redesign

### 14.1 Separate models

Use distinct models for:

1. compile/numerical failure probability;
2. hard-feasibility probability;
3. each performance output;
4. model discrepancy between fidelity levels;
5. evaluator applicability;
6. representation/behavior OOD probability.

A failed evaluation is not a bad physical score and must not enter the regression target as one.

### 14.2 Inputs

Use combinations of:

- typed graph embeddings;
- topology signatures;
- geometry statistics;
- sampled field embeddings;
- low-fidelity observed response;
- material/process state;
- world descriptors.

Do not silently fill missing physical observations with baseline values.

### 14.3 Multi-fidelity modeling

Model deltas between fidelities when possible:

```text
F3 response = F1 response + learned discrepancy(F1 observations, geometry, world)
```

The surrogate never changes the evidence tier of its prediction. A predicted F3 result is still a surrogate prediction.

### 14.4 Calibration and abstention

Require:

- ensembles or posterior uncertainty;
- conformal or equivalent calibration checks;
- leave-one-representation-out validation;
- leave-one-topology-out validation;
- leave-one-mechanism-out validation;
- leave-one-world-out validation;
- temporal holdout;
- explicit OOD threshold and abstention.

A true alien candidate should often trigger abstention. That is a feature, not a failure.

### 14.5 Acquisition

Use expected evidence-aware hypervolume improvement, incorporating:

- feasibility probability;
- numerical success probability;
- performance uncertainty;
- novelty contribution;
- archive coverage;
- cost;
- expected evidence-tier advancement;
- OOD exploration value.

---

## 15. Hard geometry, manufacturing, and topology audits

Every promoted candidate should pass the following or receive an explicit not-applicable status.

### Geometry

- finite, oriented, non-self-intersecting subject;
- valid solids and voids;
- connected-component inventory;
- exact terminal interfaces;
- positive volumes;
- local thickness and passage-size fields;
- curvature and feature-size distributions;
- no accidental sliver faces or nonsewable regions;
- no branch disconnection or false rejoin;
- no unintended interpenetration;
- no unresolved aperture.

### Topology and reachability

- Euler characteristic and Betti numbers;
- graph cycle rank;
- inlet-to-outlet reachability;
- all internal fluid regions connected to an allowed port or explicitly declared sealed function;
- trapped fluid, gas, support, or powder cavities;
- drainage and cleaning paths;
- topology stability under tolerance perturbation.

### Manufacturing

- declared process and machine envelope;
- minimum wall, ligament, hole, and channel sizes;
- build orientation;
- overhang and unsupported span;
- support insertion and removal;
- powder evacuation;
- tool accessibility;
- assembly access;
- inspection access;
- postprocess stock and datum plan;
- distortion and shrink compensation status;
- process-specific material passport.

### Metamorphic tests

The audit must be invariant to:

- rigid translation and rotation;
- consistent unit conversion;
- stable tessellation refinement;
- vertex ordering;
- semantically equivalent graph ordering;
- harmless inactive hereditary information.

---

## 16. Physics firewall

### 16.1 Evaluator input contract

Scientific evaluators receive only:

- compiled subject references;
- semantic boundary patches;
- material/process state;
- world/scenario;
- solver configuration;
- declared approximations.

They do not receive gene names such as `gyroid_bonus`, `shear_mitigation`, or `core_shield_ratio` as facts.

### 16.2 Independent behavior descriptors

Compute descriptors from outputs such as:

- angular momentum flux;
- vorticity and helicity spectra;
- dissipation distribution;
- recirculation topology;
- Lagrangian coherent structures;
- residence-time distribution;
- pressure recovery;
- wall shear and erosion exposure;
- droplet collision/coalescence/breakup statistics;
- structural mode shapes and participation factors.

### 16.3 Structural promotion

The current Lamé, concentration-factor, beam, and VIV models are valuable screens. They should remain F1 evaluators. Alien topologies must eventually receive general FEA appropriate to:

- complex load paths;
- anisotropic AM material;
- residual stress;
- contacts and assemblies;
- nonlinear geometry;
- local buckling;
- fatigue and crack sensitivity;
- coupled fluid excitation;
- process-induced defects.

### 16.4 Simulator exploitation audit

Search will find evaluator bugs. Treat this as expected.

For every exceptional candidate, run:

- mesh rotation and perturbation;
- solver-setting perturbation;
- independent mesh generator;
- conservation residual audit;
- geometry simplification and restoration;
- alternate model;
- input-field randomization control;
- duplicate phenotype under a different genome encoding.

If the advantage vanishes under harmless representational changes, quarantine it as evaluator exploitation.

---

## 17. Evidence and claims system

### 17.1 Scientific identity

```rust
pub struct ScientificIdentity {
    pub schema: ContentRef,
    pub subject: SubjectRef,
    pub requirements: Vec<ContentRef>,
    pub source: ContentRef,
    pub build: ContentRef,
    pub lockfile: ContentRef,
    pub compiler: ContentRef,
    pub config: ContentRef,
    pub evaluator: ContentRef,
    pub scenario: ContentRef,
    pub objective: ContentRef,
    pub descriptor: ContentRef,
    pub material: MaterialIdentity,
    pub process: ProcessIdentity,
    pub dependencies: Vec<ContentRef>,
}
```

### 17.2 Typed evidence

```rust
pub struct Evidence<T> {
    pub identity: ScientificIdentity,
    pub value: Option<T>,
    pub unit: UnitExpr,
    pub execution: ExecutionState,
    pub numerical: NumericalState,
    pub validation: ValidationState,
    pub applicability: ApplicabilityEnvelope,
    pub uncertainty: Vec<UncertaintyComponent>,
    pub dependencies: Vec<ContentRef>,
    pub receipt: ContentRef,
}
```

A schema-valid record is not automatically qualifying. Eligibility is a separate checked operation.

### 17.3 Claim DAG

Every public or UI claim should resolve to observations and transformations:

```text
Claim
 ├── Requirement scope
 ├── Subject identity
 ├── Observations
 ├── Reduction code and config
 ├── Uncertainty treatment
 ├── Applicability checks
 ├── Review/authorization
 └── Status: supported / contradicted / incomplete / revoked
```

### 17.4 Prohibited transfers

Block automatic transfer across:

- different phenotype hash;
- different material or heat treatment;
- different build orientation or process;
- different mesh that lacks an approved convergence relation;
- different boundary condition or world;
- different solver physics scope;
- simulation to physical validation;
- one specimen to another without a population model;
- visual finish metadata to structural properties;
- ratio-scaled safety factor without reanalysis.

### 17.5 Claim mutation tests

For every important claim, generate adversarial mutations:

- wrong candidate hash;
- stale requirement revision;
- missing observation;
- changed evaluator;
- nonfinite value;
- unit mismatch;
- deleted uncertainty;
- changed material;
- missing physical evidence;
- cached result from a different world.

The claims system passes only if every mutant is rejected or downgraded.

---

## 18. Core candidate and archive records

```rust
pub struct CandidateRecord {
    pub candidate_id: CandidateId,
    pub genome: ContentRef,
    pub development_trace: ContentRef,
    pub design_ir: Option<ContentRef>,
    pub compiler_certificate: Option<ContentRef>,
    pub phenotype: Option<ContentRef>,
    pub parents: Vec<CandidateId>,
    pub variation: VariationReceipt,
    pub world: ContentRef,
    pub evaluations: Vec<ContentRef>,
    pub evidence_root: Option<ContentRef>,
    pub novelty: Option<NoveltyVector>,
    pub status: CandidateStatus,
}

pub enum CandidateStatus {
    Proposed,
    DevelopmentInvalid,
    CompileInvalid,
    GeometryInvalid,
    OutOfApplicability,
    NumericalInvalid,
    Evaluated,
    Promoted { tier: EvidenceTier },
    Revoked { reason: String },
}

pub struct ArchiveEntry {
    pub candidate: CandidateId,
    pub phenotype_taxon: TaxonId,
    pub behavior_embedding: EmbeddingRef,
    pub world: ContentRef,
    pub utility: UtilityDistribution,
    pub novelty: NoveltyVector,
    pub evidence_tier: EvidenceTier,
    pub uncertainty: UncertaintySummary,
    pub dominance_certificate: ContentRef,
}
```

---

## 19. CLI and service contract

Suggested stable commands:

```text
joil genome validate <genome>
joil genome migrate <genome> --to v4
joil develop <genome> --emit-trace
joil compile <design-ir> --backend brep|sdf|cfd|fea|manufacturing|preview
joil audit geometry <compiler-certificate>
joil audit claims <claim-set>
joil evaluate <subject> --world <world> --fidelity F1|F2|F3|F4
joil promote <candidate> --to E3
joil evolve --experiment <preregistration>
joil replay --receipt <run-receipt>
joil compare <candidate-a> <candidate-b> --world-set <set>
joil mechanism ablate <candidate> --module <feature-id>
joil world evolve --population <archive>
joil evidence trace <claim-id>
```

The same functions should be exposed through a typed service protocol for agents. Agents may propose genomes or experiments, but only the compiler/evidence system determines eligibility.

---

## 20. File-level remediation plan

### `crates/joil-core/src/program.rs`

- Freeze the existing type as `LegacyDesignProgramV3`.
- Remove silent baseline fallback from every operator.
- Stop normalizer downcasting.
- Remove authored `topology_class` from new genomes.
- Introduce typed node graph, innovation IDs, lineage, dual identities, and explicit migration.
- Return `VariationOutcome` from all operators.
- Record repair magnitude and failure.

### `crates/joil-core/src/phenotype.rs`

- Stop treating aggregate volume changes as proof of internal topology.
- Represent actual internal solids/voids or mark the lane `screen_only`.
- Replace scalar defaults such as trapped cavities with typed evidence.
- Separate enclosing-domain mesh from complete internal-geometry mesh.
- Emit semantic feature-to-geometry mapping.

### `cad/program/exotic_graph.py`

- Deprecate independent schema normalization.
- Consume DesignIR rather than a parallel genome interpretation.
- Add feature coverage receipts.
- Until replaced, reject every Wave-2 feature explicitly.

### `crates/joil-core/src/evolve.rs`

- Remove family string from the MAP-Elites key.
- Replace one elite per cell with evidence-aware Pareto sets.
- Store parent IDs, operator receipts, world IDs, compiler/evaluator hashes, and evidence roots.
- Separate invalid proposal, numerical failure, and physical infeasibility.
- Prevent low-fidelity eviction of qualified entries.

### `crates/joil-core/src/surrogate.rs`

- Remove baseline/default feature fallback.
- Add separate failure, feasibility, and performance models.
- Add OOD detection and abstention.
- Add leave-one-representation/mechanism/world-out tests.
- Bind feature extractor and training data hashes.

### `crates/joil-core/src/redteam.rs`

- Consume measured evidence from complete geometry.
- Distinguish invalid, unknown, unsupported, and fail.
- Add branch reachability, trapped-domain, process-removal, and tolerance-topology gates.

### `crates/joil-core/src/structure.rs`

- Rename results as screening qualification where appropriate.
- Add evaluator applicability and uncertainty.
- Add promoted general FEA adapters and cross-checks.

### `analysis/evidence.py`

- Port the concepts into `joil-evidence` Rust types.
- Keep Python as a client, not the authority.
- Add claim DAG and prohibited-transfer rules.

### `README.md` and figures

- Label 11-family coverage as closed-catalog validity.
- State exactly which geometry backend generated each figure and result.
- Display separate genotype, DesignIR, phenotype, mesh, evaluator, and evidence hashes.

---

## 21. Acceptance test catalog

### Genome and development

| ID | Test | Required oracle |
|---|---|---|
| G-001 | Invalid mutation | Returns explicit failure; never baseline or another candidate |
| G-002 | Schema preservation | Validation/canonicalization never changes schema version |
| G-003 | Explicit migration | Migration emits old/new hashes and a deterministic receipt |
| G-004 | Unknown node | Fails closed with node ID and registry version |
| G-005 | Typed edge mismatch | Rejected before geometry construction |
| G-006 | Development termination | All admitted programs terminate within declared budget |
| G-007 | Latent gene identity | Changes genotype hash but not phenotype hash |
| G-008 | Graph order equivalence | Reordered equivalent graph produces identical DesignIR hash |
| G-009 | Structural complexification | New node/edge count can arise without source changes |
| G-010 | New-family challenge | Produces a viable topology outside seed taxa without editing classifier code |

### Compiler and geometry

| ID | Test | Required oracle |
|---|---|---|
| C-001 | Active-feature coverage | 100% active nodes exact/bounded in any qualifying backend |
| C-002 | Unsupported feature | Backend refuses; does not drop or approximate silently |
| C-003 | Branch/rejoin corpus | Correct connectivity and semantic face mapping |
| C-004 | TPMS corpus | Actual internal topology agrees between B-rep and SDF |
| C-005 | Trapped cavity | Known cavity detected; result can never default to zero |
| C-006 | Tiny aperture | Resolves with convergence or returns out-of-applicability |
| C-007 | Interface freeze | Position, area, normal, and semantic ID match exactly |
| C-008 | Rigid transform | Physical results invariant after transformed-frame replay |
| C-009 | Unit conversion | mm/m representations agree after explicit conversion |
| C-010 | Backend agreement | Volume/topology/clearance within preregistered tolerance |
| C-011 | Tolerance topology | No unreported component or connectivity change under tolerance cloud |
| C-012 | Semantic lineage | Every qualifying boundary face maps to an originating feature |

### Search and lineage

| ID | Test | Required oracle |
|---|---|---|
| S-001 | Parentage integrity | Child receipt identifies exact parents and edit sequence |
| S-002 | Operator locality | Measured distribution matches declared local/global class |
| S-003 | Operator accounting | Failed and repaired proposals remain in budget ledger |
| S-004 | Innovation alignment | Homologous graph crossover is stable under graph reordering |
| S-005 | Species protection | Novel structure receives declared tuning window |
| S-006 | Archive fidelity | Low-fidelity point estimate cannot erase qualified evidence |
| S-007 | Deterministic replay | Same seed/config/build yields bit-identical deterministic records |
| S-008 | Thread invariance | Sample multiset and reductions remain invariant to worker count |
| S-009 | Stepping-stone retention | Near-feasible/high-evolvability entries can survive by policy |
| S-010 | World transfer | Modules and candidates can move across compatible worlds with receipts |

### Physics and mechanism

| ID | Test | Required oracle |
|---|---|---|
| P-001 | Gene-blind evaluator | Renaming/re-encoding genes with same phenotype leaves result unchanged |
| P-002 | Phenotype synonym | Two genomes with identical phenotype hash receive identical deterministic evaluation |
| P-003 | Mechanism deletion | Claimed benefit changes according to preregistered causal expectation |
| P-004 | Matched neutral control | Cosmetic volume/mass effect separated from claimed mechanism |
| P-005 | Grid/time/parcel convergence | Meets declared GCI or equivalent bounds |
| P-006 | Conservation closure | Mass, momentum, phase, and population balances within limits |
| P-007 | Model-form cross-check | Agreement or explicit unresolved discrepancy |
| P-008 | Simulator exploit mutation | Harmless mesh/solver changes do not erase claimed effect |
| P-009 | Applicability holdout | New topology does not inherit unsupported model validity |
| P-010 | Structural promotion | Alien load path receives general FEA before high-tier structural claim |

### Surrogate

| ID | Test | Required oracle |
|---|---|---|
| M-001 | Failure quarantine | Numerical/infrastructure failures never enter performance regression |
| M-002 | No default physics | Missing observation remains missing; no baseline substitution |
| M-003 | Representation holdout | Calibrated uncertainty on unseen representation family |
| M-004 | Mechanism holdout | Abstains or remains calibrated on unseen mechanism |
| M-005 | World holdout | Reports transfer uncertainty and OOD status |
| M-006 | OOD alien | High novelty candidate triggers abstention when unsupported |
| M-007 | Training identity | Dataset, extractor, model, and dependency hashes bound to prediction |
| M-008 | Acquisition replay | Candidate ranking reproducible from receipt |

### Evidence and claims

| ID | Test | Required oracle |
|---|---|---|
| E-001 | Subject mismatch | Observation for different phenotype is rejected |
| E-002 | Requirement drift | Stale requirement revision cannot support current claim |
| E-003 | Evaluator drift | Changed evaluator invalidates cached eligibility |
| E-004 | Material/process change | Physical/structural evidence does not transfer automatically |
| E-005 | Missing physical evidence | Cannot produce validated label |
| E-006 | Nonfinite and unit errors | Rejected before reduction |
| E-007 | Claim mutant | Every targeted mutant is killed or claim downgraded |
| E-008 | Receipt tamper | Hash mismatch makes evidence ineligible |
| E-009 | Unknown preservation | Unknown never becomes pass/fail/zero through serialization |
| E-010 | Revocation | Revoked evidence propagates to dependent claims |

### Open-ended invention

| ID | Test | Required oracle |
|---|---|---|
| O-001 | No fixed family ceiling | Number of discovered taxa may exceed seed taxonomy |
| O-002 | Learned behavior descriptor | Stable under mesh density and rigid transforms |
| O-003 | Coevolved world admission | New world is solvable and discriminative |
| O-004 | Exaptation | Module can acquire a new role with lineage preserved |
| O-005 | Complexity payoff | Complexification retained only with utility, novelty, or evolvability value |
| O-006 | Reference-corpus holdout | Novelty not caused by missing or duplicated reference metadata |
| O-007 | Independent rediscovery | Mechanism can be rediscovered across seeds or lineages |
| O-008 | Transfer | Mechanism survives at least one held-out duty or scale before N5 |

---

## 22. Proposed quantitative exit gates

These are engineering targets to preregister, not claims about current status.

### M0 truth gates

- Zero silent baseline fallbacks.
- Zero silent schema downcasts.
- Zero active features ignored by a qualifying backend.
- Zero scientific fields defaulted from unknown to physical values.
- 100% variation attempts represented in the budget ledger.

### M1 compiler gates

- 100% active-feature semantic coverage for promotion.
- Independent topology invariants agree exactly for the challenge corpus.
- B-rep/SDF volume and interface metrics meet preregistered tolerances.
- Every scientific mesh binds to a semantic map and DesignIR hash.

### M2 open-genome gates

- At least one valid discovered phenotype outside all seed graph taxa without source edits.
- Structural mutation and graph crossover produce nontrivial unique phenotypes with measured locality distributions.
- No more than a preregistered fraction of structural proposals require repair; all repairs are visible.
- Innovation/speciation replay is deterministic.

### M3 search gates

- Archive stores evidence-aware Pareto sets and preserves qualified incumbents.
- Learned descriptors pass invariance and stability tests.
- New-topology coverage exceeds the fixed-catalog comparator at equal evaluation budget.
- Search reports verified novelty, not only authored-family occupancy.

### M4 evidence gates

- Every published scalar is observation-linked.
- Claim mutation suite passes.
- Prohibited transfer suite passes.
- Missing physical evidence cannot produce physical language.
- Revocation propagation is complete.

### M5 invention gates

- At least one N4 mechanism with successful module ablation, matched control, cross-model evidence, and held-out world transfer.
- Rediscovery or independent lineage convergence is demonstrated.
- Surrogate abstains appropriately on the discovery before sufficient evidence exists.

---

## 23. Migration program

### Milestone M0 — Scientific truth repair

Deliverables:

- remove all baseline fallback;
- split validate/canonicalize/migrate;
- preserve schema identity;
- hard-error unsupported backend features;
- replace physical defaults with typed unknowns;
- rename gene-derived descriptors as priors;
- relabel current expressiveness tests;
- add exact lineage and budget receipts.

Exit condition: current v3 campaign can replay with no hidden repair or subject ambiguity.

### Milestone M1 — Unified DesignIR and compiler certificate

Deliverables:

- semantic DesignIR;
- Rust compiler pipeline;
- B-rep and adaptive SDF backends;
- semantic face/cell mapping;
- feature coverage matrix;
- independent geometry challenge corpus;
- exact interface and topology certificates.

Exit condition: one v3 candidate and one Wave-2 candidate compile through the same IR to all claimed backends with explicit equivalence evidence.

### Milestone M2 — Typed open genome

Deliverables:

- graph, field, material, process, and optional control nodes;
- innovation IDs and developmental trace;
- minimal-start complexification;
- graph insert/delete/rewire/duplicate/graft operators;
- typed homologous crossover;
- dual genotype/phenotype identity;
- species and lineage explorer.

Exit condition: `G-010 new-family challenge` passes without source edits.

### Milestone M3 — Open-world QD

Deliverables:

- hierarchical archive;
- phenotype/topology descriptors;
- learned behavioral embeddings;
- emitter portfolio and contextual scheduler;
- evidence-aware replacement;
- stepping-stone and evolvability retention;
- equal-budget comparison against v3.

Exit condition: v4 increases verified coverage or verified Pareto hypervolume without increasing evaluator exploitation.

### Milestone M4 — Evidence operating system

Deliverables:

- typed scientific identity;
- evidence DAG;
- claim registry;
- prohibited transfer engine;
- release policy;
- claim mutation tests;
- UI evidence trace.

Exit condition: every public claim is machine-resolvable to immutable supporting observations or automatically marked incomplete.

### Milestone M5 — Coevolving worlds and mechanism science

Deliverables:

- mutable WorldSpec;
- minimum-criterion world admission;
- world/design transfer;
- module library and exaptation;
- mechanism cards and intervention engine;
- multi-fidelity value-of-information scheduler.

Exit condition: at least one N4 mechanism is discovered and survives held-out-world intervention tests.

### Milestone M6 — Closed-loop physical research

Deliverables:

- manufacturing and specimen passports;
- metrology import;
- paired physical experiment design;
- calibration and discrepancy models;
- active selection of the next physical test;
- independent replication package.

Exit condition: at least one E5 design and one independently replicated E6 observation.

---

## 24. Required head-to-head experiment

Preserve current JOIL v3 as a comparator. Do not replace it before measuring the redesign.

Run four preregistered equal-budget lanes:

- **B0:** uniform/random search over the current fixed genome;
- **B1:** current v3 evolutionary operators and eleven-family archive;
- **B2:** typed graph genome with direct structural operators, no developmental fields;
- **B3:** full hybrid graph + fields + developmental modules + emitter portfolio.

Use identical worlds, hard gates, compute ledger, and promotion budget.

Primary measurements:

- valid proposal yield;
- unique phenotype yield;
- topology/homology coverage;
- behavior coverage;
- robust Pareto hypervolume;
- evidence-tier-adjusted hypervolume;
- number of genuinely new taxa;
- number of N3 and N4 mechanisms;
- evaluator exploitation rate;
- surrogate abstention calibration;
- reproducibility under reevaluation;
- world-transfer performance;
- descendant evolvability;
- compute per verified discovery.

The new system succeeds only if it produces more **verified** invention, not merely more bizarre geometry.

---

## 25. The ten highest-value actions

1. **Delete every silent fallback to `baseline_ct35()`.**
2. **Create one semantic DesignIR and make every backend compile from it.**
3. **Hard-error any active feature not represented by a backend.**
4. **Replace the eleven-family authored ontology with post hoc phenotype taxonomy.**
5. **Separate genome priors from observed physical descriptors.**
6. **Introduce a typed graph/developmental genome with innovation IDs and explicit complexification.**
7. **Use independent B-rep and SDF/topology audits on the complete internal geometry.**
8. **Port Jarambit’s scientific identity, requirement semantics, prohibited transfers, and release policy.**
9. **Redesign the archive around uncertainty, evidence tier, stepping stones, and learned behaviors.**
10. **Require mechanism interventions before crediting an alien mechanism.**

---

## 26. Final architecture decision

The most important conceptual change is this:

### Current mental model

```text
Choose one of several clever human mechanisms
→ tune its parameters
→ compute descriptors from those parameters
→ reward the design when those descriptors imply success
```

### Required mental model

```text
Evolve a typed generative program with no fixed family ceiling
→ compile one exact semantic subject
→ independently observe what the subject does
→ quantify uncertainty and evaluator applicability
→ preserve useful stepping stones
→ intervene on the proposed mechanism
→ promote only claims supported by the evidence DAG
```

JOIL already has unusually strong scientific instincts and a serious search stack. The transformation is not a restart. It is a change in the center of gravity—from a catalog of exotic hydrocyclone features to a general engine for inventing new physical programs, while making it impossible for novelty, backend drift, surrogate confidence, or visual spectacle to outrun the evidence.
