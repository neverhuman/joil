# JOIL Alien Invention Engine
## Engineering Redesign Specification for an Open-Ended, Hard-Science Design Genome

**Status:** Proposed architecture and migration specification  
**Target repository:** `neverhuman/joil`  
**Reference repository:** `neverhuman/jarambit`  
**Recommended new schema name:** `joil-adg-v1`  
**ADG:** Alien Design Genome

---

# 1. Executive verdict

JOIL does not primarily need more feature knobs, more named hydrocyclone families, or a larger optimizer. It needs a representation break.

The present system has an unusually strong scientific contract for fluids, uncertainty, operating duty, numerical verification, and physical validation. That is the right foundation. The weakness is that the implemented design language is still a finite catalogue of human-authored motifs: inlet ramp, inlet split, chamber harmonics, cone modes, skimmer, ribbon, and a TPMS selector. The current search can create interesting combinations of those motifs, but it cannot yet invent arbitrary geometry or genuinely new topology.

The most important technical risk is deeper than limited expressivity: a substantial portion of the screening physics is computed from hand-authored descriptors that are themselves computed directly from the genes. The optimizer therefore searches a model of what the feature names are expected to do. It is not consistently forced to discover what the compiled geometry actually does. This creates a circular selection loop:

`gene choice -> hand-coded mechanism descriptor -> hand-coded performance equation -> fitness`

That loop can be useful as an early proposal heuristic, but it is not a neutral scientific judge. In an intelligent design system, the generator may know the genome; the evaluator should see the compiled physical subject, not the names of the genes that created it.

The target architecture should combine three strengths:

1. **JOIL's scientific discipline:** operating envelopes, mass closure, pressure and energy accounting, multi-fidelity CFD, numerical verification, held-out duties, and paired physical experiments.
2. **Jarambit's evidence discipline:** content-bound scientific identity, requirement-scoped verdicts, typed missing/failed observations, append-only lineage, operator versioning, explicit hard gates, and generated claim surfaces.
3. **A new open-ended representation:** a variable-length, strongly typed geometry program capable of producing channel graphs, arbitrary implicit fields, branch/rejoin networks, non-star-shaped domains, true porous structures, multi-region solids, and developmental geometry.

The central recommendation is therefore:

> Freeze the current JOIL representation as a legacy motif language, preserve all of its validated fixtures, and build `joil-adg-v1` as a typed program graph compiled into an independently audited geometry intermediate representation.

---

# 2. What JOIL currently does well

The redesign should preserve the parts that are already unusually strong.

## 2.1 Scientific objective discipline

JOIL correctly frames performance around treated-water compliance at a common duty, not a free-floating proxy. Its science contract distinguishes reject split, oil-mass capture, treated concentration, pressure, energy, unresolved inventory, and uncertainty. It also separates screening claims from physical qualification.

This is a much better foundation than a conventional "maximize a CFD scalar" optimizer.

## 2.2 Evidence-tier awareness

The repository explicitly distinguishes reduced-order screening, carrier CFD, full-residence oil closure, scale-resolving models, and physical validation. It does not treat an F1 screen as an F4 result. This must remain a hard architectural rule.

## 2.3 Frozen interfaces and comparator discipline

The fixed terminal geometry and common-duty comparison are excellent. They prevent the search from winning by silently changing the problem. The new genome should preserve the same immutable interface-contract concept, but express it as a content-addressed object rather than scattering constants through code.

## 2.4 Numerical and experimental thinking

The existing material on mesh refinement, turbulence-model cross-checks, oil inventory, A/B/A blocks, holdout isolation, as-built metrology, and power analysis is exactly the right kind of scientific skepticism.

## 2.5 Determinism and campaign accounting

JOIL already contains deterministic RNG, canonicalization attempts, checkpoint compatibility, evaluator hashes, failure quarantine, and a budget ledger. These are valuable. They should be lifted into first-class shared infrastructure and tied to a stronger scientific identity.

---

# 3. What Jarambit contributes

Jarambit is not the model for open-ended geometry. Its principal genotype is also a bounded domain-specific vector, and its primary phenotype is deliberately constrained. Jarambit's value is governance and evidentiary architecture.

## 3.1 Requirement-scoped truth

Jarambit distinguishes a screen verdict from a promotion verdict and binds both to a named requirement set. A candidate may be excellent under one scope and fail another. JOIL should adopt this rigor for every claim.

## 3.2 Typed observations

Jarambit represents a value, not evaluated, and failed as different states. It does not silently turn absence into a default scalar. JOIL should extend this further with out-of-envelope and superseded states.

## 3.3 Scientific identity

Jarambit binds the subject, requirements, source, build, lockfile, configuration, evaluator, scenario, objective, descriptor, material state, and dependencies. It also serializes floating-point values by exact binary representation. This is a major improvement over version-local JSON hashing.

## 3.4 Event-sourced lineage

Jarambit records parents, variation operator, operator semantics version, migration, elite appearances, and durable generation boundaries. JOIL needs the same lineage for every birth, rejected proposal, repair, compilation, evaluation, and promotion.

## 3.5 Hard gates versus search preferences

Jarambit makes feasibility structurally distinct from soft goals. JOIL already embraces this concept in its science documentation, but its implementation should use typed gate inventories rather than a combined numeric violation and loosely typed optional qualification fields.

## 3.6 Honest representation limits

Jarambit explicitly says when a desired feature cannot be expressed by the current genome. JOIL should do the same. A named "topology family" should not be treated as proof that the corresponding topology exists in the compiled subject.

---

# 4. Critical findings in the current JOIL implementation

## 4.1 The implemented genome is a feature catalogue, not a general design program

The current Rust `DesignProgram` contains a fixed set of fields:

- inlet ramp
- inlet split
- helical ribbon
- TPMS selector and parameters
- chamber harmonic modes
- three cone modes
- skimmer

This is a useful macro-genome. It is not trans-dimensional in the strong sense. The number and kinds of mechanisms are fixed in the Rust type. Branch count varies only within a small bounded menu. The "topology class" is a classification string computed from which fields are active.

The written Wave-2 plan describes a directed graph of junctions and swept branches, but the current core type does not implement that graph.

### Consequence

Search can combine known ideas, but it cannot create:

