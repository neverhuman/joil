# JOIL vNext: Alien Invention Engine
## Engineering Architecture and Migration Specification

**Repositories reviewed**

- `neverhuman/joil`, current `main`, merge commit `d1ac285d7cde6b5c8b0f2f3c9cd20965e90e0ec1`
- `neverhuman/jarambit`, current `main`, commit `c9882733f2d23a69abe82876fc65b597cb4a4e95`

**Purpose**

Transform JOIL from a high-throughput optimizer over a hand-authored hydrocyclone parameter family into an open-ended intelligent design system that can produce genuinely unfamiliar morphologies and mechanisms while preserving strict scientific traceability.

The governing doctrine is:

> **Frozen question. Open design language. Independent physics. Immutable evidence.**

---

# 1. Executive verdict

JOIL has an unusually strong scientific *intent*: frozen interfaces, evidence tiers, uncertainty language, CFD and structural verification plans, deterministic Rust infrastructure, GPU execution, and a quality-diversity search loop. Its documentation often describes the right target architecture.

The current runtime, however, is not yet an alien invention engine. It is a closed-world optimizer over a fixed `DesignProgram` containing inlet ramps, branch-count flags, helical-ribbon parameters, TPMS parameters, wall harmonics, a three-term cone profile, and a skimmer. Eleven family labels are then assigned by hand-written conditional logic.

That design can produce unusual combinations of known motifs, but it cannot invent arbitrary new topologies or mechanisms. More importantly, several active genes are not compiled into the exact geometry evaluated by the highest-fidelity judge. In the current OpenFOAM surface path, a candidate described as a `trifurcated-6jet-vortex-head` is still emitted with two terminal inlet patches. Branch count, TPMS, and ribbon fields materially influence labels and reduced-order equations, but they are not represented as faithful independent internal objects in the CFD surface.

This creates the most serious possible failure mode for intelligent design:

> The generator names a mechanism, the proxy evaluator rewards the name, and the expensive simulation evaluates a different geometry.

JARAMBIT solves a different problem and has an even narrower fixed-family genome, so its representation should not be copied. Its strongest reusable contribution is instead its evidence architecture:

- exact content identity;
- genotype, CAD, mesh, material, and specimen subject identity;
- finite typed observations;
- explicit execution, numerical-verification, and physical-validation states;
- append-only review and revocation;
- constraint-first ranking;
- operator lineage;
- claim resolution against a frozen trusted inventory.

The recommended synthesis is therefore:

> **JOIL domain science + JARAMBIT evidence discipline + a new variable-length typed developmental genome + an independent subject-based scientific judge.**

---

# 2. Current-state comparison

| Dimension | JOIL today | JARAMBIT today | JOIL vNext requirement |
|---|---|---|---|
| Design representation | Fixed hydrocyclone struct with bounded fields | Fixed 128-value C.4-family vector plus domain genes | Variable-length typed graph/AST with graph, field, developmental, and material lanes |
| Topology | Eleven hand-authored family labels | One recognizable product family | Topology observed from compiled geometry, never self-declared |
| Gene-to-geometry fidelity | Partial; several named features are proxy-only | Generally direct for its narrow 2D/extruded object | Every active feature must map to geometry entities and mesh regions |
| Physics independence | Reduced evaluator consumes gene-derived mechanism descriptors | FEM follows phenotype and hard gates more consistently | Judge receives only a compiled subject bundle, never gene labels |
| Evidence identity | Program hashes and manifests exist, but subject/evidence typing is incomplete | Strong content-addressed scientific identity | Generalize JARAMBIT evidence types to program, IR, mesh, field, variant, and specimen |
| Search openness | NSGA-II/MAP-Elites inside a fixed family ontology | Novelty/QD inside a fixed C.4 ontology | Multiple representation species under one judge and one budget ledger |
| Failure handling | Several invalid paths silently fall back to baseline/default values | More explicit rejects and legality | Invalid, failed, unknown, OOD, and repaired are distinct typed states |
| Claim discipline | Documentation is cautious, but some runtime labels and campaign claims overstate enacted geometry | Better separation of screen, promotion, and release state | Qualification is computed from frozen evidence; no mutable status strings |
| Manufacturing | Analytical screens and planned constraints | Detailed object-specific gates | Actual solid topology, process plan, as-built geometry, and uncertainty field |
| Scientific novelty | Mainly parameter novelty and family combinations | Mainly visual/structural variation | Morphological, behavioral, causal-mechanism, and scientific novelty reported separately |

---

# 3. Critical findings in current JOIL

## 3.1 The genome is a catalog, not an open language

The current `DesignProgram` has a fixed set of fields:

- `InletRamp3D`
- `InletSplit3D`
- `HelicalRibbon3D`
- `TpmsInsert3D`
- up to eight `ChamberMode3D` terms
- a three-coefficient `ConeProfile3D`
- `Skimmer3D`

The topology classifier returns one of eleven named families based on which fields are active. Mutation changes those fields and picks branch counts from a small discrete set. “Homologous graph crossover” is actually fixed-field blending and subtree-like swapping among those known fields.

This is useful parametric engineering. It is not open-ended invention.

### Required correction

Replace family selection with a **typed morphogenesis language**. Families become emergent clusters discovered from compiled geometry and behavior. A campaign may still define comparison cohorts, but those cohorts must not restrict what can be expressed.

---

## 3.2 Several semantic genes do not become the geometry sent to the judge

The current phenotype path constructs a ring-sampled wall surface. Branch splitters, ribbons, and TPMS fields affect analytical volume, clearance, labels, or wall modulation, but the candidate OpenFOAM surface remains a five-patch object with:

- `wall`
- `inlet1`
- `inlet2`
- `overflow`
- `underflow`

A branch-count gene of three therefore does not prove that six independently reachable jet paths exist in the simulated fluid domain.

### Required correction

Introduce a **feature enactment contract**. For every active program node, compilation must produce:

1. source program-node ID;
2. canonical IR entity IDs;
3. exact B-rep or implicit-region IDs;
4. mesh face/cell-zone IDs;
5. independent observed measurements;
6. an enactment status.

```rust
pub enum EnactmentState {
    Enacted,
    CanonicalNoOp,
    Shadowed,
    Failed,
}

pub struct FeatureEnactment {
    pub program_node: NodeId,
    pub feature_kind: FeatureKind,
    pub ir_entities: Vec<EntityId>,
    pub mesh_regions: Vec<MeshRegionId>,
    pub measured_effect: Vec<ObservationRef>,
    pub state: EnactmentState,
    pub reason: Option<String>,
}
```

