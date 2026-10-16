# JOIL XenoGenome v4
## Engineering Specification for Open-Ended, Science-Gated Alien Invention

**Repository audit basis**

- JOIL: `neverhuman/joil` at commit `d1ac285d7cde6b5c8b0f2f3c9cd20965e90e0ec1`
- JARAMBIT: `neverhuman/jarambit` at commit `c9882733f2d23a69abe82876fc65b597cb4a4e95`
- Audit date: 2026-10-15

---

# 1. Executive verdict

JOIL is already a serious scientific-search prototype. It has a frozen CT-35 interface, deterministic candidate hashes, strict input schemas, multi-island NSGA-II, MAP-Elites, Monte Carlo screening, OpenFOAM case generation, a structural screen, multi-seed experiments, explicit open validation gates, and a strong written science contract. JARAMBIT adds an unusually mature evidence architecture: exact content-bound subject identity, typed missing observations, scoped verdicts, append-only lineage, review authority, explicit uncertainty, and feasibility-first selection.

However, JOIL does not yet possess an open-ended design genome. It currently possesses an **alien-themed catalogue of authored mechanisms**. Search can vary inlet ramp, branch count, chamber harmonics, cone modes, skimmer, ribbon, and named TPMS basis. It cannot create an unanticipated operator, arbitrary channel graph, new hierarchy, new material architecture, new developmental rule, or new mechanism class. The current `topology_class` names are classifications over a fixed struct, not evolved topology.

The most consequential problem is deeper than limited expressiveness: the claimed genome, screening phenotype, Python CAD, CFD surface, CUDA evaluator, and structural evaluator do not all bind to one identical physical subject. Some genes are represented as wall-radius perturbations or arithmetic volume estimates rather than explicit internal geometry. Several F0 values are assigned by construction rather than independently measured. Invalid variation can silently collapse to the CT-35 baseline. A reduced-order model rewards the same hand-authored descriptors that define the candidate. This permits self-confirming optimization.

The required transformation is therefore not “add more genes.” It is:

> **Frozen question. Open-ended design language. One compiled physical subject. Independent physics. Immutable evidence.**

JOIL XenoGenome v4 should be implemented as four isolated systems:

1. **Inventor** — creates variable-length typed design programs, including graph, field, grammar, and developmental structure.
2. **Compiler** — deterministically lowers a genome into one canonical, inspectable physical subject.
3. **Science Court** — receives only the compiled subject and frozen scenario; it cannot read mechanism names or design intent.
4. **Evidence Ledger** — binds every observation, solver, mesh, model, requirement, artifact, and review to exact content hashes.

JARAMBIT should supply the scientific operating system. JOIL should supply the broader design problem and multi-fidelity search. Neither project’s fixed vector should be the future genome.

---

# 2. Repository findings

## 2.1 What JOIL gets right

JOIL has several foundations worth preserving:

- The science contract correctly distinguishes oil capture, treated concentration, reject split, inventory closure, pressure ratio, capacity, numerical error, validation, and physical superiority.
- The documentation explicitly leaves full-residence oil qualification and physical validation open.
- Unknown JSON fields are rejected.
- The CT-35 baseline and terminal interfaces are frozen and content-hashed.
- The current search combines constrained multi-objective ranking with diversity illumination.
- Search checkpoints include evaluator and CUDA hashes.
- The repository includes negative controls, multi-seed experiments, held-out duties, and explicit limitations.
- The current `docs/GENOME.md` already describes the right direction: variable channel graphs and implicit fields, and explicitly says that API is proposed rather than implemented.

This is a much stronger starting point than a typical generative-design repository.

## 2.2 What JARAMBIT gets right

JARAMBIT is narrower geometrically but substantially stronger in scientific accountability:

- The genotype maps deterministically to explicit outline, holes, thickness, and solid exports.
- Search records parent IDs, operator identity, and operator semantic version.
- Feasibility dominates optimization: an infeasible object cannot win by accumulating soft rewards.
- `Obs<T>` distinguishes a value, not evaluated, and failed; absence never silently becomes zero.
- `ScientificIdentity` binds subject, requirements, source, build, lockfile, config, evaluator, scenario, objective, descriptor, material, and dependencies.
- Numeric canonicalization preserves exact binary64 bits rather than decimal rounding.
- Evidence observations bind a specific quantity, unit, frame, region, recovery method, model, uncertainty, numerical state, and validation state.
- A synthetic or simulation result cannot masquerade as physical validation.
- Claim resolution is scoped to a frozen, trusted evidence inventory and supports review revocation or reopening.
- The event log is append-only and records every birth, elite appearance, migration, and generation boundary.

These should become common JOIL infrastructure, not remain product-specific JARAMBIT code.

## 2.3 What neither project provides

Neither repository currently provides all of the following:

- variable-length topology;
- evolved graph structure with homologous crossover;
- a bounded generative grammar;
- a developmental genotype;
- a learned or field-derived behavior space;
- a canonical shared phenotype IR across CAD, CFD, FEA, and visualization;
- evaluator blindness to authored mechanism labels;
- automatic causal ablation of discovered mechanisms;
- an explicit measure of scientific novelty distinct from visual strangeness;
- promotion based on conservative uncertainty bounds across a complete evidence inventory.

That is the gap XenoGenome v4 must close.

---

# 3. Stop-the-line findings in current JOIL

These findings should be addressed before claiming that the present Wave-2 genome performs open-ended invention.

## 3.1 The documented graph genome is not the implemented genome

`docs/GENOME.md` proposes `ChannelGraph` and `Implicit` recipes and explicitly says the graph CAD kernel is not implemented. `docs/WAVE2_PLAN.md`, by contrast, describes a `ct35-exotic-head-v3` with `InletManifoldGraph3D` and hierarchical 2→K branching. The current Rust `DesignProgram` remains a fixed struct containing inlet ramp, inlet split, ribbon, named TPMS insert, chamber modes, cone profile, and skimmer.

**Consequence:** the design space can only instantiate combinations anticipated by the schema author.

**Required action:** remove “flexible graph CAD implemented” from status until a variable graph is compiled into the exact physical subject.

## 3.2 Topology labels are not topology

The current eleven family names are derived from fixed gene combinations. “Cylindrical gyroid,” “octa branch,” “ribbon,” and similar labels do not establish that the corresponding internal surfaces or channel networks exist in the compiled artifact.