- an unanticipated number of branch and merge stages
- arbitrary channel-network topology
- nested or braided passages
- multiple independent internal solids
- arbitrary non-star-shaped cavities
- new porous morphologies not included in the enum
- developmental structures whose complexity grows with the genome

## 4.2 The primary Rust phenotype is fundamentally star-shaped

The core wall geometry is evaluated as a radius `R(theta, z)` and triangulated as axial rings. This is robust for a single star-shaped bore, but it excludes a large class of alien structures:

- overhanging fluid regions in a cross-section
- multiple radial intersections along one ray
- detached internal bodies represented as independent surfaces
- reentrant cavities
- braided channels
- true branch/rejoin networks
- resolved bicontinuous TPMS sheets

The current implementation often accounts for internal motifs through volume subtraction, scalar clearance estimates, or a wall-radius modulation. That is not equivalent to compiling and auditing the actual internal surface.

## 4.3 Several F0 facts are asserted rather than independently measured

In the current phenotype path, fields such as frozen-tail difference and trapped-cavity count are set to zero. The red-team gate then trusts those values. A hard gate should derive these quantities from an independent artifact, not from a field supplied by the same compiler being audited.

The same concern applies to approximate TPMS pore size, solid fraction, and head mass. These are useful estimates, but they must not stand in for geometry-derived measurements when a claim depends on them.

## 4.4 The screening physics is structurally self-referential

The current mechanism descriptors are formulas of the genes. The reduced-order evaluator then uses those descriptors to compute split, shear, pressure, core coherence, short circuiting, droplet efficiency, and concentration.

This creates a direct incentive for the optimizer to activate whatever gene receives a favorable coefficient. It also means a feature can receive a benefit even if the compiled geometry does not realize the intended mechanism.

### Required rule

From F1 upward, the evaluator must not receive named feature genes or a topology-class label. It should receive:

- compiled geometry
- semantic boundary surfaces
- materials
- operating scenario
- evaluator configuration

Descriptors used for selection should be extracted from geometry or computed physical fields.

## 4.5 Invalid offspring can silently become the baseline

Several mutation and crossover paths normalize an offspring and fall back to `baseline_ct35()` when normalization fails. This is scientifically dangerous because it:

- hides operator failure rates
- duplicates the baseline under unrelated lineages
- distorts archive occupancy
- corrupts operator-credit statistics
- makes a failed mutation look like a valid birth
- masks the actual boundary of the representation

Every proposal must terminate in one explicit state:

- valid birth
- rejected birth with reasons
- repaired birth with a complete repair transcript
- infrastructure failure

No proposal may silently become another design.

## 4.6 Schema identity is erased

The normalizer accepts more than one schema/family combination but canonicalizes the result back to the older schema/family. A schema migration should never erase its source semantics. The migration itself must be versioned, content-bound, and recorded.

## 4.7 The current "graph crossover" is not graph crossover

The operator called homologous graph crossover blends scalar fields and swaps fixed substructures. This is a reasonable macro crossover, but it is not subgraph matching or typed graph grafting. Naming should reflect actual semantics because operator names become part of scientific lineage.

## 4.8 The current structure model is not shape-general

The analytical Lamé, lobe, cone, vane, and skimmer calculations are useful screening oracles for baseline-like geometries. They cannot qualify arbitrary alien solids. Once the representation supports general topology, structural qualification must consume the actual solid mesh and mapped loads.

A specific code concern should also be corrected immediately: the observed-order calculation in the axial stress convergence path appears to use floating-point Euclidean division rather than ordinary division. A GCI observed order should use the logarithmic ratio divided by `ln(r)`, not a floored Euclidean quotient.

## 4.9 DSD weighting in the F1 screen needs an explicit basis

The current bin weighting is generated from a pointwise lognormal PDF and ad hoc widths. The science contract correctly requires number, area, volume, or mass basis and bin-edge conservation. The screening implementation should carry this basis explicitly and integrate over declared bin edges.

## 4.10 The surrogate silently imputes missing physical state

The current surrogate feature extraction supplies fallback values if phenotype or structural results are absent. This should be prohibited. Missing evidence must remain missing. A separate model may be trained for partial records, but it must not make absence look like a measured nominal value.

## 4.11 A 100% valid operator walk is not proof of open-endedness

A high validity rate is useful for a constrained macro-language. It can also indicate that the search is walking inside a narrow hand-authored safe manifold. The correct expressiveness test is not merely "all named families compiled." It is whether the system can produce measurably different topology and mechanism without those categories being enumerated in advance.

---

# 5. Design principles for `joil-adg-v1`

## Principle 1 — The genome is a program, not a vector

The genome should be a variable-length typed graph whose nodes construct curves, fields, regions, solids, and semantic boundaries.

## Principle 2 — The physical subject is compiled, not inferred from gene names

Fitness and scientific descriptors should be computed from the compiled subject and evaluator outputs.

## Principle 3 — Generation and judgment are separated

The generator may use priors, language models, motifs, and learned proposal models. The evaluator must be blind to the proposal story.

## Principle 4 — No silent repair

Every modification to a proposed genome is explicit, bounded, and content-addressed.

## Principle 5 — No topology label without a topology certificate

Topology is derived from the geometry: connected components, paths, Betti numbers, genus, percolation, and region adjacency.

## Principle 6 — Novelty is an acquisition signal, not evidence of superiority

Alienness may decide what to explore. It cannot qualify performance.

## Principle 7 — Missing and failed are not numbers

There are no scientific default values.

## Principle 8 — Every claim is an inventory closure

A claim is eligible only when all required observations, artifacts, reviews, and uncertainty statements are present and valid.

## Principle 9 — Complexity is bounded by resolvability and manufacturing

Do not impose a fixed vector length merely for convenience. Bound the program through minimum feature size, compiler cost, mesh cost, manufacturing process, and evaluation budget.

## Principle 10 — The current system remains a supported legacy lane

The existing motif language should remain reproducible as `legacy-ct35-v2`, serving as a baseline, calibration corpus, and macro seed library.

---

# 6. Proposed repository architecture

The monolithic core should be decomposed into crates with explicit trust boundaries.