An active node in `Shadowed` or `Failed` state blocks promotion. A `CanonicalNoOp` must disappear from the canonical genotype and hash.

A three-branches-per-port program must pass an observer test proving six reachable jet outlet regions. A TPMS program must produce the intended topology and periodic field. A ribbon program must produce actual solid faces and fluid boundaries.

---

## 3.3 Topology is named by the generator rather than measured by the phenotype

`classify_topology()` reads genes and returns labels such as `trifurcated-6jet-vortex-head`. That is a proposal description, not an observed topological fact.

### Required correction

Derive topology from the final compiled subject:

- connected fluid components;
- connected material components;
- Euler characteristic;
- Betti numbers where practical;
- recovered channel graph;
- source-to-terminal reachability;
- branch and merge counts;
- cycle rank;
- genus;
- terminal count;
- isolated or trapped regions;
- minimum-cut passage dimensions;
- path homotopy classes;
- material-to-interface connectivity.

Store generator annotations separately as untrusted metadata. The judge must never use them.

---

## 3.4 The reduced evaluator rewards assumptions encoded by the genome

Current mechanism descriptors are computed directly from gene values using hand-selected bonuses and penalties. The reduced evaluator then uses those descriptors to calculate pressure, shear, short-circuiting, reverse-core coherence, and separation. Examples include branch-count symmetry gains, TPMS shear bonuses, ribbon bonuses, and hydraulic penalties.

This is circular. A gene named for a beneficial mechanism receives a beneficial descriptor, and that descriptor produces improved performance without an independent physical observation.

### Required correction

Separate three feature classes:

1. **Proposal features**  
   Gene values and program structure. Search operators may use them for locality and health diagnostics.

2. **Observed geometry features**  
   Computed by an independent geometry observer from the compiled subject.

3. **Observed physics features**  
   Extracted from solver fields or measurements by a separate postprocessor.

Only classes 2 and 3 may enter the scientific judge. Proposal features may never directly produce a qualified performance value.

The judge package should not depend on the program crate. Enforce this in the Cargo dependency graph and at the process boundary.

---

## 3.5 Invalid designs silently become the baseline

Mutation, crossover, random sampling, and surrogate feature extraction contain paths that normalize a program and silently substitute `baseline_ct35()` or default numerical values when normalization, phenotype construction, or structural qualification fails.

This is catastrophic for both science and search:

- operator failure is hidden;
- baseline clones inflate apparent validity;
- invalid regions look deceptively safe;
- lineage is falsified;
- novelty and coverage statistics are contaminated;
- search pressure cannot improve bad operators.

### Required correction

Every stage returns a typed outcome:

```rust
pub enum ProposalOutcome {
    Proposed(ProgramRef),
    Invalid {
        original: ProgramRef,
        errors: Vec<ProgramError>,
    },
    Repaired {
        original: ProgramRef,
        repaired: ProgramRef,
        receipt: RepairReceipt,
    },
}
```

No fallback design is permitted. Baseline insertion is allowed only as an explicit operator named `insert_baseline_control`, with its own lineage receipt.

A repair is a real search operator. It must record:

- original hash;
- repaired hash;
- rule IDs;
- parameter or topology displacement;
- reason;
- repair version;
- random seed;
- whether repair changed behavior descriptors;
- repair cost used by selection.

---

## 3.6 The requirements set is not admitted, while the runtime invents policy thresholds

The checked-in requirement set declares a draft/no-campaign-admission state and leaves important pressure, energy, and duty fields unresolved. The evolution code nevertheless imposes thresholds such as a pressure cap relative to baseline and a fixed PDR floor.

### Required correction

Create an executable `CampaignAdmission` gate. Claim-bearing campaigns may run only when:

- requirement set is approved and content-addressed;
- all mandatory limits are non-null;
- scenarios are frozen;
- baseline/comparator identity is frozen;
- objective and constraint recovery methods are frozen;
- evidence tiers are frozen;
- evaluator contexts of use are approved;
- units and coordinate frames are explicit;
- calibration, validation, and audit data splits are frozen.

Exploratory campaigns remain allowed under a `sandbox` state, but their outputs cannot be promoted or described as qualified.

```rust
pub enum CampaignState {
    Sandbox,
    Admitted,
    Suspended,
    Superseded,
    Closed,
}
```

No scientific threshold may live as an unexplained constant in search code. It must come from the admitted requirement/scenario bundle.

---

## 3.7 Structural “qualification” contains non-independent checks

Current structural screening includes valuable analytical oracles, but some checks are constructed to pass:

- mapped reaction force is assigned from the fluid load;
- mapped torque is assigned from the fluid torque;
- their relative error is therefore zero by construction;
- observed grid order uses floating-point `div_euclid` rather than ordinary division;
- the evolutionary constraint violation does not include structural failure, even though the result is stored.

### Required correction

Treat the current module as a **screening proxy**, not qualification.

Implement candidate-specific solid mechanics on the actual solid subject:

- wall, branch vanes, ribbons, lattices, skimmer, collars, and interfaces;
- mapped CFD loads from field artifacts;
- independent surface integration of load;
- conservative interpolation and conservation audit;
- nonlinear/contact/buckling where relevant;
- modal, harmonic, fatigue, and manufacturing residual-stress models where required;
- mesh convergence using correct observed-order mathematics;
- uncertainty and model-form state.

Any mandatory structural failure must enter constrained dominance. A stored-but-ignored failure is scientifically unsafe.

---

## 3.8 CUDA workload size is being confused with independent physical evidence

The candidate CUDA path allocates a large tensor based on available VRAM and labels each tensor cell as a Lagrangian parcel. Candidate-level `d50` and `Cout` are then calculated again from the same closed-form descriptor equations rather than reduced from independent path histories.

Large tensor counts are useful throughput diagnostics. They are not evidence of billions of independently integrated candidate-specific trajectories.

### Required correction

Report separately:

- allocated tensor cells;
- initialized parcel states;
- integration steps actually executed;
- unique trajectories;
- path duration and termination state;
- accepted trajectories after numerical checks;
- trajectory-derived output metrics;
- analytic/proxy-derived metrics.

A result may be called Lagrangian only when the reported performance is actually aggregated from the integrated histories under a frozen carrier field and validated particle model.

CPU/GPU parity proves implementation parity. It does not prove physical correctness.

---

## 3.9 Surrogate validation is not truly out of ontology

The GP surrogate uses features derived from the same gene descriptors and reduced model used to create its targets. “Leave-one-topology-family-out” folds use topology labels assigned by the same hand-written family classifier.