**Consequence:** family count and family transition count can overstate representational diversity.

**Required action:** delete `topology_class` from the evolvable genome. Derive topology and morphology descriptors after compilation from the actual artifact. Keep human-readable names only as post hoc annotations.

## 3.3 The screening mesh does not instantiate all claimed mechanisms

The Rust phenotype builds an axial ring surface using `inner_radius_mm(theta,z)`. Ribbon and TPMS terms alter the wall radius. Branch, skimmer, ribbon, and TPMS volumes are partly calculated arithmetically. `trapped_cavities` is assigned zero. Frozen-tail difference and terminal metrics are assigned rather than independently measured in the same compiled artifact.

**Consequence:** watertightness of the outer ring mesh does not prove watertightness, connectivity, pore accessibility, or manufacturability of the claimed internals.

**Required action:** a gene may influence science only if it creates explicit geometry in the canonical subject. No semantic-only mechanism genes.

## 3.4 The exact CAD paths disagree

The Python `wall_field.py` compiler states that it supports a smooth chamber wall only and that inlet graph and guide-vane compilers remain pending. `build_proposals.py` calls that wall-field compiler. The separate `exotic_graph.py` implements an older v2 schema with different branch and feature limits. The Rust schema has newer ribbon and TPMS fields.

**Consequence:** “the same genome” can mean different geometry depending on the code path.

**Required action:** there must be one semantic compiler and one canonical Phenotype IR. Alternative kernels may independently check the result, but they may not reinterpret the genome.

## 3.5 Invalid variation can silently become the baseline

Several mutation and crossover methods normalize using:

```rust
self.normalize().unwrap_or_else(|_| Self::baseline_ct35())
```

and return the baseline again when final normalization fails.

**Consequence:** invalid children become apparently valid CT-35 descendants, corrupting ancestry, operator statistics, diversity, archive occupancy, and benchmark results.

**Required action:** every variation operator returns a typed `VariationOutcome`. Failure is recorded as failure. A repair is explicit, versioned, and logged. Silent fallback is prohibited.

## 3.6 The reduced-order evaluator contains the design theory it is supposed to test

The F1 evaluator computes performance from hand-authored descriptors such as swirl flux, shear mitigation, core shielding, and cone acceleration. Those descriptors are themselves functions of the authored genes. Search therefore optimizes a model that already assumes the intended mechanisms work.

**Consequence:** the optimizer can discover the author’s equations rather than discover physics.

**Required action:** authored descriptors may guide proposal generation, but qualification observables must be extracted from an independent physical solution or measurement. The court receives geometry, materials, boundary conditions, and scenarios—not design-theory labels.

## 3.7 Structural qualification is an F1 screen, not 3D qualification

The current structural module uses Lamé cylinder equations, empirical concentration factors, simple beam formulas, and axial sampling. Load transfer error is guaranteed to zero by assigning mapped reactions equal to calculated loads. LPBF overhang is estimated from a few scalar genes. The observed-order expression also uses floating `div_euclid`, which is not ordinary division and should not be used in a GCI calculation.

**Consequence:** useful analytic screens are being placed too close to 3D FEA/manufacturing qualification.

**Required action:** relabel this layer `F1_ANALYTIC_STRUCTURAL_SCREEN`. Add actual 3D solid FEA, CFD-to-FEA load mapping residuals, modal convergence, fatigue, and manufacturing analysis before a structural qualification verdict.

## 3.8 CUDA and surrogate subjects are incomplete

The CUDA audit request sends only a subset of genome fields plus derived descriptors. The surrogate uses a twelve-dimensional hand-authored feature vector and is trained largely on reduced-order outputs. Leave-one-family-out validation over authored labels does not establish extrapolation safety for genuinely new topology.

**Consequence:** evaluation and uncertainty may be attached to a partial or theory-contaminated subject.

**Required action:** evaluators receive the canonical subject hash and canonical subject artifact. Surrogates encode the graph, field, mesh, operating condition, and fidelity directly; they must include explicit OOD rejection.

## 3.9 Some tests prove internal consistency rather than external truth

Tests that assert all generated families pass self-authored geometry gates, or that the exotic search must outperform random and conventional search under the same reduced model, are valuable regression tests but cannot be scientific evidence of superiority.

**Required action:** divide tests into:

- software-contract tests;
- independent geometry tests;
- numerical-verification tests;
- model-validation tests;
- scientific benchmark tests.

Never combine their verdicts.

---

# 4. Target system doctrine

The following rules are architectural invariants.

## D1. Freeze the question, not the answer vocabulary

The product envelope, ports, materials allowed, operating duties, safety requirements, comparison policy, and claim thresholds are immutable content-addressed inputs. The genome may invent any design that compiles inside that contract.

## D2. The compiled artifact is the scientific subject

A genome is a recipe. Science applies to the exact compiled fluid domain, solid domain, material field, interface map, and manufacturing state. Every observation binds to that subject hash.

## D3. A gene must have executable geometry semantics

No gene may earn a descriptor, score, family label, or mechanism claim unless it changes the canonical physical subject.

## D4. Inventor and judge are separated

The inventor may use heuristics, LLMs, prior physics, mechanism labels, and surrogate beliefs. The court may not. Court APIs accept only the subject, frozen scenario, and evaluator configuration.

## D5. Unknown never passes

Missing, failed, not run, numerically invalid, out of domain, and physically unvalidated are distinct states. None is converted to a favorable default.

## D6. No silent repair

Clamping, projection, topology repair, remeshing repair, and interface repair are explicit operators with receipts. A repair changes identity.

## D7. Complexity is budgeted, not categorically forbidden

The language is open-ended but gas-metered. Limits apply to nodes, rewrite steps, field evaluations, minimum feature scale, mesh resolution, and compute cost. They are campaign-versioned, not embedded as a permanent eleven-family ceiling.

## D8. Novelty is not evidence

Novelty determines exploration and archive placement. It never upgrades fidelity or validates a claim.

## D9. Promotion uses conservative bounds

A candidate passes a requirement only when the adverse confidence/coverage bound passes, all dependencies close, and the evaluator is valid for the candidate’s applicability envelope.

## D10. Every claim is revocable