```text
crates/
  joil-units/                  typed physical quantities and coordinate frames
  joil-content/                ContentRef, canonical wire, strict decoding
  joil-requirements/           requirement sets and claim inventories
  joil-evidence/               observations, uncertainty, reviews, verdicts
  joil-lineage/                births, operators, event log, checkpoints

  joil-genome/
    src/adg_v1/
    src/legacy_ct35_v1/
    src/legacy_exotic_v2/

  joil-geometry-ir/            representation-neutral geometry graph
  joil-compiler-brep/          OpenCASCADE/native B-rep compiler
  joil-compiler-sdf/           adaptive implicit/octree compiler
  joil-mesh/                   surface/volume meshing and semantic patches
  joil-topology/               connectivity, Betti numbers, percolation
  joil-manufacturing/          LPBF/machining/assembly gates

  joil-evaluator-api/          isolated evaluator protocol
  joil-eval-f0/                geometry and manufacturing certificates
  joil-eval-rom/               explicitly screening-only reduced physics
  joil-eval-cfd/               OpenFOAM and extraction adapters
  joil-eval-particles/         droplet/PBM evaluation
  joil-eval-structure/         general solid FEA and modal analysis
  joil-eval-experiment/        physical test records and analysis

  joil-descriptors/            phenotype and behavior descriptor extraction
  joil-surrogate/              multi-fidelity predictive models
  joil-search/                 QD, Pareto, islands, active learning
  joil-redteam/                adversarial corpus and falsification
  joil-cli/
```

The existing `joil-core` can initially re-export these crates during migration.

---

# 7. The Alien Design Genome

## 7.1 Top-level subject

```rust
pub struct AlienDesignGenome {
    pub schema: ContentRef,              // joil.adg.v1
    pub interface_contract: ContentRef,  // frozen cartridge boundary
    pub graph: ProgramGraph,
    pub outputs: DesignOutputs,
    pub material_program: MaterialProgram,
    pub manufacturing_intent: ManufacturingIntent,
}
```

Search strategy parameters should not be part of the physical genome. They belong in lineage state so two identical physical designs do not acquire different subject identities merely because they were mutated differently.

## 7.2 Typed program graph

```rust
pub struct ProgramGraph {
    pub nodes: BTreeMap<NodeId, Node>,
    pub edges: Vec<TypedEdge>,
}

pub struct Node {
    pub id: NodeId,
    pub operator: Operator,
    pub parameters: ParameterBlock,
}

pub struct TypedEdge {
    pub from: OutputPort,
    pub to: InputPort,
}
```

Every operator declares input and output types. Examples:

```rust
pub enum GeoType {
    Scalar,
    Vec3,
    Curve3,
    FrameField3,
    Section2,
    ScalarField3,
    VectorField3,
    Surface3,
    Region3,
    RegionSet3,
    BoundarySet,
    MaterialField3,
}
```

The graph is rejected before geometry execution if any edge is type-incompatible.

## 7.3 Generic operators

The base language should be mechanism-neutral.

```rust
pub enum Operator {
    Constant(ConstantOp),
    Curve(CurveOp),
    Section(SectionOp),
    Primitive(PrimitiveOp),
    Sweep(SweepOp),
    Loft(LoftOp),
    Field(FieldOp),
    Boolean(BooleanOp),
    Shell(ShellOp),
    Offset(OffsetOp),
    Transform(TransformOp),
    Warp(WarpOp),
    Pattern(PatternOp),
    Network(NetworkOp),
    Label(LabelOp),
    Constraint(ConstraintOp),
}
```

Named hydrocyclone mechanisms should be implemented as macros that compile into these primitives. A `GyroidShell` macro is useful, but it must not be the only way to produce a porous field.

## 7.4 Curves and sections

Curves should support:

- line and arc
- cubic B-spline
- clothoid
- helix
- composite curve
- field-integrated streamline
- graph-generated centerline

Sections should support:

- circle and ellipse
- rounded rectangle
- superellipse
- arbitrary periodic B-spline
- field-defined section
- multi-lobed section

Section schedules should vary along normalized arc length and include area, orientation, eccentricity, and wall offset.

## 7.5 Field expression language

A compact field program can create immense morphological diversity.

```rust
pub enum FieldExpr {
    Coordinate(Axis),
    Constant(Finite),
    Add(FieldId, FieldId),
    Sub(FieldId, FieldId),
    Mul(FieldId, FieldId),
    SafeDiv { a: FieldId, b: FieldId, floor: Finite },
    Min(FieldId, FieldId),
    Max(FieldId, FieldId),
    SmoothMin { a: FieldId, b: FieldId, radius: Length },
    Abs(FieldId),
    Sin(FieldId),
    Cos(FieldId),
    Tanh(FieldId),
    Gaussian(FieldId),
    DistanceTo(GeometryRef),
    Periodic { basis: PeriodicBasis, transform: Affine3 },
    Voronoi { seeds: SeedProgram, metric: Metric3 },
    BandLimitedNoise { seed: u64, wavelength: Interval<Length> },
    Warp { source: FieldId, displacement: VectorFieldId },
    Window { source: FieldId, mask: FieldId },
}
```

Each expression should produce static certificates where possible:

- value interval
- gradient bound
- differentiability class
- minimum wavelength
- singularity risk
- deterministic evaluation budget

A deterministic CPPN or small implicit neural field may be added as one field operator, but it must have:

- fixed supported activations
- exact weight serialization
- architecture bounds
- gradient/Lipschitz estimates
- explicit minimum-feature filtering
- no external runtime dependency

## 7.6 Channel-network operator

A high-level channel network is still valuable:

```rust
pub struct ChannelNetwork {
    pub junctions: BTreeMap<JunctionId, Junction>,
    pub branches: BTreeMap<BranchId, Branch>,
    pub terminal_bindings: Vec<TerminalBinding>,
}

pub struct Branch {
    pub from: JunctionId,
    pub to: JunctionId,
    pub centerline: CurveRef,
    pub section_schedule: SectionScheduleRef,
    pub local_frame: FrameFieldRef,
}
```

This graph must support arbitrary branch and rejoin stages. It should not be restricted to a predefined list of counts.

Static checks should include:

- terminal reachability
- flow-region connectivity
- branch area continuity
- maximum diffuser angle
- curvature relative to hydraulic diameter
- branch separation
- no orphan junctions
- no zero-length branches