This tests interpolation across named parameter cohorts, not transfer to genuinely new mechanisms.

### Required correction

Surrogate training labels must come from an independent fidelity tier. Surrogates must be validated using:

- held-out graph motifs;
- held-out representation lanes;
- held-out behavior clusters;
- temporal campaign holdouts;
- hidden solver scenarios;
- physical holdouts where available;
- calibration curves and interval coverage;
- rank stability;
- out-of-distribution detection;
- model ensembles or disagreement estimates.

Surrogates may propose promotions. They may never supply final qualified performance.

---

# 4. Governing architecture

JOIL vNext should be divided into five trust zones.

## Zone A — Frozen question

Contains:

- requirements;
- system boundary;
- terminal interfaces;
- materials/process permissions;
- scenarios;
- comparators;
- objectives;
- constraints;
- uncertainty policy;
- evaluator contexts of use;
- promotion policy.

This zone is not evolvable.

## Zone B — Open proposer

Contains:

- evolutionary operators;
- graph grammars;
- developmental encodings;
- topology-optimization proposals;
- generative-model proposals;
- human proposals;
- module reuse;
- repair operators.

This zone may be creative and wrong. It cannot write to Zone A or Zone D.

## Zone C — Deterministic compiler

Converts programs into canonical physical subjects. It may reject. It cannot score performance.

## Zone D — Independent scientific judge

Consumes only compiled subject bundles and scenario bundles. It cannot read topology names, mechanism claims, generator identity, parent fitness, or proposal descriptors.

## Zone E — Immutable evidence and claim resolver

Stores observations, dependencies, review actions, uncertainty, and exact subject identities. It computes qualification from a frozen inventory.

The dependency direction is one-way:

```text
requirements --> compiler contract --> subject bundle --> judge --> evidence --> claim resolver
                       ^
                       |
                    proposer
```

The proposer never imports the judge implementation. The judge never imports the genome implementation.

---

# 5. Genome v4: a typed morphogenesis program

## 5.1 Design goals

The genome must support:

- variable topology;
- variable length;
- hierarchical modules;
- reusable subassemblies;
- exact interfaces;
- emergent combinations;
- field-generated geometry;
- developmental repetition and differentiation;
- multiple materials and process annotations;
- explicit relational constraints;
- safe deterministic compilation;
- mutation locality at several scales;
- cross-representation competition;
- content-addressed canonical identity.

It must not be arbitrary executable code. A Turing-complete CAD script would make termination, determinism, resource control, security, and canonical identity much harder.

Use a **total, typed, bounded, declarative language**.

---

## 5.2 Proposed Rust model

```rust
pub struct DesignProgramV4 {
    pub schema: ContentRef,
    pub campaign: ContentRef,
    pub interface: InterfaceRef,
    pub root: ExprId,
    pub exprs: BTreeMap<ExprId, MorphExpr>,
    pub flow_graphs: BTreeMap<GraphId, FlowGraph>,
    pub fields: BTreeMap<FieldId, FieldExpr>,
    pub materials: BTreeMap<MaterialRegionId, MaterialProgram>,
    pub manufacturing: ManufacturingProgram,
    pub control: Option<ControlProgram>,
    pub budgets: ProgramBudgets,
    pub proposal_annotations: ProposalAnnotations,
}

pub enum MorphExpr {
    Primitive(Primitive),
    Transform {
        child: ExprId,
        transform: Transform3,
    },
    Sweep {
        path: CurveExpr,
        section: SectionExpr,
        scale_law: ScalarLaw,
        twist_law: ScalarLaw,
    },
    Loft {
        stations: Vec<LoftStation>,
        continuity: Continuity,
    },
    Shell {
        child: ExprId,
        thickness: ScalarFieldRef,
    },
    Thicken {
        surface: ExprId,
        thickness: ScalarFieldRef,
    },
    Boolean {
        op: BooleanOp,
        children: Vec<ExprId>,
    },
    Blend {
        children: Vec<ExprId>,
        radius: ScalarFieldRef,
    },
    Repeat {
        child: ExprId,
        group: SymmetryGroup,
        count: CountGene,
    },
    Warp {
        child: ExprId,
        field: VectorFieldRef,
        jacobian_floor: f64,
    },
    Implicit {
        field: FieldId,
        iso_value: Dimensionless,
        domain: BoundingRegion,
    },
    FlowGraph {
        graph: GraphId,
        realization: ChannelRealization,
    },
    Develop {
        grammar: GrammarRef,
        seed: DevelopmentSeed,
        steps: BoundedU32,
    },
    Compose {
        children: Vec<ExprId>,
    },
}
```

Use dimensional types rather than raw `f64`:

```rust
pub struct Quantity<D> {
    pub value: FixedDecimal,
    pub _dimension: PhantomData<D>,
}
```

Geometry-critical dimensions should use fixed-point micrometres or canonical decimals. Dimensionless continuous genes can use a canonical fixed representation. Avoid hash changes caused by textual float formatting.

---

## 5.3 Macro graph layer

A `FlowGraph` expresses system connectivity without preselecting a named mechanism.

```rust
pub struct FlowGraph {
    pub nodes: BTreeMap<FlowNodeId, FlowNode>,
    pub edges: BTreeMap<FlowEdgeId, FlowEdge>,
    pub terminals: BTreeMap<TerminalId, TerminalBinding>,
    pub invariants: Vec<GraphInvariant>,
}

pub enum FlowNode {
    Port,
    Junction {
        policy: JunctionPolicy,
    },
    Branch {
        fanout: BoundedU16,
        allocation: AllocationLaw,
    },
    Merge {
        fanin: BoundedU16,
    },
    SwirlCoupler {
        handedness: Handedness,
        strength: ScalarLaw,
    },
    ExchangeRegion {
        domain_a: DomainTag,
        domain_b: DomainTag,
        interface: ExchangeLaw,
    },
    StorageRegion {
        volume: VolumeLaw,
    },
    Separator {
        geometry_ref: ExprId,
    },
}

pub struct FlowEdge {
    pub from: FlowNodeId,
    pub to: FlowNodeId,
    pub centerline: CurveExpr,
    pub section: SectionExpr,
    pub section_law: SectionLaw,
    pub wall: WallProgram,
    pub semantic_tags: BTreeSet<SemanticTag>,
}
```

This layer can express:

- arbitrary branch/merge networks;
- recirculation loops;
- serial stages;
- counterflow;
- coaxial flow;
- distributed injection;
- split recombination;
- bypass paths;
- manifold networks;
- staged separation;
- dynamic or passive control surfaces.