New evidence, a solver defect, a material change, an invalid mesh, or a failed physical replication can reopen or revoke a prior verdict without rewriting history.

---

# 5. XenoGenome v4 representation

## 5.1 Use a hybrid representation

No single encoding is ideal. A pure direct vector has good locality but weak invention. A pure developmental encoding can create large novelty but may damage locality and crossover. XenoGenome should combine four layers:

1. **Direct interface spine** — frozen ports, keep-out regions, terminal frames, and product envelope.
2. **Variable typed topology graph** — channels, junctions, regions, solids, interfaces, and assemblies.
3. **Indirect field programs** — cross-sections, wall fields, lattices, textures, materials, and repeated organization.
4. **Bounded developmental grammar** — hierarchy, repetition, symmetry, differentiation, branching, and module growth.

A module can later be “decompressed” into explicit local parameters for fine tuning. This permits macro-scale invention and micro-scale optimization in the same lineage.

## 5.2 Canonical genome envelope

```rust
pub struct GenomeEnvelope {
    pub schema: SchemaRef,
    pub problem: ContentRef,
    pub interface_contract: ContentRef,
    pub root_module: ModuleId,
    pub modules: BTreeMap<ModuleId, Module>,
    pub expressions: BTreeMap<ExprId, TypedExpr>,
    pub rewrite_rules: Vec<RewriteRule>,
    pub material_program: MaterialProgram,
    pub process_program: ProcessProgram,
    pub operation_program: Option<OperationProgram>,
    pub budgets: ComplexityBudget,
    pub extension_manifests: Vec<ContentRef>,
}
```

The genome contains no free-form executable code. Every module and expression has a frozen semantic version.

## 5.3 Typed topology graph

```rust
pub enum Module {
    FrozenPort(FrozenPortRef),
    Junction(Junction),
    Channel(Channel),
    Chamber(Chamber),
    Shell(Shell),
    Lattice(LatticeRegion),
    Insert(Insert),
    Interface(InterfaceConstraint),
    Assembly(Assembly),
    Repeat(RepeatOp),
    Symmetry(SymmetryOp),
    FieldRegion(FieldRegion),
    MaterialRegion(MaterialRegion),
}
```

Each connection is typed:

```rust
pub struct PortType {
    pub domain: DomainKind,       // fluid, solid, thermal, electrical, assembly
    pub role: BoundaryRole,       // inlet, outlet, wall, bond, contact, free
    pub frame: FrameRef,
    pub section: SectionContract,
    pub continuity: Continuity,   // C0, C1, C2, flux-conserving
    pub unit_system: UnitSystem,
}
```

Compilation rejects incompatible edges before geometry construction.

## 5.4 Channel and chamber semantics

A channel edge should not be limited to a hard-coded branch count or section family:

```rust
pub struct Channel {
    pub start: TypedPortRef,
    pub end: TypedPortRef,
    pub centerline: CurveProgram,
    pub section: SectionProgram,
    pub orientation: FrameProgram,
    pub wall: WallProgram,
    pub boundary_tags: BoundaryTagProgram,
}
```

`CurveProgram`, `SectionProgram`, and `WallProgram` can be direct splines, expression graphs, CPPNs, or bounded developmental fields. A junction accepts N typed parents and M typed children, subject to geometric and conservation constraints. N and M are data, not enum variants.

## 5.5 Field expression language

The field language creates geometry from composable mathematics rather than named motifs alone.

Required primitives:

- coordinates, frames, distances, dot/cross products, norms;
- constants and dimensioned parameters;
- polynomial, rational, trigonometric, exponential, logistic, and radial basis functions;
- smooth minimum/maximum, union, intersection, subtraction, blend, offset, shell;
- repeat, phase, warp, twist, bend, taper, conformal map;
- deterministic band-limited noise;
- distance to graph feature, boundary, port, or medial field;
- small evolved CPPN nodes with innovation IDs;
- bounded PDE-growth operators such as diffusion, reaction-diffusion, curvature flow, and stress/flow-aligned deposition, when their numerical scheme and step count are frozen.

Example:

```rust
pub enum FieldExpr {
    Constant(Quantity),
    Coord(Axis, FrameRef),
    Add(ExprId, ExprId),
    Mul(ExprId, ExprId),
    Sin(ExprId),
    SmoothMin { a: ExprId, b: ExprId, radius: Quantity },
    Warp { field: ExprId, displacement: [ExprId; 3] },
    DistanceTo(ModuleId),
    Cppn(CppnGraph),
    ReactionDiffusion(BoundedPdeProgram),
}
```

Every expression carries units and output type. Adding a length to an angle fails type checking. NaN, infinity, nondeterministic state, unbounded recursion, and unknown operators fail decoding.

## 5.6 Bounded developmental grammar

```rust
pub struct RewriteRule {
    pub innovation_id: InnovationId,
    pub pattern: TypedSubgraphPattern,
    pub replacement: TypedSubgraphTemplate,
    pub guard: GuardExpr,
    pub max_applications: u32,
    pub priority: i32,
}
```

Development is deterministic for a given genome and seed. It has:

- maximum rewrite steps;
- maximum graph nodes and edges;
- maximum field DAG nodes;
- maximum spatial frequency;
- minimum resolvable feature size;
- maximum memory and compiler time;
- a canonical rule-application order.

A gas-exhausted genome is `COMPILE_BUDGET_EXCEEDED`, not repaired or partially accepted.

## 5.7 Material and manufacturing chromosomes

Geometry alone is not a complete design.

```rust
pub struct MaterialProgram {
    pub phases: Vec<MaterialRef>,
    pub assignment: MaterialField,
    pub interface_rules: Vec<MaterialInterfaceRule>,
}

pub struct ProcessProgram {
    pub process: ProcessRef,
    pub orientation: FrameProgram,
    pub support_policy: SupportPolicy,
    pub removal_access: RemovalAccessPolicy,
    pub finishing_access: FinishingPolicy,
    pub tolerance_model: ContentRef,
}
```

For the first JOIL lane, material and process choices may remain tightly frozen. They should still be explicit so later campaigns can evolve them without changing the genome architecture.

## 5.8 Keep the hydrocyclone as a domain plugin

The open-ended engine should be domain-neutral. CT-35 constraints belong in:

- `joil-domain-hydrocyclone`;
- `requirements/RS-joil-research-v2`;
- hydrocyclone-specific scenario and observable packages;
- hydrocyclone-specific manufacturing and fluid models.

The core genome should also be usable for heat exchangers, nozzles, structures, blades, coalescers, metamaterials, or other engineered systems.

---

# 6. Canonical phenotype compiler

## 6.1 One semantic pipeline

The genome must be interpreted exactly once:

```text
Genome bytes
  -> strict decoder
  -> unit/type/effect checker
  -> bounded developmental expansion
  -> graph embedding and constraint solve
  -> canonical Phenotype IR
  -> exact/implicit geometry kernel
  -> artifact audit
  -> canonical subject bundle
```

Python, Rust, CUDA, OpenCASCADE, meshers, viewers, and solvers consume the resulting IR or subject artifact. They do not independently reinterpret genes.

## 6.2 Phenotype IR

```rust
pub struct PhenotypeIr {
    pub schema: SchemaRef,
    pub units: UnitSystem,
    pub fluid_regions: Vec<RegionIr>,
    pub solid_regions: Vec<RegionIr>,
    pub material_fields: Vec<MaterialFieldIr>,
    pub named_boundaries: BTreeMap<BoundaryId, BoundaryIr>,
    pub contacts: Vec<ContactIr>,
    pub keep_outs: Vec<KeepOutIr>,
    pub process_state: ProcessStateIr,
    pub semantic_lineage: BTreeMap<EntityId, OriginTrace>,
}
```

`OriginTrace` maps every face, edge, cell region, and boundary back to genome modules and compiler operations. This supports persistent naming, ablation, causal analysis, load mapping, and debugging.

## 6.3 Exact and implicit backends

Use two roles:

- **Primary compiler backend:** the authoritative physical subject, preferably an OpenCASCADE B-rep for exact machinable boundaries plus volumetric fields for lattices/materials.
- **Independent audit backend:** voxel/SDF reconstruction and measurements at declared convergence levels.

The audit backend verifies volume, connected components, minimum wall, passages, cavities, interfaces, genus, terminal areas, and boundary orientation. It cannot silently substitute its geometry for the primary subject.

For field-heavy designs, the primary subject may be an adaptive implicit volume with a frozen extraction algorithm and tolerance. The subject identity must then include field bytes, domain, extraction method, and tolerance.

## 6.4 Subject bundle

```rust
pub struct SubjectBundle {
    pub phenotype_ir: ContentRef,
    pub fluid_brep: Option<ContentRef>,
    pub solid_brep: Option<ContentRef>,
    pub implicit_fields: Vec<ContentRef>,
    pub boundary_map: ContentRef,
    pub material_map: ContentRef,
    pub process_state: ContentRef,
    pub reference_tessellation: ContentRef,
    pub compile_receipt: ContentRef,
}
```

The scientific subject hash includes every component. A mesh is a derived artifact and has its own hash.

## 6.5 Compile receipt

```rust
pub struct CompileReceipt {
    pub genome: ContentRef,
    pub problem: ContentRef,
    pub compiler: ContentRef,
    pub geometry_kernel: ContentRef,
    pub operator_registry: Vec<ContentRef>,
    pub config: ContentRef,
    pub status: CompileStatus,
    pub diagnostics: Vec<Diagnostic>,
    pub subject: Option<ContentRef>,
    pub resource_usage: ResourceUsage,
}
```

Statuses must distinguish:

- decoded;
- type error;
- effect violation;
- budget exceeded;
- constraint unsatisfied;
- kernel failure;
- geometry invalid;
- manufacturing infeasible;
- success.

---

# 7. Variation and heredity

## 7.1 Transactional variation API

```rust
pub enum VariationOutcome {
    Child {
        genome: GenomeEnvelope,
        receipt: VariationReceipt,
    },
    Rejected {
        receipt: VariationReceipt,
        reason: VariationReject,
    },
    Failed {
        receipt: VariationReceipt,
        diagnostic: String,
    },
}
```

There is no fallback child.

## 7.2 Variation receipt

Record:

- parent subject and genome IDs;
- operator ID and semantic version;
- RNG state before and after;
- structural innovation IDs;
- exact edit script;
- any projection or repair;
- changed modules and expressions;
- compile result;
- phenotype distance from each parent.

## 7.3 Mutation operators

At minimum:

### Parametric operators

- dimension-aware scalar perturbation;
- spline control-point mutation;
- field coefficient mutation;
- local frequency refinement;
- material/process parameter mutation.

### Structural graph operators

- insert/delete module;
- split channel;
- rejoin channels;
- insert bypass;
- duplicate module;
- duplicate subgraph with variation;
- change connectivity;
- change hierarchy;
- add/remove chamber;
- add/remove internal region;
- module substitution by matching port type.

### Generative operators

- add/remove rewrite rule;
- modify rule pattern or replacement;
- alter repetition count or phase;
- symmetry creation;
- symmetry breaking;
- CPPN node/edge add/remove;
- activation-function mutation;
- field composition or warp insertion;
- developmental timing mutation.

### Science-guided operators

- stress-aligned reinforcement;
- flow-aligned channel growth;
- shear-hotspot smoothing;
- dead-zone puncture;
- manufacturability repair;
- tolerance-robust thickening;
- uncertainty-directed local refinement.

These operators may use prior evaluation fields, but every such dependency is recorded in the variation receipt.

## 7.4 Homologous crossover

Use NEAT-style innovation IDs and module lineage to align different graph topologies. Crossover is permitted only where typed interfaces match. Nonmatching innovations are inherited according to explicit rules and speciation context.

For developmental genomes, crossover occurs at rule, module, or expression-subgraph boundaries—not by arbitrary byte slicing.

## 7.5 Explicit repair

A repair is a named transformation such as:

- `restore_terminal_contract_v2`;
- `increase_minimum_ligament_v1`;
- `remove_trapped_powder_component_v1`;
- `reduce_field_frequency_to_mesh_budget_v1`.

The repaired genome is a new child. Search statistics report raw validity and post-repair validity separately. Repair frequency becomes a quality signal for operators.

---

# 8. Search architecture for open-ended invention

## 8.1 Separate archives

Maintain at least four archives:

1. **Feasible Performance Archive** — all hard gates pass at the current required fidelity.
2. **Speculative Frontier Archive** — geometrically valid, scientifically incomplete candidates with high expected value or information gain.
3. **Failure Atlas** — invalid geometry, numerical failure, physical failure, and manufacturing failure clustered by cause.
4. **Mechanism Archive** — candidates indexed by measured physical-response signatures and causal ablation signatures.

A candidate can occupy more than one archive, but archive status does not upgrade evidence.

## 8.2 Replace authored topology bins

The current key based on topology label, swirl bin, shear bin, and shield bin should be retired. Use descriptors derived from the artifact and solution fields.

Recommended descriptor sources:

### Geometry descriptors

- graph spectra and motif counts;
- branch/rejoin depth;
- Betti numbers and connected components;
- genus and cavity structure;
- cross-section distribution;
- curvature and thickness spectra;
- material topology;
- manufacturing-access graph;
- learned SDF or mesh embedding.

### Physics descriptors

- vortex-core skeleton and precession spectrum;
- helicity/vorticity distributions;
- residence-time distribution;
- pressure-loss spatial distribution;
- shear-dose distribution;
- phase transfer matrix;
- energy-flux decomposition;
- sensitivity to flow, viscosity, DSD, roughness, and tolerances.

### Causal descriptors

- performance delta after removing each module;
- response to phase reversal or mirror;
- matched-volume counterfactual delta;
- feature-scale sweep response;
- interaction effects between modules.

Use AURORA/VQ-style learned descriptors or CVT archives to avoid freezing human expectations into four bins. Descriptor encoder versions are frozen per campaign and included in scientific identity.

## 8.3 Quality and diversity policy

Hard constraints dominate. Among candidates at the same evidence tier:

1. compare conservative performance bounds;
2. compare robustness/regret;
3. compare cost and complexity;
4. use novelty and archive coverage for exploration.

Novelty should never compensate for a failed safety, geometry, manufacturing, or duty requirement.

## 8.4 Multi-fidelity scheduling

Use an evidence-aware scheduler:

```text
millions: decode + coarse implicit geometry
hundreds of thousands: exact F0 geometry/manufacturing
thousands: F1 reduced physics and uncertainty sweeps
hundreds: F2 mesh-converged CFD/FEA
few tens: F3 cross-model transient multiphysics
few: F4 manufactured tests
```

Actual budgets are campaign inputs. Promotion uses expected decision value per unit cost, not a fixed score threshold alone.

## 8.5 Memetic optimization

Evolution should handle topology and representation. Within a fixed compiled topology, use gradient, adjoint, Bayesian, or trust-region local optimization for continuous parameters. The local optimizer is a versioned variation operator with a complete receipt.

## 8.6 Adversarial scenarios without objective drift

A scenario generator may search for difficult operating conditions, manufacturing perturbations, or damage states **inside the frozen applicability envelope**. It may not redefine success. This gives POET-like curriculum and stepping stones without changing the qualification contract.

## 8.7 Operator evolution

Allow the inventor to propose new operators or grammar modules through signed extension manifests. An extension enters production only after:

- semantics are frozen;
- unit/effect rules exist;
- deterministic compilation is proven;
- challenge-corpus tests pass;
- negative controls exist;
- it cannot access court outputs except through declared observation inputs;
- its version and source hash enter identity.

An LLM may author an extension proposal, but deterministic code and tests define its meaning.

---

# 9. Science Court

## 9.1 Blind evaluator boundary

The Science Court API should look like:

```rust
pub fn evaluate(
    subject: &SubjectBundle,
    scenario: &FrozenScenario,
    evaluator: &EvaluatorIdentity,
) -> EvaluationBundle;
```

It must not accept:

- topology class;
- mechanism descriptors from the genome;
- claimed reason the design should work;
- family label;
- search fitness;
- parent performance;
- caller-supplied “qualified” flags.

## 9.2 Observation model

Port and extend JARAMBIT’s observation model:

```rust
pub struct Observation<T> {
    pub subject: ContentRef,
    pub scenario: ContentRef,
    pub observable: ObservableSpec,
    pub result: ObservationResult<T>,
    pub execution: ExecutionState,
    pub numerical: NumericalState,
    pub validation: ValidationState,
    pub applicability: ApplicabilityState,
    pub origin: EvidenceOrigin,
    pub uncertainty: Vec<UncertaintyComponent>,
    pub dependencies: Vec<ObservationRef>,
    pub producer: ContentRef,
}
```

`ObservationResult` includes `Value`, `Interval`, `NotEvaluated`, `Failed`, `Invalid`, and `NotApplicable`.

## 9.3 Fidelity ladder

### F−1: Identity and software contract

- strict decode;
- exact hashes;
- dependency closure;
- deterministic replay;
- unknown-field rejection;
- no duplicate identity;
- unit and frame consistency.

### F0A: Geometry

Measured from the compiled subject:

- valid fluid and solid regions;
- orientation and closure;
- self-intersection;
- connectedness;
- terminal areas, positions, normals, and tags;
- frozen-region symmetric difference;
- minimum wall, ligament, and passage;
- actual trapped cavities and inaccessible regions;
- B-rep/SDF/voxel measurement agreement with convergence.

### F0B: Manufacturing

- build orientation;
- true local overhang and support map;
- support-removal access;
- powder evacuation graph;
- drainage;
- minimum printable feature after compensation;
- distortion and shrinkage allowance;
- surface-finishing and inspection access;
- tolerance distributions;
- material/process passport applicability.

### F1: Independent low-cost physics

- analytical bounds;
- 0D/1D network models;
- reduced-order conservation models;
- coarse RANS/FEA;
- conservative breakup/coalescence bounds;
- robustness screens.

F1 models may reject candidates and prioritize search. They cannot validate an untested mechanism.

### F2: Numerically verified 3D physics

- mesh and time-step convergence on decision observables;
- iterative convergence;
- stochastic/particle convergence;
- exact boundary-condition closure;
- phase mass balance;
- CFD-to-FEA load transfer residual;
- solid FEA and modal convergence;
- declared turbulence, multiphase, and material model identities.

### F3: Model-form challenge