## 7.7 Design outputs

```rust
pub struct DesignOutputs {
    pub fluid_regions: Vec<RegionRef>,
    pub solid_regions: Vec<RegionRef>,
    pub removable_regions: Vec<RegionRef>,
    pub support_regions: Vec<RegionRef>,
    pub semantic_boundaries: Vec<BoundaryBinding>,
}
```

This permits actual internal solids, porous shells, multiple material regions, and assemblies.

## 7.8 Interface contract

The frozen interface should be a content-addressed artifact containing:

- external envelope
- immutable downstream geometry
- terminal surfaces
- terminal frames, normals, and areas
- no-go regions
- allowed attachment surfaces
- coordinate convention
- unit convention
- tolerances

The compiler should intersect the candidate with the immutable interface and prove exact equality in the protected region.

---

# 8. Canonicalization and subject identity

## 8.1 Exact numeric wire

Use exact binary64 serialization for floating-point values. Do not use rounded JSON text as the scientific identity.

## 8.2 Structural graph hashing

Canonical graph identity should be computed from operator semantics and input hashes, not arbitrary node IDs.

For an acyclic program graph:

1. Canonicalize each parameter block.
2. Hash source nodes.
3. Hash each node from operator version, parameters, and sorted input-port hashes.
4. Hash the declared outputs.
5. Compute a Merkle root.

Commutative operators must sort child hashes. Ordered operators must preserve order.

## 8.3 Schema migration

A migration produces:

- source genome hash
- target genome hash
- migration tool hash
- migration semantics version
- deterministic transcript
- equivalence audit result

A migrated genome is never silently serialized as an older schema.

## 8.4 Phenotype identity

The following identities must remain separate:

- genome identity
- geometry-IR identity
- exact B-rep identity
- implicit field identity
- surface-mesh identity
- volume-mesh identity
- as-built specimen identity

Two genomes may compile to the same phenotype. Search deduplication should use phenotype identity, while lineage retains both genome identities.

---

# 9. Geometry compilation

## 9.1 Compile stages

```text
Genome
  -> type check
  -> dimensional check
  -> interval/singularity analysis
  -> GeometryIR
  -> B-rep compiler
  -> implicit/octree compiler
  -> topology audit
  -> semantic-surface audit
  -> manufacturing audit
  -> mesh generation
  -> subject bundle
```

## 9.2 GeometryIR

```rust
pub struct GeometryIr {
    pub schema: ContentRef,
    pub coordinate_frame: Frame3,
    pub regions: Vec<RegionIr>,
    pub semantic_surfaces: Vec<SemanticSurfaceIr>,
    pub immutable_interface: ContentRef,
    pub tolerance_policy: TolerancePolicy,
    pub compile_trace: Vec<CompileStep>,
}
```

The IR must use SI units internally and retain declared source-unit transforms.

## 9.3 Dual compiler requirement

Every new representation class should compile through two independent paths where possible:

- native B-rep path
- adaptive SDF/octree path

The comparison certificate should include:

- volume difference
- surface-area difference
- symmetric Hausdorff distance
- terminal-surface agreement
- connected-component agreement
- Betti-number agreement
- minimum-wall agreement
- minimum-gap agreement

If a reliable B-rep cannot be produced, retain an explicit mesh/implicit subject and restrict claims. Never emit a nominal STEP file from an unsewn shell and call it exact CAD.

## 9.4 True topology audit

Compute independently for fluid and solid domains:

- connected components
- Euler characteristic
- Betti numbers
- genus where defined
- terminal-to-terminal reachability
- number of distinct channel paths
- dead-end branches
- trapped fluid components
- trapped powder/support components
- percolation across porous regions
- minimum cut area between terminals

The `topology_class` should be derived from this certificate or replaced by a topology signature.

## 9.5 Actual TPMS geometry

A TPMS feature must be represented as the level set or sheet offset of the chosen field, clipped by an explicit window. The compiler must extract the actual sheet, not approximate it with wall corrugation or a volume fraction.

Audit:

- bicontinuity
- sheet thickness
- local pore hydraulic diameter
- disconnected islands
- trapped cells
- core obstruction
- curvature extremes
- mesh convergence of surface area and porosity

## 9.6 Manufacturing audit

For LPBF:

- local wall and ligament thickness
- local overhang angle from actual surface normals
- support requirement
- support removal path
- powder evacuation connectivity
- recoater collision risk
- thermal island and distortion proxy
- roughness-sensitive passage size
- build orientation identity
- post-processing/tool access

For machining:

- tool reach
- cutter-radius compatibility
- undercut access
- fixturing
- minimum internal radius
- sequence feasibility

The manufacturing process is part of the requirement set and subject identity.

---

# 10. Birth, mutation, crossover, and repair

## 10.1 Birth outcome

```rust
pub enum BirthOutcome {
    Valid {
        child: GenomeRef,
        transcript: BirthTranscript,
    },
    Rejected {
        proposal: GenomeRef,
        reasons: Vec<BirthReject>,
        transcript: BirthTranscript,
    },
    Repaired {
        proposal: GenomeRef,
        child: GenomeRef,
        repair: RepairTranscript,
        transcript: BirthTranscript,
    },
    InfrastructureFailure {
        reason: String,
    },
}
```

No path returns the baseline unless the operator explicitly proposed the baseline.

## 10.2 Operator semantics version

Every operator has a content reference and semantics version. A change in clamping, repair, grafting, or random-number consumption creates a new operator identity.

## 10.3 Numeric mutation

- log-space mutation for positive scales
- reflected or transformed bounded distributions
- covariance adaptation within a module
- self-adaptive step sizes stored in lineage state
- no hard clamp without a transcript

## 10.4 Structural mutations

- insert node
- delete inactive or redundant node
- duplicate subgraph
- prune subgraph
- replace operator with type-compatible operator
- increase/decrease field basis rank
- split branch
- merge branches
- add branch/rejoin loop
- change representation of a region
- break or introduce symmetry
- change material-region partition
- alter scale hierarchy

## 10.5 Homologous crossover

1. Compute typed interface signatures for candidate subgraphs.
2. Match subgraphs by input/output types, semantic context, scale, and local frame.
3. Graft a compatible donor subgraph.
4. Insert explicit transition operators if necessary.
5. Re-solve continuity constraints.
6. Compile and audit.
7. Record every matched and inserted node.