Graph validity does not imply physical validity. It only guarantees syntax, bounded size, and terminal consistency.

---

## 5.4 Microstructure and field layer

A field lane supports structures too irregular for explicit graph primitives.

```rust
pub enum FieldExpr {
    Constant(Scalar),
    Coordinate(Axis),
    DistanceTo {
        entity: EntitySelector,
    },
    Fourier {
        modes: Vec<FourierMode3>,
    },
    Tpms {
        basis: TpmsBasis,
        wavelength: Vector3<Length>,
        phase: Vector3<Angle>,
        grading: ScalarFieldRef,
    },
    Cppn {
        network: CppnGenome,
        inputs: Vec<FieldInput>,
    },
    ReactionDiffusion {
        parameters: RdParameters,
        steps: BoundedU32,
        seed: FieldSeed,
    },
    Noise {
        basis: NoiseBasis,
        seed: u64,
        bandwidth: FrequencyBand,
    },
    Eigenfield {
        operator: EigenOperator,
        mode_ids: Vec<BoundedU32>,
    },
    Compose {
        op: FieldOp,
        children: Vec<FieldId>,
    },
}
```

The field lane should support:

- gyroid, Schwarz, diamond, and other level sets;
- spatially graded porosity;
- reaction-diffusion patterns;
- CPPN developmental encodings;
- spectral wall fields;
- anisotropic lattices;
- stress-aligned or flow-aligned material;
- locally differentiated repeated modules.

A field is not allowed to create sub-resolution features. The compiler proves feature-size bounds or rejects it.

---

## 5.5 Developmental lane

The highest-value alien designs may require repeated but differentiated structures. Use bounded graph grammars and duplication-divergence.

Example rules:

```text
Branch(x) -> Junction(x) + {Channel(x_i)} for i in 1..k
Channel(x) -> Sweep(path(x), section(x), twist(x))
Module(x) -> Repeat(ModuleChild(x), symmetry(x), count(x))
RepeatChild(i) -> Perturb(base, developmental_coordinate=i/count)
```

Developmental state may include:

- normalized spatial coordinates;
- distance to terminals;
- branch depth;
- path length;
- local module index;
- symmetry orbit;
- inherited scalar fields;
- prior-stage coarse-solver fields referenced by immutable evidence IDs.

Any physics-conditioned growth is a new design iteration, not a mutable evaluation. The final generated geometry is frozen and independently re-evaluated.

---

## 5.6 Relational genes

Alien concepts often require coherent relationships, not independent knobs. Support typed equations such as:

- preserve total inlet area;
- equalize nominal branch resistance;
- phase-lock repeated features;
- set pitch relative to local radius;
- taper thickness with path curvature;
- align lattice direction with an immutable prior stress field;
- maintain a clearance ratio;
- distribute branch area using a simplex.

Relations must be declarative, dimensionally valid, bounded, and acyclic.

---

## 5.7 Representation species

Do not force one encoding to solve every problem. Run multiple species under a shared compiler and judge:

1. **Explicit graph/sweep species**
2. **Implicit SDF species**
3. **Developmental grammar/CPPN species**
4. **Topology-optimization species**
5. **Human-authored species**
6. **Generative-model species**
7. **Module-recombination species**

All species compile into the same canonical IR and receive identical evidence rules.

Cross-species recombination occurs only through typed module interfaces or IR-to-module abstraction, not raw byte crossover.

---

# 6. Canonical Geometry and Physics IR

## 6.1 The IR is the scientific subject boundary

The IR must represent:

- coordinate frames;
- units;
- fluid regions;
- material regions;
- interfaces;
- terminals;
- semantic surfaces;
- adjacency;
- topology;
- local feature provenance;
- manufacturing intent;
- control interfaces;
- expected minimum feature size;
- permitted tolerances.

```rust
pub struct GeometryIr {
    pub schema: ContentRef,
    pub frame: Frame3,
    pub units: UnitSystem,
    pub entities: BTreeMap<EntityId, IrEntity>,
    pub regions: BTreeMap<RegionId, Region>,
    pub interfaces: BTreeMap<InterfaceId, Interface>,
    pub terminals: BTreeMap<TerminalId, Terminal>,
    pub adjacency: RegionAdjacency,
    pub provenance: BTreeMap<EntityId, Vec<ProgramNodeRef>>,
    pub invariants: Vec<IrInvariant>,
}
```

Never treat an STL alone as the design identity. It loses material regions, semantic topology, tolerances, and construction history.

---

## 6.2 Separate identities

A single “design hash” is insufficient. Define:

- `ProgramRef`
- `CanonicalProgramRef`
- `GeometryIrRef`
- `NominalPhenotypeRef`
- `ManufacturingVariantRef`
- `SolidSubjectRef`
- `FluidSubjectRef`
- `MeshRef`
- `SolverCaseRef`
- `FieldArtifactRef`
- `SpecimenRef`
- `MeasurementCampaignRef`

Changing the compiler, kernel, tolerances, mesher, patch map, material state, manufacturing perturbation, or solver configuration creates a new subject or evidence identity.

---

## 6.3 Dual geometry realization

Use at least two independent views:

### Compiler A — exact B-rep/CSG

Produces:

- exact or tolerance-bounded surfaces;
- fluid solid;
- material solid;
- named terminal and interface faces;
- STEP/B-rep artifacts.

### Compiler B — implicit/voxel observer

Produces:

- signed-distance field;
- adaptive octree;
- topology and accessibility estimates;
- independent volume and clearance measures.

### Observer C — mesh-only forensic observer

Reads the exported mesh without access to the genome and recovers:

- components;
- terminals;
- branch graph;
- area and normal;
- passage size;
- cavities;
- non-manifold edges;
- inverted elements;
- material connectivity.

Promotion requires agreement within campaign-defined tolerances. Topology disagreements are always fatal.

---

## 6.4 Mechanism presence proof

Every claimed mechanism must be bound across the chain:

```text
program node
  -> IR entities
  -> B-rep/implicit regions
  -> mesh faces/cells
  -> solver field region
  -> extracted observable
  -> evidence record
  -> claim
```

A label without this chain is decorative metadata.

---

# 7. Variation and repair operators

## 7.1 Operator classes

### Parametric mutation

Small changes to canonical scalar genes.

### Typed subtree mutation

Replace an expression with another expression of the same output type.

### Graph rewrite

- add/remove branch;
- split edge;
- insert merge;
- duplicate module;
- introduce loop;
- graft subgraph;
- prune subgraph;
- reroute terminal;
- change section law.