- alternative turbulence and multiphase closures;
- transient scale-resolving cases where needed;
- roughness, tolerance, fouling, erosion, and off-nominal duties;
- topology/feature-scale applicability checks;
- independent solver or implementation cross-check for high-consequence claims;
- explicit model discrepancy estimate.

### F4: Physical validation

- specimen passport;
- build and post-process records;
- dimensional metrology;
- material coupons;
- randomized paired baseline/candidate tests;
- blinded sample analysis where possible;
- replicate builds and fluid preparations;
- uncertainty and power analysis;
- applicability envelope and exclusions.

No computational status is automatically promoted to physical validation.

## 9.4 Numerical verification rules

- Calculate observed order with ordinary arithmetic, not Euclidean division.
- Refuse decorative GCI when convergence is oscillatory, nonmonotonic, or outside the asymptotic range.
- Perform convergence on the actual decision variables, not only an integrated internal proxy.
- Separate mesh, temporal, iterative, particle, sampling, and model-form uncertainties.
- Propagate grade-efficiency error to treated concentration and capacity decisions.
- Preserve common-random-number pairing without treating samples as independent when they are not.

## 9.5 Structural and manufacturing correction

The current analytic structure module remains valuable as F1. Add:

- tetrahedral/hex solid FEA on the exact solid subject;
- pressure and shear loads mapped from CFD wetted faces;
- independently integrated fluid and solid resultants;
- load-map residual threshold;
- local stress, strain, plasticity, fatigue, and notch convergence;
- modal and harmonic response with fluid added mass;
- VIV/PVC uncertainty;
- build anisotropy and residual-stress envelope;
- proof and burst factors;
- inspection-access and defect assumptions.

## 9.6 Hydrocyclone-specific physical observables

The court should report raw observables before derived scores:

- inlet, overflow, underflow volume and mass flux;
- oil transfer matrix from inlet size bins to each outlet size bin, retained, wall, and unresolved states;
- treated concentration with adverse unresolved-mass bounds;
- pressure taps, pressure drop, PDR, and energy flux;
- residence-time distribution;
- vortex-core trajectory, radius, stability, and precession spectrum;
- shear and turbulent dissipation exposure distributions along droplets;
- breakup and coalescence events with mass closure;
- absolute-pressure/cavitation margin;
- wall shear, erosion proxy, fouling/bridging risk;
- structural loads and moments;
- robustness to flow, viscosity, density, interfacial tension, DSD, roughness, and geometry tolerance.

---

# 10. Mechanism discovery and falsification

A strange design is not an invention until its effect survives causal challenge.

## 10.1 Mechanism card

```rust
pub struct MechanismCard {
    pub subject: ContentRef,
    pub hypothesis: String,
    pub predicted_observables: Vec<Prediction>,
    pub falsification_tests: Vec<TestRef>,
    pub counterfactuals: Vec<SubjectRef>,
    pub status: MechanismStatus,
}
```

The hypothesis is explanatory metadata. It is never an evaluator input.

## 10.2 Automatic counterfactual suite

For every promoted alien candidate, generate:

- module knockouts;
- branch-order permutations;
- phase randomization;
- twist reversal;
- mirror and rotational controls;
- matched-volume smooth control;
- matched-pressure-loss control;
- fixed-mass control;
- feature-scale sweeps;
- replacement with a conventional module;
- interaction ablations for module pairs.

A mechanism claim passes only if the measured response follows preregistered predictions and exceeds numerical/model uncertainty.

## 10.3 Separate performance novelty from scientific novelty

Report these independently:

- **genomic novelty** — edit distance in the typed program;
- **topological novelty** — graph and homology distance;
- **geometric novelty** — SDF/mesh embedding distance;
- **physical novelty** — field/transport signature distance;
- **causal novelty** — ablation-response distance;
- **scientific survival** — highest closed evidence tier.

Do not optimize a single “alienness” scalar for qualification. For reporting, an `AlienIndex` may be a geometric mean of normalized novelty components multiplied by an evidence-survival factor. A visually bizarre F0 object should never outrank a subtle F3 mechanism simply because it looks stranger.

## 10.4 Definition of an alien invention

A candidate earns the label **alien invention** only when:

1. it contains at least one structural innovation not representable as scalar changes to an existing module graph;
2. it occupies a new topology, geometry, or physical-response region relative to the seed and historical archive;
3. it passes exact F0 compilation and manufacturing checks;
4. it survives an independent F2 physical model with conservative uncertainty bounds;
5. its advantage survives matched counterfactual and ablation controls;
6. the proposed mechanism makes at least one successful preregistered prediction;
7. all artifacts and observations close under the evidence inventory;
8. the label is scoped to the tested applicability envelope.

---

# 11. Surrogate and active-learning redesign

## 11.1 Inputs

Replace the twelve hand-authored scalar vector with a multimodal representation:

- typed graph encoder;
- adaptive SDF or mesh encoder;
- boundary/interface encoder;
- material/process encoder;
- operating-condition encoder;
- fidelity/model identity encoder.

## 11.2 Outputs

Predict distributions, not single values:

- feasibility probabilities by gate;
- raw observables;
- aleatory and epistemic uncertainty;
- model discrepancy relative to higher fidelity;
- cost and failure probability.

Use separate heads or models for geometry validity, numerical failure, manufacturability, and performance.

## 11.3 Multi-fidelity learning

Train explicit mappings among:

- analytic F1;
- coarse CFD/FEA;
- converged CFD/FEA;
- transient/multiphase F3;
- physical F4.

Do not train the high-fidelity correction exclusively on survivors; preserve a stratified rejected cohort to prevent selection bias.

## 11.4 OOD policy

A surrogate prediction is unusable for promotion when any of these exceed thresholds:

- latent distance;
- ensemble disagreement;
- conformal residual bound;
- graph motif novelty outside training support;
- feature scale below training resolution;
- scenario outside applicability envelope;
- fidelity-correction uncertainty.

OOD candidates should receive physical evaluation priority when scientifically interesting, not favorable surrogate scores.

## 11.5 Validation

Replace authored family LOFO alone with:

- leave-one-graph-motif-cluster-out;
- leave-one-geometry-embedding-cluster-out;
- leave-one-feature-scale-band-out;
- leave-one-duty-out;
- temporal holdout by campaign;
- adversarial OOD challenge set;
- reliability diagrams and coverage tests;
- high-fidelity rank-reversal rate.