## 10.6 Non-homologous invention operators

To create large conceptual jumps:

- typed random program synthesis
- developmental seed mutation
- motif exaptation into a new region
- representation transposition
- symmetry rupture
- scale duplication
- recursive graph rewrite
- field-to-network conversion
- network-to-field conversion
- archive recombination across behaviorally distant parents

These operators should have protected exploration budget even when their short-term validity rate is lower.

## 10.7 Repair policy

Allowed repairs must be named and bounded:

- solve local area continuity
- move a control point away from collision
- reduce field amplitude to satisfy minimum feature size
- add a transition loft at a typed graft
- project a terminal to its frozen frame

Every repair records:

- pre-repair hash
- post-repair hash
- algorithm identity
- changed parameters
- geometric displacement
- repair objective
- convergence status

Reject if repair displacement exceeds a predeclared threshold.

---

# 11. Scientific evaluator architecture

## 11.1 Evaluator blindness

The evaluator request must not contain:

- motif names
- topology-class marketing labels
- expected mechanism descriptions
- parent fitness
- search rank

It may contain only the physical subject, requirements, scenario, and evaluator configuration.

## 11.2 Evaluator API

```rust
pub struct EvaluationRequest {
    pub subject: SubjectBundleRef,
    pub requirements: ContentRef,
    pub scenario: ContentRef,
    pub evaluator: ContentRef,
    pub resource_budget: ResourceBudget,
}

pub enum EvaluationOutcome {
    Observed(EvidenceBundle),
    RejectedByGate(Vec<GateFailure>),
    NotApplicable(ApplicabilityReason),
    NumericalFailure(FailureRecord),
    InfrastructureFailure(FailureRecord),
    OutOfEnvelope(ApplicabilityReason),
}
```

## 11.3 Fidelity ladder

### F0 — Geometry and manufacturing

- dual compiler agreement
- exact interface
- topology
- manifold/orientation
- feature sizes
- manufacturing
- tolerance envelope
- solid and fluid connectivity

### F0.5 — Cheap subject-based physics

Use actual geometry:

- hydraulic-network resistance
- potential-flow or inviscid diagnostics
- coarse pressure-loss screen
- streamline residence and short-circuit screen
- actual-mesh structural proxy

These are filters, not qualification.

### F1 — Reduced 3D physics

- coarse RANS or verified reduced PDE model
- particle trajectories on computed fields
- explicit DSD basis
- conservation and residual gates
- uncertainty and OOD status

### F2-Carrier — Verified carrier CFD

- at least three spatial resolutions when a convergence claim is made
- temporal convergence where transient
- closure-model comparison
- boundary-condition and outlet-extension checks
- exact extraction identity

### F2-Oil — Full-residence oil evaluation

- inlet-to-outlet transfer matrix
- complete inventory
- retained-mass threshold
- bin-edge refinement
- parcel/substep convergence
- treated concentration uncertainty

### F3 — Scale-resolving and population balance

- transient coherent structures
- breakup/coalescence
- model discrepancy
- upset scenarios

### F4 — Physical validation

- as-built metrology
- randomized paired blocks
- independent preparations
- matched comparator
- power analysis
- uncertainty-bound claim

## 11.4 Applicability is an output

Each evaluator returns an applicability statement. A novel topology is not automatically inside a surrogate or calibration envelope because its dimensions are numerically bounded.

## 11.5 Solver disagreement

Disagreement between models is evidence, not noise to average away. Store each result and a discrepancy observation.

## 11.6 Control-policy separation

Geometry and operating control must be distinct subjects. Fixed-duty comparison uses the same declared controller. Later geometry/control co-design requires a separate requirement set and comparator policy.

---

# 12. Replace gene-derived mechanism descriptors

Current descriptor names can remain as hypotheses, but their values should be measured.

## 12.1 Geometry descriptors

- fluid and solid Betti numbers
- channel path count
- path-length and tortuosity distributions
- hydraulic-diameter distribution
- curvature spectrum
- surface-area-to-volume ratio
- anisotropy tensor
- porosity and pore-size distribution
- branch-angle distribution
- cross-section evolution spectrum
- minimum cut area
- symmetry spectrum
- spectral shape signature

## 12.2 Flow descriptors

- axial profile of swirl number
- angular momentum flux
- helicity integral and distribution
- vorticity and strain invariants
- Q-criterion structure statistics
- reverse-core position, radius, and stability
- precessing-vortex-core frequency
- residence-time distribution
- short-circuit probability
- recirculating-volume fraction
- pressure recovery
- entropy-generation rate
- wall shear distribution
- droplet shear-exposure distribution
- inlet-to-outlet droplet transfer matrix
- breakup and coalescence event spectra

## 12.3 Structure and manufacturing descriptors

- stress hot-spot distribution
- modal density and margins
- support volume
- overhang-area distribution
- powder-removal path diameter
- tolerance sensitivity
- surface-roughness exposure
- predicted distortion field

Descriptors must contain:

- value
- units
- uncertainty
- source evaluator
- evidence tier
- applicability envelope

---

# 13. Defining alienness without corrupting science

"Alien" should not mean visually strange. It should mean the system has found a geometry or mechanism outside the known design corpus.

Maintain separate novelty channels:

## 13.1 Program novelty

Distance in typed graph structure:

- graph edit distance
- operator n-gram distance
- subgraph novelty
- minimum description length
- developmental-program novelty

## 13.2 Geometric novelty

Distance in:

- topology signature
- spectral shape signature
- curvature distribution
- channel-network statistics
- SDF/voxel embedding

## 13.3 Behavioral novelty

Distance in measured physical behavior:

- swirl profile
- RTD
- pressure recovery
- coherent-structure statistics
- droplet transfer matrix
- shear exposure

## 13.4 Mechanism novelty

A candidate becomes a **novel mechanism candidate** only after:

1. behavior is outside the validated archive,
2. performance survives conservative uncertainty,
3. targeted ablation removes the benefit,
4. the effect repeats under held-out operating conditions.

This separates "strange shape" from "new physics."

## 13.5 Alienness is not promotion fitness

Use novelty to allocate evaluations and preserve diversity. Use hard science to qualify.