### Duplication-divergence

Copy a module, then independently perturb geometry, material, or phase.

### Symmetry operators

- impose symmetry;
- increase/decrease group order;
- break one orbit;
- phase shift;
- chirality inversion;
- local symmetry breaking.

### Developmental mutation

Change grammar rule, growth depth, field function, or developmental coordinate coupling.

### Field mutation

Change basis, frequency, phase, composition, grading, or network topology.

### Module grafting

Insert a content-addressed module whose ports and domain types match the host.

### Co-design mutation

Couple geometry, material, and process variables in one atomic operator.

### Representation jump

Abstract an explicit module into a field or instantiate a field into explicit geometry. This is expensive and separately metered.

---

## 7.2 Operator health

For each operator/version, record:

- attempts;
- syntax-pass rate;
- canonicalization-pass rate;
- F0 geometry-pass rate;
- structural-screen pass rate;
- duplicate-phenotype rate;
- no-op rate;
- repair rate;
- median phenotype displacement;
- descriptor displacement;
- novelty gain;
- objective gain at each fidelity;
- numerical-failure rate;
- compute cost;
- lineage survival;
- representation-specific bias.

Operators that produce high invalidity or near-zero phenotype displacement should be adapted, quarantined, or removed. Never hide their failure with a baseline substitution.

---

## 7.3 Repair discipline

Repairs may handle recoverable conditions such as:

- tiny negative clearances caused by tolerance;
- minor terminal misalignment;
- feature-size quantization;
- small self-intersections;
- build-orientation adjustment.

Repairs may not silently:

- change topology;
- remove a major mechanism;
- change branch count;
- replace a candidate with a baseline;
- alter a frozen interface;
- change material class;
- change the requirement set.

Topology-changing correction must be a new explicit proposal.

---

# 8. Search and open-ended invention

## 8.1 Feasibility before performance

Use a strict state lattice:

```text
Invalid proposal
    |
Compiled
    |
F0 feasible / F0 infeasible / F0 unknown
    |
Numerically valid / numerical failure / OOD
    |
Performance evidence
    |
Promotion evidence
```

Novelty never overrides a hard constraint. Unknown never becomes zero.

---

## 8.2 Separate archives

Maintain independent archives:

1. **Morphology archive**  
   Independent geometry descriptors.

2. **Behavior archive**  
   Solver-observed field and system-response descriptors.

3. **Mechanism archive**  
   Designs with causal-ablation evidence.

4. **Robustness archive**  
   Manufacturing/operating envelope elites.

5. **Uncertainty archive**  
   Designs that expose model disagreement or missing knowledge.

6. **Failure archive**  
   Invalid, numerical-failure, OOD, and model-exploitation cases.

7. **Module archive**  
   Reusable typed subassemblies with evidence summaries.

This prevents a single scalar score from collapsing the scientific landscape.

---

## 8.3 Quality-diversity descriptors

### Geometry descriptors

- recovered branch and merge counts;
- cycle rank;
- fluid-region genus;
- material-region genus;
- terminal-to-terminal path-count vector;
- path tortuosity distribution;
- curvature spectrum;
- cross-section entropy;
- anisotropy;
- surface-to-volume ratio;
- minimum feature distribution;
- porosity;
- module repetition and differentiation;
- chirality;
- symmetry group.

### Physics descriptors

Extracted from independent fields:

- swirl-number profile;
- residence-time distribution;
- pressure recovery profile;
- dissipation distribution;
- shear-exposure integrals;
- coherent-structure spectrum;
- recirculation topology;
- short-circuit path fraction;
- particle fate matrix;
- droplet-size transfer matrix;
- wall-load distribution;
- modal participation;
- fatigue hot-spot distribution.

### Causal descriptors

- performance delta under feature ablation;
- rank stability under matched-volume counterfactual;
- mechanism persistence across scenarios;
- model-family agreement;
- physical-transfer discrepancy.

Hand-selected descriptors must be frozen for a claim-bearing campaign. Learned descriptors may be used in exploratory epochs, then frozen before comparison.

---

## 8.4 Novelty is multidimensional

Report four independent concepts:

1. **Genotypic novelty**  
   Useful only for operator diagnostics.

2. **Morphological novelty**  
   Difference in compiled geometry/topology.

3. **Behavioral novelty**  
   Difference in observed physics.

4. **Causal-mechanism novelty**  
   A distinct, reproducible causal pathway established by intervention.

Do not create one weighted “alien score” that can trade scientific credibility against visual strangeness.

---

## 8.5 Open-ended challenge generation

A POET-like lane can co-evolve designs and diagnostic scenarios, but it must not evolve regulatory or owner requirements.

The scenario grammar may vary:

- flow;
- feed distribution;
- fluid properties;
- temperature;
- contamination;
- manufacturing perturbation;
- orientation;
- transients;
- fouling state;
- damage state.

Generated scenarios remain exploratory until reviewed and admitted. Solutions should be transferable across scenario niches, allowing stepping stones that direct optimization would miss.

---

## 8.6 Proposal pools

Run at least these proposal pools concurrently:

- evolutionary graph rewrite;
- developmental field evolution;
- topology optimization;
- novelty-driven search;
- surrogate-assisted search;
- human proposals;
- LLM proposals;
- archive/module recombination.

Every pool receives the same evaluator budget accounting. Generator origin is hidden from the judge.

An LLM may produce only validated DSL programs and mechanism hypotheses. It may not:

- edit requirements;
- assign evidence grades;
- change judge code;
- self-score;
- directly write a qualified observation;
- promote its own output.

---

## 8.7 Exploration against evaluator exploitation

Reserve a fixed audit budget for:

- random candidates;
- low-ranked candidates;
- OOD candidates;
- candidates clustered at parameter limits;
- candidates with unusually large surrogate gains;
- candidates with solver-residual/score correlation;
- candidates with rank reversals across model families.

Create an `EvaluatorExploitAlert` when:

- score improves as convergence worsens;
- mesh distortion correlates with benefit;
- a semantic gene changes score without changing the subject;
- duplicate phenotype hashes receive different scores;
- a candidate is excellent in one model and collapses in siblings;
- benefits disappear under matched-volume or matched-pressure ablation.

---

# 9. Independent scientific judge

## 9.1 Subject-only API

```rust
pub trait Judge {
    fn evaluate(
        &self,
        subject: &SubjectBundle,
        scenario: &ScenarioBundle,
        context: &ContextOfUse,
    ) -> EvaluationReceipt;
}
```

`SubjectBundle` contains no genome labels or proposal annotations.