---

# 12. Evidence architecture

## 12.1 Port JARAMBIT evidence into a shared crate

Create a domain-neutral crate or repository:

```text
engineered-evidence/
  identity
  observations
  uncertainty
  inventory
  claims
  review
  eventlog
  artifact-store
```

JOIL and JARAMBIT should depend on it.

## 12.2 Extended scientific identity

```rust
pub struct ScientificIdentity {
    pub subject: SubjectRef,
    pub problem: ContentRef,
    pub requirements: Vec<ContentRef>,
    pub genome: ContentRef,
    pub compiler: ContentRef,
    pub geometry_kernel: ContentRef,
    pub source: ContentRef,
    pub build: ContentRef,
    pub lockfile: ContentRef,
    pub evaluator: ContentRef,
    pub model: ContentRef,
    pub mesh: Option<ContentRef>,
    pub numerics: ContentRef,
    pub scenario: ContentRef,
    pub observable: ContentRef,
    pub material: MaterialState,
    pub process: ProcessState,
    pub dependencies: Vec<ContentRef>,
}
```

Runtime host and thread count remain separate metadata unless they affect numerical semantics.

## 12.3 Evidence DAG

Every derived observation names exact dependencies. Examples:

```text
Cout upper bound
  <- outlet oil mass
  <- outlet water volume
  <- unresolved inventory bound
  <- phase mass balance
  <- particle convergence
  <- mesh/time convergence
  <- subject + scenario + solver
```

A claim resolver walks the DAG and returns:

- pass;
- fail;
- incomplete;
- invalid;
- reopened;
- revoked.

## 12.4 Generated status

`docs/STATUS.json` should be generated from the evidence registry. Manually written flags such as `flexible_graph_cad_implemented` or `structural_qualification` should not be authoritative.

---

# 13. Repository architecture

Recommended Rust workspace:

```text
crates/
  joil-spec                 frozen problem, requirements, scenarios
  joil-units                dimensional quantities and frames
  joil-genome               strict XenoGenome schema
  joil-grammar              bounded typed rewrite engine
  joil-field                expression DAG, CPPN, PDE growth
  joil-phenotype-ir         canonical physical IR
  joil-compiler             lowering and compile receipts
  joil-kernel-occt          authoritative B-rep bridge
  joil-kernel-implicit      adaptive SDF/volume backend
  joil-geometry-audit       independent B-rep/voxel checks
  joil-mesh                 CFD/FEA mesh derivation and identity
  joil-evidence             shared evidence system
  joil-eval-contract        blind evaluator protocol
  joil-eval-geometry        F0 geometry evaluator
  joil-eval-manufacturing   F0 manufacturing evaluator
  joil-eval-reduced         F1 screens
  joil-eval-cfd             F2/F3 fluid evaluation
  joil-eval-structure       F2/F3 structural evaluation
  joil-eval-multiphase      grade/PBM/full-residence evaluation
  joil-search               archives, selection, variation
  joil-surrogate            multimodal multifidelity models
  joil-mechanism            ablation and causal test generation
  joil-campaign             scheduler, workers, checkpoints
  joil-cli                  user interface

domains/
  hydrocyclone-ct35/

services/
  geometry-kernel/
  solver-workers/
  evidence-registry/
```

Keep solver wrappers out of the genome crate. Keep design descriptors out of evaluators. Keep human-readable reports downstream of evidence records.

---

# 14. Implementation sequence

## PR-0 — Scientific stop-the-line

- replace all baseline fallbacks with typed errors;
- fix GCI arithmetic and add monotonic/asymptotic checks;
- downgrade analytic structural status to F1 screen;
- block promotion for genes not instantiated in the CFD/solid subject;
- correct `STATUS.json` implementation flags;
- make CUDA audits reject partial subject binding;
- add regression tests proving invalid children cannot become CT-35.

**Exit:** no candidate can acquire evidence under a different geometry than its claimed genome.

## PR-1 — Shared evidence v2

- port JARAMBIT identity, exact numeric wire, observation state, uncertainty, trusted inventory, review, and event log;
- add JOIL units, frames, regions, scenarios, meshes, models, and process identity;
- generate status from registry.

**Exit:** every current result can be ingested as software, F1, F2, or incomplete evidence without overpromotion.

## PR-2 — Frozen ProblemSpec

- move interface, requirements, duties, material/process envelope, and claim policy into content-addressed schemas;
- separate hardware from synthetic CFD extensions;
- freeze evaluator applicability envelopes.

**Exit:** a design cannot change the question it is judged against.

## PR-3 — Phenotype IR and subject bundle

- define canonical fluid/solid/material/interface IR;
- preserve entity lineage;
- define canonical subject hash;
- add B-rep and SDF/voxel audit representations.

**Exit:** all downstream tools consume one subject.

## PR-4 — XenoGenome graph and field compiler

- implement typed graph;
- implement field DAG;
- implement bounded rewrite grammar;
- compile baseline and all challenge-corpus examples;
- support arbitrary branch/rejoin counts within budgets.

**Exit:** representation can express designs outside the current eleven families.

## PR-5 — Geometry and manufacturing court

- independent measurements;
- true cavity/powder/access checks;
- exact terminal/frozen-region checks;
- convergence of voxel measurements;
- process/tolerance identity.

**Exit:** no F0 value is a hard-coded success constant.

## PR-6 — Transactional variation and lineage

- innovation IDs;
- graph/grammar/field mutation;
- typed crossover;
- explicit repair;
- invalid/failure atlas;
- deterministic replay.

**Exit:** every child can be reconstructed from parents and receipt.

## PR-7 — Blind physics court

- evaluators receive only subject/scenario;
- remove mechanism descriptors from court inputs;
- map actual CFD loads to actual solid;
- add observation DAG and uncertainty.

**Exit:** design theory cannot directly certify itself.

## PR-8 — Open-ended QD

- learned geometry and physics descriptors;
- CVT or vector-quantized archive;
- separate feasible, speculative, failure, and mechanism archives;
- complexity and evidence-tier aware selection.

**Exit:** archive structure does not presuppose the mechanism families to be discovered.

## PR-9 — Multifidelity surrogate

- graph + geometry + scenario model;
- explicit fidelity correction;
- OOD rejection;
- conformal/ensemble calibration;
- high-fidelity rank-reversal tracking.