---

# 14. Search architecture

## 14.1 Two archives

### Speculative archive

Contains F0/F1 candidates. Optimized for coverage, novelty, and value of information.

### Verified archive

Contains candidates that meet a defined evidence inventory. An F1 estimate cannot displace an F2 result merely because its point estimate is better.

## 14.2 Quality-diversity archive

Do not key the archive by a topology-class string derived from genes.

Use:

- behavior descriptors from fields
- geometry topology signature
- uncertainty-aware bins
- dynamic CVT cells or an unstructured archive
- per-tier elites

## 14.3 Pareto state

Maintain objectives as intervals/distributions:

- conservative common treated capacity
- energy per compliant treated volume
- pressure burden
- robustness/regret
- manufacturing burden
- evidence uncertainty

Complexity may be a soft objective to favor elegant mechanisms, not a hard cap that blocks invention.

## 14.4 Value-of-information promotion

Higher-fidelity evaluation should maximize expected decision value:

- probability of meeting hard gates
- plausible improvement over verified incumbent
- uncertainty reduction
- novelty of mechanism
- evaluation cost
- coverage of an underexplored region

## 14.5 Operator credit

Credit operators for:

- valid novel phenotype production
- archive coverage
- verified improvement
- transfer across scenarios
- successful higher-fidelity promotion

Do not credit them merely for favorable F1 heuristic scores.

## 14.6 Species and extinction protection

Track lineages/species using topology and program embeddings. Protect newly opened niches for a limited age so they are not immediately eliminated by mature conventional designs.

## 14.7 Adversarial scenario coevolution

Within the approved applicability envelope, evolve operating scenarios that maximize failure or comparator regret:

- flow
- viscosity
- density contrast
- interfacial tension
- DSD
- roughness
- manufacturing tolerance
- outlet-control perturbation

A design and its adversarial scenario history become a robustness record.

## 14.8 Local refinement

After macro invention, use local methods:

- CMA-style parameter adaptation
- Bayesian optimization
- adjoint/differentiable refinement where valid
- deterministic coordinate search

Every local step remains in lineage.

## 14.9 LLM and agent proposals

Language models may propose:

- typed graph programs
- rewrite sequences
- mechanism hypotheses
- ablation plans

Each proposal records model identity, prompt hash, tool version, and seed. The model never supplies a scientific score.

---

# 15. Surrogate redesign

## 15.1 Train only on physical observations

The surrogate target must be a result from an evaluator, not another handcrafted gene equation.

## 15.2 Multi-fidelity model

Represent fidelity explicitly:

- F0 geometry
- F0.5 cheap physics
- F1
- F2
- F3
- F4

Learn discrepancy between levels rather than pretending all targets are equivalent.

## 15.3 Inputs

Combine:

- invariant geometry features
- topology signature
- graph embedding
- SDF or mesh embedding
- measured low-fidelity flow descriptors
- scenario descriptors

Do not rely solely on the raw gene vector.

## 15.4 Failure model

Model numerical and geometry failure separately. Failed runs do not enter the regression target as poor performance.

## 15.5 OOD behavior

If a candidate is outside calibration:

- increase uncertainty
- mark OOD
- do not allow surrogate veto
- route a subset to direct evaluation

## 15.6 Calibration

Require:

- leave-one-topology-cluster-out validation
- held-out operating scenarios
- empirical interval coverage
- calibration curves
- error versus novelty distance
- error versus fidelity

## 15.7 No default imputation

A missing structural result is not a nominal stress. Use a model designed for partial evidence or wait for the observation.

---

# 16. Evidence and claim architecture

## 16.1 Scientific identity

Each evaluation should bind:

- genome
- geometry IR
- exact CAD or mesh
- material
- manufacturing state
- requirements
- scenario
- evaluator
- solver build/container
- configuration
- objective definition
- descriptor definition
- extraction code
- dependencies
- seed and stochastic stream

## 16.2 Observation type

```rust
pub enum ObservationState<T> {
    Value(T),
    NotEvaluated,
    Failed(FailureRecord),
    NotApplicable(String),
    OutOfEnvelope(String),
    Superseded(ContentRef),
}

pub struct Observation<T> {
    pub identity: EvidenceIdentity,
    pub state: ObservationState<T>,
    pub uncertainty: Vec<UncertaintyComponent>,
    pub numerical_state: NumericalState,
    pub validation_state: ValidationState,
}
```

## 16.3 Claim inventory

A claim is a frozen list of required criteria. Examples:

- F0 manufacturable subject
- F1 search elite
- F2 carrier-verified elite
- F2 oil-qualified duty
- F4 physical superiority

Missing criteria produce `Incomplete`, never pass.

## 16.4 Review and revocation

Support:

- approved
- reopened
- revoked
- superseded

A later discovery can invalidate a claim without deleting history.

## 16.5 Generated status

Generate:

- README status
- explorer badges
- release tables
- claim summaries

from the evidence registry. CI fails on stale hand-authored status.

---

# 17. Red-team and acceptance test catalogue

## 17.1 Identity tests

- **ADG-ID-001:** one-bit float change changes identity
- **ADG-ID-002:** harmless JSON key ordering does not
- **ADG-ID-003:** graph node-ID permutation canonicalizes
- **ADG-ID-004:** commutative child order canonicalizes
- **ADG-ID-005:** schema migration creates a new identity and receipt
- **ADG-ID-006:** duplicate Unicode-normalized keys reject
- **ADG-ID-007:** evaluator/config/compiler drift invalidates cache

## 17.2 Birth and operator tests

- **ADG-OP-001:** invalid mutation never becomes baseline
- **ADG-OP-002:** every rejection appears in budget ledger
- **ADG-OP-003:** every repair has a bounded transcript
- **ADG-OP-004:** operator RNG consumption is versioned
- **ADG-OP-005:** crossover has no orphan nodes
- **ADG-OP-006:** typed graft cannot cross incompatible ports
- **ADG-OP-007:** phenotype duplicates are detected across distinct genomes
- **ADG-OP-008:** 100-step random walk produces variable graph size
- **ADG-OP-009:** operator failure rate is reported, not hidden

## 17.3 Expressiveness tests