```rust
pub struct SubjectBundle {
    pub identity: SubjectIdentity,
    pub geometry_ir: ContentRef,
    pub solid_subject: ContentRef,
    pub fluid_subject: ContentRef,
    pub semantic_map: ContentRef,
    pub observer_report: ContentRef,
    pub manufacturing_state: Option<ContentRef>,
}
```

---

## 9.2 Evidence ladder

### F0 — construction and geometry

- compilation;
- topology;
- interfaces;
- wall and passage dimensions;
- material connectivity;
- trapped regions;
- accessibility;
- manufacturing feature size;
- semantic patch integrity.

### F1 — independent reduced physics

- conservation-aware network models;
- analytical bounds;
- low-order flow and structural screens;
- manufactured-solution tests;
- cheap multiphysics models.

F1 is for triage, not publication-level performance.

### F2 — candidate-specific engineering simulation

- actual candidate geometry;
- mesh verification;
- RANS/RSM model family;
- size-resolved Lagrangian transport or validated PBM;
- conservation closure;
- extraction from raw fields;
- numerical uncertainty.

### F3 — model-form challenge

- transient simulation;
- LES or alternate closures;
- alternate breakup/coalescence laws;
- parameter sensitivity;
- model discrepancy;
- blinded hidden scenarios;
- causal ablations.

### F4 — physical validation

- as-built specimen;
- material and process passport;
- metrology;
- paired baseline/candidate testing;
- measurement uncertainty;
- preregistered recovery;
- repeated runs and blocking/randomization.

### F5 — independent replication and transfer

- separate build or laboratory;
- new operating window;
- replicated mechanism;
- transfer to a neighboring design or scale.

---

## 9.3 Typed scientific values

```rust
pub enum ScientificValue<T> {
    Known {
        estimate: T,
        uncertainty: UncertaintyBundle,
        domain: DomainRef,
        observations: Vec<ObservationRef>,
    },
    Interval {
        lower: T,
        upper: T,
        confidence: CoverageInterpretation,
        observations: Vec<ObservationRef>,
    },
    Censored {
        bound: T,
        direction: CensorDirection,
        reason: String,
    },
    NotRun {
        reason: String,
    },
    Failed {
        class: FailureClass,
        reason: String,
    },
    Invalid {
        reason: String,
    },
    OutOfDomain {
        reason: String,
    },
    Indeterminate {
        reason: String,
    },
}
```

Never use:

- `NaN`;
- zero;
- baseline value;
- missing optional value;

as a substitute for unknown or failed evidence.

---

## 9.4 Uncertainty decomposition

Track separately:

- input/scenario uncertainty;
- manufacturing geometry uncertainty;
- material uncertainty;
- numerical discretization;
- iterative convergence;
- temporal sampling;
- stochastic particle sampling;
- model-form discrepancy;
- calibration uncertainty;
- measurement uncertainty;
- data-processing uncertainty.

Dependencies and correlations must be explicit. Do not root-sum-square quantities that are not independent.

---

## 9.5 Probability language

A Monte Carlo pass fraction is a probability only when:

- distributions are empirically or contractually justified;
- dependence is specified;
- sampling is correct;
- the campaign freezes those distributions.

Otherwise call it:

- scenario coverage;
- finite-envelope pass rate;
- sampled sensitivity result.

Use worst-case or interval language when distributions are unknown.

---

## 9.6 Model credibility by context of use

Every evaluator has a model card:

- intended use;
- prohibited uses;
- equations and assumptions;
- parameter sources;
- verification evidence;
- validation evidence;
- applicability domain;
- numerical controls;
- uncertainty method;
- calibration data;
- validation data;
- known failure modes;
- credibility level;
- decision consequence and model influence.

The required credibility rises with the influence of the model and consequence of a wrong decision.

---

# 10. Causal mechanism validation

## 10.1 Mechanism card

A proposer may attach an untrusted hypothesis:

```rust
pub struct MechanismCard {
    pub hypothesis: String,
    pub predicted_observables: Vec<PredictedObservable>,
    pub spatial_regions: Vec<RegionSelector>,
    pub operating_domain: DomainRef,
    pub required_ablation: Vec<AblationSpec>,
    pub failure_signatures: Vec<FailureSignature>,
}
```

The judge is blind to the prose during ordinary evaluation. A separate mechanism-challenge service uses it only to design falsification tests.

---

## 10.2 Mandatory counterfactuals

For a claimed mechanism, generate:

- feature removal;
- feature neutralization;
- phase randomization;
- chirality reversal;
- matched-volume alternative;
- matched-pressure-drop alternative;
- topology-preserving perturbation;
- parameter dose-response;
- alternate model family;
- operating-scenario transfer.

A mechanism claim requires:

- predicted observables in the predicted region;
- consistent intervention effect;
- performance change beyond combined uncertainty;
- survival across model challenges;
- physical confirmation before the highest grade.

---

## 10.3 Alien Evidence Grade

Use a simple maturity label rather than a blended score:

- **A0 — Rendered novelty**  
  A proposal or visualization only.

- **A1 — Enacted novelty**  
  Unique compiled topology/geometry with feature-presence proof.

- **A2 — Behavioral novelty**  
  Independent F1/F2 fields exhibit a distinct behavior.

- **A3 — Causal mechanism candidate**  
  Ablations and model challenges support the proposed causal pathway.

- **A4 — Physically observed mechanism**  
  As-built paired tests observe the effect with uncertainty.

- **A5 — Replicated transferable invention**  
  Independent replication and transfer beyond the original point design.

A spectacular image is A0. A true scientific invention begins at A3 and becomes defensible at A4/A5.

---

# 11. Evidence and identity kernel

Generalize JARAMBIT’s evidence architecture.

## 11.1 Content references

```rust
pub struct ContentRef {
    pub id: String,
    pub revision: String,
    pub sha256: Sha256Digest,
    pub media_type: MediaType,
}
```

A hash is not sufficient by itself. Evidence ingestion must verify:

- bytes;
- semantic type;
- canonical decoding;
- ID;
- revision;
- hash;
- dependencies.

---

## 11.2 Scientific identity

```rust
pub struct ScientificIdentity {
    pub campaign: ContentRef,
    pub requirements: Vec<ContentRef>,
    pub subject: SubjectRef,
    pub compiler: ContentRef,
    pub geometry_kernel: ContentRef,
    pub observer: ContentRef,
    pub mesher: Option<ContentRef>,
    pub evaluator: ContentRef,
    pub solver_build: Option<ContentRef>,
    pub container: Option<ContentRef>,
    pub configuration: ContentRef,
    pub scenario: ContentRef,
    pub material_state: MaterialState,
    pub calibration: Vec<ContentRef>,
    pub dependencies: Vec<ObservationRef>,
}
```