**Exit:** surrogate cannot promote an unsupported alien topology.

## PR-10 — Mechanism laboratory

- automatic ablation and matched controls;
- mechanism cards;
- causal signatures;
- scientific novelty metrics.

**Exit:** promoted alien designs have falsifiable explanations.

## PR-11 — XenoGenome hydrocyclone campaign

- equal-budget comparison against current fixed genome;
- preregistered metrics;
- hidden high-fidelity holdout;
- blinded finalist review;
- F4 test plan.

**Exit:** evidence demonstrates whether the new representation improves invention, not merely search score.

---

# 15. Acceptance criteria

## 15.1 Representation

- Exact CT-35 zero-mutation round trip.
- Arbitrary N→M typed branch/rejoin networks within budget.
- Nested hierarchy and module repetition.
- Non-axisymmetric free-form chambers.
- Explicit internal vanes and lattices.
- Mixed direct, CPPN, implicit, and developmental modules.
- At least 50 challenge designs impossible to encode in the current fixed schema.
- No permanent hard-coded family list.

## 15.2 Compiler

- Deterministic bit-identical subject hash for identical inputs.
- No silent repair or fallback.
- Unknown operator rejection.
- Semantic face/region lineage preserved.
- B-rep and voxel volume agree within declared converged tolerance.
- Trapped cavities are measured, not assigned.
- Minimum wall and passage measurements converge with audit resolution.

## 15.3 Variation

Across at least 100,000 operator applications:

- every outcome accounted for;
- zero invalid-to-baseline substitutions;
- replay reproduces every child;
- topology-changing accepted mutations exceed a preregistered minimum;
- operator success, repair, invalidity, novelty, and high-fidelity yield are reported separately.

## 15.4 Search

- Equal-budget benchmark versus fixed-vector JOIL, random/LHS, and conventional topology optimization.
- Archive coverage and QD score measured in frozen learned-descriptor versions.
- High-fidelity discovery rate reported.
- No benchmark asserts that XenoGenome must win; outcomes are measured, not encoded as unit-test expectations.
- Results replicated across seeds and withheld campaigns.

## 15.5 Surrogate

- calibrated interval coverage on in-distribution and cluster holdouts;
- explicit OOD rejection rate;
- high-fidelity rank-reversal below preregistered threshold;
- failures never inserted as favorable numeric targets;
- no promotion based solely on surrogate mean.

## 15.6 Science

- all decision observables have uncertainty budgets;
- F2 mesh/time/stochastic verification closes;
- phase mass balance closes;
- mapped load residual closes;
- alternate-model discrepancy is reported;
- every finalist has ablation controls;
- every public claim resolves from a frozen evidence inventory.

---

# 16. Highest-priority engineering changes

Ranked by impact:

1. **Create one canonical physical subject and force every evaluator to bind to it.**
2. **Remove silent baseline fallback immediately.**
3. **Port JARAMBIT scientific identity and evidence closure into JOIL.**
4. **Replace the fixed `DesignProgram` with a typed variable graph plus field language.**
5. **Move all mechanism descriptors out of the qualification evaluator.**
6. **Instantiate every claimed internal feature as explicit geometry.**
7. **Make geometry/manufacturing checks independent measurements, not assigned constants.**
8. **Reclassify current structural logic as F1 and add true 3D FEA/load mapping.**
9. **Replace authored topology bins with learned artifact/field descriptors.**
10. **Add automatic causal ablation before calling anything an invention.**

---

# 17. Final architectural statement

The strongest possible JOIL is not a larger hydrocyclone parameter optimizer. It is a **scientific invention machine** in which:

- an open-ended morphogenetic language can express structures the authors did not enumerate;
- a deterministic compiler makes those ideas physically explicit;
- independent solvers and experiments judge only the artifact;
- uncertainty and applicability are first-class;
- failures are retained as knowledge;
- causal tests distinguish mechanism from decorative strangeness;
- and every claim is bound to immutable, replayable evidence.

That is the path from “eleven exotic families” to genuine alien invention.

---

# Appendix A — Immediate code-level patch list

1. Change all `mutate_*` and crossover functions to return `Result<VariationOutcome, VariationError>`.
2. Delete every `unwrap_or_else(|_| baseline_ct35())` in variation.
3. Remove `topology_class` from serialized genome; derive annotations from the subject.
4. Remove hard-coded `trapped_cavities: 0` and frozen-difference constants.
5. Remove arithmetic feature volumes from geometry qualification.
6. Reject a candidate when a claimed gene is not represented in the canonical subject.
7. Replace partial CUDA JSON with subject-bundle reference plus exact artifact bytes.
8. Fix observed-order arithmetic and add GCI validity states.
9. Rename current structure verdict to `AnalyticStructuralScreen`.
10. Make OpenFOAM and FEA case manifests embed full scientific identity.
11. Move `MechanismDescriptors` to an inventor-only crate.
12. Replace `Option<f64>` science fields with typed observations.
13. Generate `STATUS.json` from the evidence registry.
14. Split `joil-core` into genome/compiler/evidence/evaluator/search crates.
15. Add a test that two different compilers for one genome cannot share a subject ID.
16. Add a test that inactive genes canonicalize but active unsupported genes fail.
17. Add a test that every boundary face has a semantic origin trace.
18. Add a test that a trapped cavity is found independently by B-rep and voxel audits.
19. Add a test that solver failure cannot become a performance number.
20. Add a test that a revoked dependency revokes every downstream claim.

# Appendix B — Research foundations used in this design

The architecture draws on:

- MAP-Elites and quality-diversity illumination;
- unsupervised behavior-descriptor learning such as AURORA;
- NEAT innovation IDs, structural alignment, and speciation;
- CPPN and generative encodings;
- graph grammars and typed shape grammars;
- bounded developmental encodings and neural cellular automata;
- POET-style stepping-stone transfer, constrained to a frozen qualification envelope;
- multi-fidelity Bayesian optimization;
- ASME/NASA verification and validation practices;
- NIST uncertainty and computational-model credibility principles.

The design deliberately uses a hybrid encoding because generativity, locality, mutation viability, and crossover robustness are different objectives. The evidence system prevents improvements in search creativity from being mistaken for improvements in scientific truth.