- **ADG-EXP-001:** true branch/rejoin network with measured path count
- **ADG-EXP-002:** non-star-shaped fluid cross-section
- **ADG-EXP-003:** independent internal solid with explicit surface
- **ADG-EXP-004:** resolved bicontinuous TPMS
- **ADG-EXP-005:** asymmetric feed distribution
- **ADG-EXP-006:** nested scale hierarchy
- **ADG-EXP-007:** generic field-generated geometry with no named motif
- **ADG-EXP-008:** same phenotype from two representation paths
- **ADG-EXP-009:** variable number of branches and merges
- **ADG-EXP-010:** at least one valid geometry outside every legacy family classifier

## 17.4 Geometry tests

- **ADG-GEO-001:** B-rep/SDF volume agreement
- **ADG-GEO-002:** surface-area agreement
- **ADG-GEO-003:** Hausdorff agreement
- **ADG-GEO-004:** self-intersection rejection
- **ADG-GEO-005:** non-manifold rejection
- **ADG-GEO-006:** inward-orientation rejection
- **ADG-GEO-007:** trapped-fluid detection
- **ADG-GEO-008:** trapped-powder detection
- **ADG-GEO-009:** near-touching passage detection
- **ADG-GEO-010:** hidden sub-resolution aperture rejection
- **ADG-GEO-011:** exact frozen interface
- **ADG-GEO-012:** unit/rigid-transform invariance
- **ADG-GEO-013:** semantic patch stability
- **ADG-GEO-014:** tolerance perturbation topology stability

## 17.5 Evaluator anti-cheat tests

- **ADG-PHY-001:** changing topology label alone changes no output
- **ADG-PHY-002:** inactive gene changes no physical result
- **ADG-PHY-003:** identical geometry from different genomes gives identical deterministic result
- **ADG-PHY-004:** gene expected-benefit coefficient is inaccessible to evaluator
- **ADG-PHY-005:** mesh coarsening cannot improve qualified fitness
- **ADG-PHY-006:** numerical failure is not mapped to good or bad performance
- **ADG-PHY-007:** mass leakage blocks oil score
- **ADG-PHY-008:** reject-split cheating blocks comparison
- **ADG-PHY-009:** outlet recrossing is not double-counted
- **ADG-PHY-010:** unresolved inventory returns bounds/incomplete
- **ADG-PHY-011:** DSD basis conversion conserves oil volume
- **ADG-PHY-012:** OOD surrogate uncertainty rises
- **ADG-PHY-013:** closure-model disagreement is retained
- **ADG-PHY-014:** exact-baseline special case has no fitness discontinuity

## 17.6 Search tests

- **ADG-SRCH-001:** low-fidelity candidate cannot overwrite higher-tier elite
- **ADG-SRCH-002:** archive cell derives from measured descriptor
- **ADG-SRCH-003:** holdout geometry cannot enter training
- **ADG-SRCH-004:** repeated holdout queries invalidate holdout
- **ADG-SRCH-005:** operator credit uses fresh births only
- **ADG-SRCH-006:** speculative and verified archives remain distinct
- **ADG-SRCH-007:** lineage extinction protection expires deterministically
- **ADG-SRCH-008:** adversarial scenarios remain inside approved envelope

## 17.7 Evidence tests

- **ADG-EV-001:** torn event-log tail recovers valid prefix
- **ADG-EV-002:** missing required observation gives incomplete verdict
- **ADG-EV-003:** artifact bytes must match content reference
- **ADG-EV-004:** physical validation requires specimen identity
- **ADG-EV-005:** synthetic evidence cannot qualify physical claim
- **ADG-EV-006:** revoked review removes publication eligibility
- **ADG-EV-007:** stale generated README fails CI
- **ADG-EV-008:** runtime metadata does not change scientific identity

---

# 18. Initial exit thresholds for ADG-v1

These should be preregistered and adjusted only by versioned policy.

## 18.1 Representation campaign

For a 1,000-proposal corpus:

- at least 95% type-check
- no silent fallback
- every failed proposal categorized
- at least 60% independent F0 validity without repair
- at least 100 distinct structural program hashes
- at least 20 distinct measured topology signatures
- at least 10 valid non-star-shaped phenotypes
- at least 10 valid branch/rejoin phenotypes
- at least 10 valid generic-field phenotypes not produced by a named hydrocyclone macro

The purpose is not to maximize validity at all costs. It is to demonstrate real morphology and topology diversity with transparent failure accounting.

## 18.2 Dual-compiler campaign

Initial targets:

- relative volume disagreement <= 0.2%
- relative surface-area disagreement <= 0.5%
- symmetric Hausdorff distance <= min(0.05 mm, 5% of minimum feature size)
- exact agreement on terminal surfaces
- exact agreement on connected-component count
- exact agreement on declared topology invariants

## 18.3 Search campaign

- archive descriptors are phenotype/field derived
- no search metric is computed directly from a named motif
- all candidate promotion is inventory-based
- surrogate intervals meet preregistered held-out coverage
- OOD error and uncertainty are reported by novelty distance
- operator success and rejection rates are published

---

# 19. File-by-file migration plan

## `crates/joil-core/src/geometry.rs`

- retain as a legacy CT-35 baseline adapter
- move canonical identity to `joil-content`
- remove scientific reliance on `serde_json::to_vec`
- represent dimensions with typed units

## `crates/joil-core/src/program.rs`

- freeze current structures as `legacy_exotic_v2`
- remove baseline fallback from every operator
- rename fixed-field "graph crossover" to macro crossover
- stop erasing schema version
- add migration adapter into `joil-adg-v1`

## `crates/joil-core/src/phenotype.rs`

- split mesh utilities from representation
- move radial `R(theta,z)` compiler into legacy module
- remove hard-coded zero cavity and frozen-difference values
- create independent topology and geometry certificates
- compile true internal surfaces

## `crates/joil-core/src/evolve.rs`

- move hand equations into `joil-eval-rom/legacy_screen_v1`
- label every result screening-only
- prevent invalid F0 candidates from receiving physical metrics
- remove exact-baseline special-case discontinuity
- replace raw arrays/objectives with typed observations
- make archive tier-aware

## `crates/joil-core/src/surrogate.rs`

- remove nominal fallback features
- train on evaluator observations only
- support topology/geometry embeddings and fidelity
- publish calibration and OOD status
- bind model and training set identities