---

## 11.3 Observation identity

Every scalar binds:

- quantity;
- unit;
- coordinate frame;
- region;
- recovery method;
- raw artifact;
- extractor code;
- evidence class;
- execution state;
- numerical state;
- validation state;
- uncertainty;
- dependencies.

---

## 11.4 Review and revocation

Use append-only events:

- approved;
- reopened;
- revoked;
- superseded.

A changed review does not mutate history. It creates a later event. Claim resolution uses the latest valid event under trusted authority.

---

## 11.5 Claim resolver

Qualification is never a mutable boolean written by the candidate or solver. It is resolved from:

- frozen expected inventory;
- required evidence;
- exact subject identity;
- accepted observation states;
- thresholds;
- trusted review receipts;
- revocations;
- campaign state.

---

# 12. Manufacturing and the physical digital thread

## 12.1 Nominal is not as-built

Represent:

1. nominal program;
2. nominal geometry;
3. manufacturing plan;
4. manufacturing realization;
5. measured as-built geometry;
6. physical specimen;
7. test configuration.

Each has a different identity.

---

## 12.2 Manufacturing genome

```rust
pub struct ManufacturingProgram {
    pub process: ProcessRef,
    pub orientation: OrientationGene,
    pub supports: SupportStrategy,
    pub machining: Vec<MachiningOperation>,
    pub powder_removal: PowderRemovalPlan,
    pub heat_treatment: HeatTreatmentPlan,
    pub surface_finish: SurfaceFinishPlan,
    pub inspection: InspectionPlan,
}
```

Process constraints must be evaluated on the actual solid:

- overhang distribution;
- minimum wall;
- local feature size;
- trapped powder;
- drain paths;
- inaccessible support;
- distortion risk;
- residual stress;
- anisotropic material properties;
- roughness-sensitive passages;
- post-machining access;
- cleanability;
- inspection access.

No hardcoded `trapped_cavities = 0`.

---

## 12.3 Robust design

Generate manufacturing variants using:

- measured machine capability;
- tolerance fields;
- roughness;
- shrinkage/distortion;
- material lot variation;
- orientation dependence.

The robust candidate is evaluated on those variant subjects. When distributions are not justified, use a finite adversarial envelope rather than invented probabilities.

---

# 13. Test architecture

## 13.1 Program tests

- parser fuzzing;
- canonical serialization;
- dimensional type errors;
- bounded recursion;
- cyclic relation rejection;
- node-budget enforcement;
- deterministic expansion;
- duplicate ID and Unicode-normalization collision rejection;
- non-finite rejection;
- exact no-op canonicalization.

---

## 13.2 Feature-enactment tests

Examples:

- `branch fanout = 3` at two source ports recovers exactly six reachable terminal jet paths;
- deleting the branch node changes the IR hash and mesh topology;
- TPMS basis produces the expected periodicity and topology;
- ribbon creates actual solid and fluid-boundary entities;
- skimmer length changes the exact fluid subject;
- material-field mutation changes the solid subject and property map;
- every active node maps to at least one IR entity or is removed canonically;
- a gene that affects only a label fails the test.

---

## 13.3 Geometry negative corpus

Include:

- near-touching sheets;
- branch/rejoin networks;
- tiny channels;
- zero-thickness surfaces;
- disconnected material;
- disconnected fluid pockets;
- trapped cavities;
- inverted normals;
- non-manifold junctions;
- topology changes under tolerance;
- coincident surfaces;
- sliver faces;
- open terminal patches;
- duplicated semantic IDs;
- spoofed patch names;
- metre/millimetre errors;
- inaccessible powder regions;
- support islands;
- high-genus lattices.

---

## 13.4 Metamorphic tests

- rigid-transform invariance;
- unit conversion equivalence;
- symmetric permutation equivalence;
- deterministic compilation across machines;
- mesh-refinement convergence;
- no-op program equivalence;
- module-ID renaming invariance;
- orientation changes only the manufacturing subject, not nominal geometry;
- semantically equivalent graph ordering yields the same canonical hash.

---

## 13.5 Judge tests

- manufactured solutions;
- analytical flow/structural oracles;
- conservation negative controls;
- solver failure injection;
- residual plateau detection;
- mesh-quality exploitation cases;
- hidden scenario recovery;
- rank-reversal detection;
- raw-field/extractor tampering;
- uncertainty dependency tests;
- unknown-state propagation;
- claim resolver revocation.

---

## 13.6 Search tests

- invalid offspring never become baseline;
- every child has parent IDs, operator ID/version, and seed;
- budget conservation includes every attempt;
- duplicate phenotype handling;
- operator no-op telemetry;
- archive determinism;
- checkpoint drift rejection;
- learned-descriptor freeze;
- surrogate OOD rejection;
- low-score audit sampling;
- generator-origin blindness.

---

# 14. Metrics for genuine expressiveness

Do not use “number of families” as the principal expressiveness metric.

Measure:

- unique canonical phenotype count;
- unique observed topology count;
- recovered graph motif count;
- topology entropy;
- behavior-cluster count;
- archive coverage;
- active-gene enactment rate;
- dead-gene rate;
- duplicate rate;
- fallback rate, required to be zero;
- operator phenotype displacement distribution;
- local genotype-to-phenotype smoothness;
- long-jump operator reach;
- module reuse and differentiation;
- cross-species transfer;
- causal-mechanism discovery rate;
- F1-to-F2 rank stability;
- F2-to-physical transfer discrepancy;
- uncertainty reduction per expensive evaluation;
- number of A3/A4/A5 inventions.

A campaign with 220 watertight meshes can still have one effective topology. Watertightness is an F0 property, not evidence of creativity.

---

# 15. Immediate P0 changes

These should happen before another claim-bearing campaign.