## `crates/joil-core/src/redteam.rs`

- consume independent certificates
- recompute or verify fields rather than trusting mutable phenotype scalars
- add adversarial representation corpus
- distinguish invalid geometry, manufacturing infeasible, numerical failure, and unknown

## `crates/joil-core/src/structure.rs`

- correct the apparent Euclidean-division issue in observed-order calculation
- retain analytical solutions as verification oracles
- add general solid-mesh FEA adapter
- map pressure and torque from actual CFD surfaces
- make material state and environment explicit

## `crates/joil-core/src/experiment.rs`

- preserve the existing protocols
- emit typed evidence observations
- bind specimen passport and as-built geometry
- connect claim inventories to test outcomes

## `crates/joil-core/src/cfd_case.rs`

- consume GeometryIR semantic surfaces
- hash every generated dictionary and mesh
- reject unlabeled or ambiguous surfaces
- preserve feature-level wall groups where useful
- separate case construction from claim qualification

## `docs/WAVE2_PLAN.md`

- distinguish planned graph architecture from implemented macro language
- avoid "trans-dimensional" as an implementation claim until a variable-length graph exists
- add an implementation-status matrix generated from tests

## `README.md` and `docs/STATUS.json`

- generate claim-bearing sections from the evidence registry
- show geometry novelty, evidence tier, and qualification separately

---

# 20. Ordered implementation milestones

## Milestone A — Truth barrier

1. Freeze current representation as legacy.
2. Remove all invalid-to-baseline fallback.
3. Correct schema erasure.
4. Mark current reduced-order physics as screening-only.
5. Replace hard-coded unknown geometry facts with typed unknowns.
6. Correct the structure observed-order calculation.
7. Add operator identity and rejection accounting.

**Exit:** existing runs reproduce, but no current heuristic result can masquerade as independent physical grading.

## Milestone B — Evidence substrate

1. Port/adapt Jarambit content identity.
2. Port/adapt event-sourced lineage.
3. Add typed observations and uncertainty.
4. Add claim inventories and review state.
5. Generate status from evidence.

**Exit:** every candidate and result has complete content-bound lineage.

## Milestone C — GeometryIR and dual compilation

1. Implement units and typed graph ports.
2. Implement GeometryIR.
3. Implement baseline through both compilers.
4. Implement independent topology audit.
5. Implement semantic surfaces.
6. Add dual-compiler comparison certificate.

**Exit:** baseline and legacy motifs compile reproducibly with independent geometry checks.

## Milestone D — Variable topology

1. Implement channel-network graph.
2. Implement generic field language.
3. Implement true TPMS.
4. Implement internal independent solids.
5. Implement structural mutation and typed crossover.
6. Run the representation challenge corpus.

**Exit:** valid subjects exist that cannot be assigned to any legacy family and have measured new topology.

## Milestone E — Neutral physical grading

1. Isolate evaluator processes.
2. Blind evaluators to genome semantics.
3. Compute descriptors from geometry/fields.
4. Replace heuristic selection targets with actual low-fidelity solver observations.
5. Add multi-fidelity evidence and calibration.

**Exit:** changing a gene label without changing geometry cannot change performance.

## Milestone F — Open-ended search

1. Add speculative and verified archives.
2. Add dynamic QD descriptors.
3. Add value-of-information promotion.
4. Add species protection.
5. Add adversarial scenario coevolution.
6. Add LLM proposal channel with strict sandboxing and lineage.

**Exit:** search discovers new measured behavior, not merely new combinations of named motifs.

## Milestone G — Qualification

1. Promote candidates through F2 carrier.
2. Close F2 oil inventory.
3. Execute F3 where needed.
4. Manufacture and CT-scan specimens.
5. Run preregistered F4 paired experiments.

**Exit:** at least one alien mechanism is supported by a complete claim inventory.

---

# 21. Prohibited shortcuts

The following should be explicit engineering prohibitions:

1. No invalid offspring may become the baseline.
2. No topology label may be accepted as topology evidence.
3. No gene-derived descriptor may be used as qualified physical evidence.
4. No missing observation may be replaced by a nominal scalar.
5. No mesh-only shell may be labeled exact B-rep.
6. No F1 candidate may be called physically superior.
7. No holdout may be queried repeatedly and remain a holdout.
8. No solver failure may improve a score.
9. No objective may reward unresolved oil inventory.
10. No schema migration may erase source semantics.
11. No search result may be published without parent/operator lineage.
12. No evaluator cache may survive evaluator, compiler, requirement, or scenario drift.
13. No manufacturing claim may ignore the declared build orientation and as-built tolerance.
14. No visual novelty metric may qualify a scientific invention.

---

# 22. The recommended north-star demonstration

A convincing first demonstration of the redesigned system is not another larger search count. It is a small, undeniable test:

1. Start from the frozen CT-35 interface.
2. Generate a variable-length genome using only generic geometry operators.
3. Produce a design with:
   - a true branch/rejoin network,
   - a non-star-shaped region,
   - an internal field-generated porous structure,
   - no named legacy topology class.
4. Compile it through independent B-rep and SDF paths.
5. Agree on topology and geometry within preregistered tolerances.
6. Pass manufacturing and structural F0 gates.
7. Evaluate it with an evaluator that never receives the genome.
8. Extract behavior descriptors from the resulting fields.
9. Demonstrate novelty relative to the legacy archive.
10. Perform an ablation showing which subgraph causes the behavior.
11. Promote it through the evidence ladder without changing the requirement set.

That would be genuine machine invention: an unfamiliar concept, expressed by a general genome, judged by independent physics, and surrounded by evidence strong enough to survive a hostile review.

---

# 23. Bottom line

JOIL already contains the beginnings of a serious scientific discovery platform. Its next leap is not "more exotic families." It is to stop encoding the answer in the feature menu and the scoring equations.

The current motifs should become seeds and macros. The new genome should be a language.

The current reduced-order formulas should become proposal priors. The evaluator should judge artifacts.

The current topology labels should become measured certificates.

The current campaign summaries should become evidence-resolved claims.

That architecture would let JOIL search beyond human design vocabulary while remaining more skeptical than the human designer. That is the combination required for alien invention graded by hard science.