1. Remove every silent fallback to `baseline_ct35()` or default performance features.
2. Make invalid branch counts and schema mismatches hard errors.
3. Stop accepting one schema/family and serializing it as another.
4. Derive topology from final geometry.
5. Compile branch, ribbon, TPMS, and skimmer mechanisms into the exact solver subject.
6. Add feature-enactment receipts.
7. Relabel the existing `6jet` CFD result according to the geometry actually simulated until six paths are proven.
8. Treat current reduced-order descriptors as proposal heuristics, not scientific observations.
9. Make structural failure part of constrained dominance when structure is mandatory.
10. Fix observed-order calculation using ordinary division, not `div_euclid`.
11. Replace constructed-zero load-transfer checks with independent field integration and mapped-load comparison.
12. Compute trapped cavities, frozen-interface difference, passage sizes, and terminal geometry independently.
13. Replace mutable evidence-tier strings with enums and evidence references.
14. Add parent IDs, operator ID/version, seed, repair receipt, and compiler identity to every individual.
15. Stop calling tensor-cell count an independently integrated parcel count unless the metric is reduced from those histories.
16. Use admitted requirement values; remove unexplained search thresholds.
17. Rename Monte Carlo “probability” outputs unless distributions are justified.
18. Train surrogates only on independent labels and use real OOD holdouts.
19. Make the scientific judge blind to topology labels and generator origin.
20. Add an explicit sandbox/claim-bearing campaign boundary.

---

# 16. Proposed crate layout

```text
crates/
  joil-contract/          # requirements, campaign admission, scenarios
  joil-identity/          # content refs and scientific subject identity
  joil-evidence/          # observations, uncertainty, review, claim resolver
  joil-program/           # typed morphogenesis DSL
  joil-program-canon/     # canonicalization, no semantic repair
  joil-ir/                # canonical geometry/material/physics IR
  joil-compile-graph/     # explicit graph/sweep compiler
  joil-compile-field/     # implicit/developmental compiler
  joil-observer-brep/     # independent exact geometry observer
  joil-observer-sdf/      # independent voxel/SDF observer
  joil-meshing/           # deterministic meshing and semantic mapping
  joil-feature-proof/     # feature enactment and mechanism presence
  joil-search/            # QD, lineage, archives, operator telemetry
  joil-operator-lab/      # offline operator qualification
  joil-judge-api/         # subject-only protocol
  joil-judge-f0/
  joil-judge-f1/
  joil-transfer/          # simulation/physical discrepancy and test selection
adapters/
  openfoam14/
  structural-solver/
  metrology/
  laboratory/
schemas/
  campaign/
  program/
  ir/
  subject/
  evidence/
  review/
  claim/
```

Enforce dependency direction in CI. In particular, `joil-judge-*` must not depend on `joil-program` or `joil-search`.

---

# 17. Migration PR sequence

## PR 0 — Truth and safety repair

- remove fallbacks;
- fix grid-order bug;
- add structural constraint;
- type failure states;
- relabel unsupported mechanism claims;
- add campaign sandbox state;
- add regression tests proving zero fallback.

**Exit gate:** invalid proposals remain invalid; current claims describe the actual subject.

## PR 1 — Identity and evidence kernel

Generalize JARAMBIT’s content identity, observation, uncertainty, review, and claim resolver.

**Exit gate:** no claim can resolve without exact subject/evaluator/scenario evidence.

## PR 2 — Canonical subject bundle

Create `GeometryIr`, separate program/phenotype/mesh identities, and semantic surface maps.

**Exit gate:** same program under different compiler/tolerance/mesher produces a distinct subject identity.

## PR 3 — Independent observers

Add B-rep, SDF, and mesh-only topology/clearance/cavity observers.

**Exit gate:** disagreement blocks promotion; no hardcoded cavity or frozen-interface result.

## PR 4 — Explicit flow-graph genome

Implement variable branch/merge/loop topology and sweep/loft realization.

**Exit gate:** no family enum is needed to create or classify graph topology.

## PR 5 — Field and developmental genome

Implement implicit fields, CPPN, TPMS, reaction-diffusion, and bounded grammar development.

**Exit gate:** at least two representation species compile to the same IR contract.

## PR 6 — Feature-enactment proof

Map each active node through IR, B-rep, mesh, and observed geometry.

**Exit gate:** label-only genes are impossible.

## PR 7 — Search rewrite

Add typed lineage, operator telemetry, separate archives, species, novelty, and strict state handling.

**Exit gate:** every attempt is budget-accounted and every child is reproducible.

## PR 8 — Judge separation

Move evaluators behind a subject-only process/API boundary.

**Exit gate:** the judge cannot read genes, topology labels, or generator identity.

## PR 9 — Multi-fidelity evidence

Bind F1/F2/F3 outputs to raw artifacts, extractors, uncertainty, and context of use.

**Exit gate:** no performance scalar is promotion-eligible without evidence state and uncertainty.

## PR 10 — Causal challenge engine

Add ablations, matched counterfactuals, model-family challenge, and Alien Evidence Grade.

**Exit gate:** mechanism novelty cannot be claimed from morphology alone.

## PR 11 — Manufacturing digital thread

Add process plan, variant subjects, metrology, as-built identity, and specimen passports.

**Exit gate:** nominal and physical subjects cannot be confused.

## PR 12 — Physical transferability loop

Use paired tests to model simulation discrepancy and select the next most informative builds.

**Exit gate:** A4/A5 evidence can be resolved automatically from trusted observations and review.

---

# 18. Definition of done

JOIL becomes an alien invention engine when all of the following are true:

1. There is no canonical list of allowed design families.
2. Programs have variable topology and variable length.
3. Multiple representation species compete under one judge.
4. Every active feature is proven in the compiled geometry and mesh.
5. Topology is observed from the phenotype.
6. The judge sees only exact subject bundles.
7. Proposal descriptors cannot directly create qualified physics.
8. Invalid, failed, unknown, OOD, and repaired states are distinct.
9. No failure path substitutes the baseline.
10. Every candidate has complete deterministic lineage.
11. Every scientific scalar has unit, frame, region, recovery, uncertainty, and evidence identity.
12. Numerical verification and physical validation are distinct.
13. Surrogates cannot issue final qualification.
14. Current and future claims resolve against a frozen trusted inventory.
15. Novel morphology, novel behavior, and novel causal mechanism are reported separately.
16. A3 mechanism candidates survive ablation and model challenge.
17. A4 inventions are observed in as-built paired tests.
18. A5 inventions replicate or transfer.
19. Search telemetry shows continuing growth in observed topology and behavior diversity, not merely parameter variation.
20. The strongest designs beat the baseline by more than combined uncertainty under the admitted duty envelope.

---

# 19. The most important design principle

Do not add more exotic named genes to the current struct.

Adding fields called `metamaterial_vortex`, `fractal_branch`, `gyroid_mode`, or `alien_insert` would only enlarge the catalog. It would not create open-ended invention.

Build a language capable of expressing structures the authors did not name in advance, compile every structure into the exact judged subject, and force every scientific claim through independent evidence.

That is the transition from **generative styling** to **intelligence-designed invention**.
