# Engineering verdict

**JOIL is much closer than Jarambit to an alien-invention engine—but it is not yet an open-ended invention engine.**

JOIL has the stronger multidisciplinary science stack: a staged fidelity ladder, OpenFOAM integration, geometric red-teaming, structural screening, Monte Carlo evaluation, multi-seed comparisons, holdouts, and explicit physical-validation boundaries. Jarambit has the stronger **scientific constitution**: exact identity, requirement-set binding, lineage, hard-versus-soft gate semantics, evidence-generated status, and exceptionally conservative promotion rules.   

Neither repository currently contains a genome capable of truly open-ended alien invention:

- Jarambit deliberately constrains designs to a 128-dimensional product genome and recognizable karambit envelope.
- JOIL’s current genome is a more creative but still closed catalog of human-authored mechanisms: inlet ramp, splitter, ribbon, TPMS choice, Fourier chamber modes, cone modes, and skimmer. Those are classified into exactly eleven named families.  
- JOIL’s documentation already describes the right future direction—channel graphs, implicit geometry, and no fixed vector ceiling—but the actual implementation has not yet realized that architecture.  

The correct synthesis is:

> **JOIL’s multidisciplinary physics + Jarambit’s evidence constitution + a new typed developmental geometry language.**

The single most important architectural rule is:

> **The imagination engine and the scientific judge must be structurally independent.**

At present, they are not.

---

# 1. JOIL versus Jarambit

| Dimension | JOIL today | Jarambit today | Required JOIL v4 |
|---|---|---|---|
| Genome | Named engineering feature structs | Fixed 128-value product vector | Typed graph + implicit fields + bounded developmental program |
| Topology | Eleven hand-classified families | One intentionally anchored product family | Unbounded combinations within explicit resource budgets |
| Continuous shape | Fourier modes, cone modes, bounded scalar fields | C² spline silhouette and structural genes | Spline/RBF/Fourier/CPPN fields composed through a general field DAG |
| Structural mutation | Feature toggles and bounded branch-count changes | Operator-aware blocks | Typed insert/delete/split/merge/rewrite/subgraph transplantation |
| Crossover | Layer copying and scalar blending called graph crossover | Block-wise crossover | True homologous typed-subgraph crossover |
| Phenotype | Primarily star-shaped radial surface plus analytic feature accounting | Product-specific CAD/FEM phenotype | Multi-region fluid/solid/porous geometry with first-class internal topology |
| Fast evaluator | Hand-derived formulas using genome-derived mechanism descriptors | Phenotype-derived structural screens | Generic phenotype-only physics, forbidden from seeing the genome |
| Quality diversity | Fixed family and hand-designed descriptor bins | Novelty/QD over a fixed product space | Adaptive morphology and behavior archives based on measured outputs |
| Requirements | Strong draft documents, but values are duplicated in code | Versioned requirement sets with hard and soft semantics | Executable, content-addressed requirements as sole authority |
| Identity | Genome hash, baseline hash, checkpoint/evaluator hashes | Full `ScientificIdentity` over subject, requirements, code, config, evaluator, scenario, material, dependencies | Jarambit identity model extended to CAD, fields, meshes, solvers, and specimens |
| Failure handling | Some mutation failures become the CT-35 baseline | Explicit gate and evidence states | Every failure retained as structured evidence; never silent repair |
| Promotion | Strong intended F0–F4 ladder, but implementation drift exists | Deliberately conservative and “no auto-promotion” | State-machine promotion with CI-enforced evidence admission |
| Alienness | Exotic combinations within a human vocabulary | Not an objective | Topological, morphological, developmental, and mechanism novelty measured separately |

Jarambit’s genome should **not** be copied. Its 128-dimensional representation is intentionally optimized around a known object archetype. What should be copied almost verbatim is its governance:

1. exact scientific identity;
2. requirement-set versioning;
3. parent and operator lineage;
4. hard versus soft gates;
5. screen versus promotion distinction;
6. evidence-generated public status;
7. full-precision serialization.

Jarambit explicitly records that four-decimal serialization changed a certified design’s result materially, and therefore preserves full round-trip precision. JOIL currently rounds many genome parameters to four decimals during normalization and mutation. That may be a reasonable *declared quantization*, but it must be represented as such rather than treated as exact scientific identity. 

---

# 2. Critical blockers in current JOIL

## 2.1 The genome is a feature menu, not a design language

The actual `DesignProgram` contains:

- `InletRamp3D`
- `InletSplit3D`
- `HelicalRibbon3D`
- `TpmsInsert3D`
- a vector of `ChamberMode3D`
- `ConeProfile3D`
- `Skimmer3D`

The topology classifier then maps combinations of those fields into eleven canonical families. A cylindrical gyroid is one family, Schwarz-P and Diamond are collapsed into another, four branches produce an octa-branch family, and particular ribbon/skimmer combinations produce a counter-fluted family.  

This supports **combinatorial novelty inside a human-authored ontology**, but it cannot invent:

- an unexpected nested flow network;
- asymmetric branch/rejoin topologies;
- multiple interacting vortex chambers;
- a fluidic oscillator;
- a recursively bifurcating separator;
- an internal compliant structure;
- an anisotropic porous gradient not expressible as one of four TPMS enums;
- a geometry whose operating mechanism has no existing JOIL feature name.

Adding a twelfth or fiftieth family would not solve this. It would enlarge the catalog without changing its ontological closure.

### Required correction

Remove `topology_class` from the genotype. It may remain as a post hoc reporting tag, but search, crossover, validation, and promotion must not depend on it.

The existing eleven families should become:

- legacy seed recipes;
- test fixtures;
- human-readable annotations;
- regression cases.

They should not define the boundary of possibility.

---

## 2.2 The implemented CFD phenotype does not faithfully contain the claimed internal topology

This is the most serious source-level finding.

`Phenotype3D::from_program` accounts for splitter vanes, ribbons, and TPMS largely through:

- minimum-clearance calculations;
- minimum-passage calculations;
- analytic subtraction from fluid volume;
- analytic addition to solid mass.

For example, ribbon volume is calculated as vane count × span × chord × thickness, while TPMS volume is approximated using annular volume and an estimated solid fraction. 

The actual `MultiPatchSurface` is then constructed primarily as axial rings governed by a radial function and exported as a single five-patch STL. `build_openfoam_candidate_case` writes that one surface into OpenFOAM as the wall, two terminal inlets, overflow, and underflow. It does not emit independently resolved internal splitter walls, guide-ribbon surfaces, or porous-region cell zones corresponding to the claimed internal mechanisms.  

This means:

> A watertight manifold result currently proves that the compiled shell is closed. It does not prove that the intended six-jet manifold, guide-ribbon cascade, or TPMS domain is present in the CFD geometry.

The existing OpenFOAM pressure-drop results may still be valid for the actual compiled shell. They do **not**, by themselves, establish that the named internal feature caused the result.

### Required correction

Every physically claimed feature must compile into one of the following:

1. a resolved fluid-domain boundary;
2. a resolved solid-domain boundary;
3. a typed porous `cellZone`;
4. a typed internal `faceZone`;
5. a validated homogenized material law with an explicit applicability scale.

A six-jet design must have six actual internal jet passages. A ribbon must exist as a guide surface. A TPMS insert must either be geometrically resolved or represented by a separately validated porous constitutive model. Visual centerlines and volume arithmetic are not sufficient.

---

## 2.3 The fast evaluator is self-fulfilling

JOIL’s fast physics is derived from hand-authored `MechanismDescriptors`. Those descriptors themselves encode assumptions such as:

- a TPMS insert multiplying the shear-mitigation factor by a favorable constant;
- a ribbon multiplying it by another favorable constant;
- hand-computed swirl bonuses;
- fixed TPMS drag penalties by basis;
- fixed ribbon recovery bonuses;
- hardcoded hydraulic resistance effects. 

The reduced-order evaluator then uses those same descriptors to compute pressure, shear, breakup, reverse-core coherence, grade efficiency, and treated concentration. 

That creates a closed causal loop:

\[
\text{gene says “TPMS”}
\rightarrow
\text{descriptor assumes lower shear}
\rightarrow
\text{evaluator predicts better droplets}
\rightarrow
\text{evolution selects TPMS}.
\]

The system can discover the best way to exploit its author’s assumptions. It cannot discover that the assumption is wrong, or that an unnamed mechanism is better.

### Required correction: evaluator blindness

At the package level, enforce this dependency rule:

```text
joil-genome ───────┐
                   ├──> joil-compiler ──> joil-phenotype
joil-ir ───────────┘                          │
                                             ▼
                                      joil-evaluator
```

`joil-evaluator` must be unable to depend on:

- `joil-genome`;
- mutation operators;
- family labels;
- TPMS enum names;
- proposal provenance;
- “expected mechanism” descriptors.

It should receive only:

- geometry;
- materials;
- boundary conditions;
- operating scenario;
- mesh;
- solver configuration.

A mandatory architectural test should fail CI if an evaluator crate imports a genome or search crate.

The corresponding scientific invariant is:

> Two different genomes that compile to the same phenotype must receive the same evaluation, within declared numerical reproducibility bounds.

---

## 2.4 Invalid offspring silently become the baseline

Multiple mutation and crossover functions end with:

```rust
c.normalize().unwrap_or_else(|_| Self::baseline_ct35())
```

This occurs in TPMS mutation, graph crossover, general mutation, conventional mutation, random sampling, and family-stratified sampling. 

This is extremely damaging to evolutionary science:

- invalid operators appear successful;
- topology failure rates are hidden;
- diversity is artificially collapsed;
- the CT-35 baseline can appear repeatedly under unrelated lineage;
- operator statistics become false;
- archive occupancy can be distorted;
- the search never learns which edits are infeasible;
- failure examples cannot train a constraint model.

### Required correction

Variation must return an explicit result:

```rust
pub enum OffspringAttempt {
    Compiled {
        genome: GenomeV4,
        lineage: LineageEvent,
    },
    Rejected {
        proposed_genome_hash: Digest,
        lineage: LineageEvent,
        violations: Vec<Violation>,
        witness: Option<FailureWitness>,
    },
}
```

There must be **zero silent baseline fallbacks**.

An optional repair operator may run, but it must produce:

- pre-repair genome hash;
- repaired genome hash;
- list of geometric changes;
- phenotype distance moved;
- requirement that triggered each change;
- repair operator version.

Repair is a variation operator, not normalization.

---

## 2.5 Schema v3 is normalized back into v2

`normalize()` accepts schema versions 2 and 3 and families v2 and v3, but constructs the normalized result with:

```rust
schema_version: 2,
family: "ct35-exotic-head-v2"
```

It therefore destroys the input schema identity. 

This creates ambiguity around:

- which representation was actually evaluated;
- whether v3 semantics exist;
- whether hashes distinguish v2 and v3;
- whether results can be reproduced after migration.

### Required correction

A schema version must be immutable after parsing. Migration must be explicit:

```rust
pub trait GenomeMigration {
    const SOURCE: SchemaId;
    const TARGET: SchemaId;

    fn migrate(
        source: &CanonicalGenome,
    ) -> Result<MigrationReceipt, MigrationError>;
}
```

The receipt must bind:

- source hash;
- migration implementation hash;
- target hash;
- semantic-difference report;
- any information loss.

Do not let `normalize()` perform migration.

---

## 2.6 The preregistered objective contract and executable objective differ

The preregistration declares five objectives:

1. treated concentration;
2. specific energy;
3. one minus 20 µm grade capture;
4. cut size;
5. pressure drop. 

The core evolutionary `Individual` objective vector instead uses:

1. treated concentration;
2. pressure drop;
3. negative grade capture;
4. entry shear;
5. short-circuit fraction. 

This is not a harmless presentation difference. It changes Pareto dominance, parent selection, archive composition, and promoted elites.

### Required correction

There must be no hand-authored objective array in `evolve.rs`.

A frozen campaign manifest should be loaded into a typed runtime contract:

```rust
pub struct ObjectiveSpec {
    pub id: ObservableId,
    pub sense: OptimizationSense,
    pub units: UnitId,
    pub transform: ObjectiveTransform,
    pub reference_point: Option<f64>,
}

pub struct CampaignContract {
    pub requirements: ContentRef,
    pub objectives: Vec<ObjectiveSpec>,
    pub constraints: Vec<ConstraintSpec>,
    pub scenarios: Vec<ScenarioRef>,
    pub statistics: StatisticalProtocol,
}
```

CI must verify that:

- objective IDs match;
- order matches;
- units match;
- optimization sense matches;
- transforms match;
- reference points match;
- scenario distributions match.

Any drift creates a new campaign ID. It cannot silently reuse previous results.

---

## 2.7 The current “hypervolume” is not Pareto hypervolume

`compute_normalized_hypervolume` evaluates only three dimensions, finds the largest single axis-aligned box contributed by any one candidate, and adds a diversity bonus based on archive size. It does not calculate the union volume dominated by the Pareto front, and it does not use the five preregistered objectives. 

Consequently, a search arm can be ranked differently from how a correct hypervolume implementation would rank it.

The bootstrap protocol also declares 10,000 resamples, while the implementation runs 2,000. The implementation is a percentile bootstrap, not the BCa procedure described in the Wave-2 plan.  

The `RandomLhsExotic3d` comparator is also not a Latin hypercube implementation: each candidate independently samples scalar ranges from the RNG. 

### Required correction

Before additional claims are generated:

- implement a validated five-dimensional Pareto hypervolume;
- test it against known analytic fronts and a second independent implementation;
- remove diversity bonuses from hypervolume;
- report QD score and archive coverage separately;
- implement actual LHS or rename the comparator;
- execute the exact preregistered bootstrap count;
- implement BCa if BCa is claimed;
- retain per-seed empirical attainment surfaces;
- compare equal computational cost as well as equal candidate count.

---

## 2.8 The requirement set says the campaign is not admitted

`RS-joil-research-v1` has status `draft_no_campaign_admission` and lists unresolved dependencies including pressure-flow behavior, vessel geometry, actual fluid properties, AM material/process data, breakup regime, holdout evidence, and operator-specific requirements. 

That honesty is excellent. The implementation should enforce it.

### Required correction

A requirement set with a non-admitted status must be able to produce:

- exploratory screens;
- diagnostic results;
- engineering-development artifacts.

It must not produce a state named `Promoted`, `Qualified`, or `Validated`.

Use terms such as:

```text
GENERATED
F0_VALID
F1_SCREENED
CFD_DIAGNOSTIC
NUMERICALLY_VERIFIED
PHYSICALLY_VALIDATED
INDEPENDENTLY_REPLICATED
```

Only an admitted requirement set can enter the latter states.

---

## 2.9 Fixed family labels make the QD system anti-alien

The current MAP-Elites key is based on topology-family strings and bins of hand-authored mechanism descriptors. This means the archive can only illuminate dimensions already anticipated by the engineers.

MAP-Elites is valuable precisely because it maps diverse high-performing behaviors—but the dimensions of variation determine what the archive can reveal. [arXiv](https://arxiv.org/abs/1504.04909?utm_source=chatgpt.com)

### Required correction

Use three distinct archives:

1. **Morphology archive**  
   Descriptors derived from actual geometry and topology.

2. **Mechanism archive**  
   Descriptors measured from flow, droplet, thermal, structural, or other physical fields.

3. **Robust-performance archive**  
   Pareto-efficient candidates under requirement and uncertainty constraints.

Family names may be attached afterward for readability. They must not define archive cells.

---

## 2.10 The current structural evaluator is a valuable screen, not arbitrary-topology FEA

The existing structure module includes Lamé-cylinder calculations, lobe/cone stress factors, vane and skimmer beam approximations, modal ratios, and LPBF checks. This is useful F0 engineering screening. It is not a topology-general structural solver for arbitrary internal networks, lattices, shells, or load paths. 

### Required correction

Retain the current evaluator as `F0StructuralScreen`, but add a generic promotion path:

1. solid mesh generated from the exact compiled solid;
2. spatial pressure and wall-shear loads mapped from CFD;
3. linear elastic solve;
4. buckling solve where appropriate;
5. modal solve;
6. fatigue screening using spatial stress histories;
7. mesh-convergence assessment;
8. AM anisotropy and defect sensitivity;
9. as-built CT geometry reassessment.

No feature-specific stress multiplier should be required for a newly invented topology.

---

# 3. JOIL Genome v4: Typed Developmental Field Graph

I recommend naming the new representation:

## **TDFG — Typed Developmental Field Graph**

It combines four complementary representations:

1. **A typed module graph** for connectivity and true topology.
2. **An implicit field DAG** for arbitrary freeform geometry.
3. **A bounded developmental rewrite program** for repetition, symmetry, hierarchy, and growth.
4. **An immutable interface contract** for drop-in compatibility.

A pure parameter vector cannot express open topology. A pure implicit neural field is difficult to inspect, mutate structurally, and manufacture. A pure graph cannot express smooth organic surfaces. A pure CAD operation history becomes brittle under evolution.

The hybrid is the key.

Graph grammars already provide a practical precedent for synthesizing network topologies, while developmental encodings such as CPPNs compactly express symmetry, repetition, regularity, and repetition-with-variation. [arXiv](https://arxiv.org/html/2306.07778v2?utm_source=chatgpt.com)

---

## 3.1 Proposed top-level schema

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenomeV4 {
    pub schema: ContentRef,
    pub interface_contract: ContentRef,

    /// Root of the developmental/module program.
    pub root: ModuleId,

    /// Arena-based structures avoid recursive serialization ambiguity.
    pub modules: BTreeMap<ModuleId, ModuleNode>,
    pub fields: BTreeMap<FieldId, FieldExpr>,
    pub rules: BTreeMap<RuleId, RewriteRule>,

    pub material_regions: Vec<MaterialAssignment>,
    pub manufacturing_intent: ManufacturingIntent,

    /// Limits growth without fixing the resulting topology.
    pub expansion_budget: ExpansionBudget,

    /// Declared numerical precision and quantization.
    pub numeric_contract: NumericContract,
}
```

The genome should not include:

- a topology-family label;
- a predicted performance score;
- an assumed shear benefit;
- an assumed pressure-recovery bonus;
- a mechanism descriptor used by the judge;
- solver results;
- requirement thresholds.

Those belong elsewhere.

---

## 3.2 Module graph

```rust
pub enum ModuleNode {
    FixedInterfacePort(FixedPortRef),

    Channel(ChannelSpec),
    Junction(JunctionSpec),
    Chamber(ChamberSpec),
    Collector(CollectorSpec),

    GuideSurface(GuideSurfaceSpec),
    PorousRegion(PorousRegionSpec),
    SolidSupport(SolidSupportSpec),

    GenericFluidRegion {
        field: FieldId,
    },
    GenericSolidRegion {
        field: FieldId,
        material: MaterialRef,
    },

    Transform {
        child: ModuleId,
        transform: TransformSpec,
    },
    Repeat {
        child: ModuleId,
        group: SymmetryGroup,
        variation_field: Option<FieldId>,
    },

    RewriteSite {
        nonterminal: NonterminalId,
        ports: Vec<TypedPort>,
    },
}
```

The named modules are ergonomic macros. Every module must lower to a small generic IR:

- material field;
- void/fluid field;
- boundary semantics;
- typed ports;
- region labels;
- continuity constraints.

This prevents named modules from becoming another closed catalog. A future “fluidic oscillator” module would be a convenience macro, not a new fundamental ontology.

---

## 3.3 Typed ports

Each module connection should carry more than a point:

```rust
pub struct TypedPort {
    pub id: PortId,
    pub domain: PortDomain,
    pub frame: RigidFrame,
    pub section: SectionRef,
    pub normal: Direction3,
    pub continuity: ContinuityClass,
    pub flow_role: Option<FlowRole>,
    pub conservation: ConservationContract,
    pub clearance_envelope: FieldId,
}
```

`PortDomain` should distinguish:

- fluid;
- solid;
- porous interface;
- thermal;
- mechanical support;
- manufacturing access;
- measurement or instrumentation.

The compiler should reject connections that violate:

- cross-sectional compatibility;
- orientation;
- continuity;
- conservation;
- material compatibility;
- minimum ligaments;
- manufacturing access.

This creates **validity by composition** rather than relying only on repair afterward.

---

## 3.4 Implicit field language

Use an arena-based expression DAG:

```rust
pub enum FieldExpr {
    Constant(f64),
    Coordinate(Axis),
    Parameter(ParameterRef),

    Sphere(SphereSpec),
    Cylinder(CylinderSpec),
    Superquadric(SuperquadricSpec),

    Sweep {
        centerline: CurveRef,
        section: SectionRef,
        frame: SweepFrame,
    },
    Loft {
        sections: Vec<LocatedSection>,
    },

    Union(Vec<FieldId>),
    Intersection(Vec<FieldId>),
    Difference {
        left: FieldId,
        right: FieldId,
    },
    SmoothUnion {
        children: Vec<FieldId>,
        radius: Quantity<Length>,
    },

    Transform {
        child: FieldId,
        transform: TransformSpec,
    },
    Warp {
        child: FieldId,
        displacement: VectorFieldRef,
    },

    FourierField(FourierFieldSpec),
    RbfField(RbfFieldSpec),
    BSplineField(BSplineFieldSpec),
    TpmsField(TpmsFieldSpec),
    BandLimitedNoise(BandLimitedNoiseSpec),

    /// Small deterministic developmental field network.
    PatternNetwork(PatternNetworkSpec),

    Clip {
        child: FieldId,
        envelope: FieldId,
    },
}
```

The critical escape hatches are:

- generic field composition;
- coordinate warps;
- distance-to-skeleton fields;
- band-limited pattern networks;
- graph-generated skeletons.

These permit shapes that no engineer explicitly named.

### Pattern networks

A small CPPN-like network can map spatial and contextual coordinates to:

- wall displacement;
- local porosity;
- thickness;
- curvature bias;
- repetition phase;
- anisotropy direction.

Inputs might include:

\[
x,\ y,\ z,\ r,\ \theta,\ d_{\text{wall}},\ d_{\text{skeleton}},
\text{module depth},\text{flow direction}.
\]

Outputs must be bounded and dimensionally typed.

Do not allow arbitrary executable code. Use:

- a frozen operation set;
- deterministic math;
- bounded node and edge counts;
- bounded Lipschitz constants;
- operation-count limits;
- no filesystem or network access.

---

## 3.5 Developmental rewrite rules

```rust
pub struct RewriteRule {
    pub lhs: GraphPattern,
    pub rhs: GraphReplacement,
    pub guards: Vec<RewriteGuard>,
    pub parameter_program: ParameterProgram,
    pub max_applications: u32,
    pub priority: i32,
}
```

Useful generic productions include:

```text
channel       → branch(k) → child channels
channel       → channel + guide surface
channel       → channel + periodic wall field
chamber       → chamber + nested chamber
junction      → branch + rejoin
region        → porous shell around region
module        → duplicate + transform + parameter divergence
symmetric set → break selected symmetry
surface       → locally refine field basis
```

The expansion process must be:

- deterministic for a given genome and random-stream reference;
- bounded by depth, module count, field-node count, and geometric resolution;
- fully recorded in an expansion receipt.

This permits complexity growth without an arbitrary fixed dimension.

---

## 3.6 Complexity must be budgeted, not prohibited

```rust
pub struct ExpansionBudget {
    pub max_modules: u32,
    pub max_connections: u32,
    pub max_rule_applications: u32,
    pub max_field_nodes: u32,
    pub max_pattern_network_nodes: u32,
    pub max_compilation_octree_cells: u64,
}
```

These are computational resource bounds, not topology classes.

The search should apply a mild minimum-description-length pressure only after feasibility and performance. Complexity should be allowed when it purchases:

- performance;
- robustness;
- manufacturability;
- mechanism novelty;
- evolvability.

---

# 4. Canonical identity and scientific provenance

Jarambit’s `ScientificIdentity` is the strongest architectural element in either repository. It binds the subject to requirements, source, build, dependency lock, configuration, evaluator, scenario, objective, descriptor, material, and other content-addressed dependencies. 

JOIL should extend this model.

## 4.1 Separate identities at every transformation

```rust
pub struct CandidateIdentityV2 {
    pub genome: ContentRef,
    pub expansion_program: ContentRef,
    pub expanded_ir: ContentRef,

    pub compiler: ContentRef,
    pub fluid_field: ContentRef,
    pub solid_field: ContentRef,
    pub cad_model: ContentRef,

    pub surface_mesh: Option<ContentRef>,
    pub volume_mesh: Option<ContentRef>,

    pub requirement_sets: Vec<ContentRef>,
    pub scenario: ContentRef,
    pub evaluator: ContentRef,
    pub solver: Option<ContentRef>,
    pub solver_config: Option<ContentRef>,

    pub material_model: ContentRef,
    pub manufacturing_model: ContentRef,

    pub objective_contract: ContentRef,
    pub descriptor_contract: ContentRef,
    pub random_stream: ContentRef,

    pub source_tree: ContentRef,
    pub build: ContentRef,
    pub dependency_lock: ContentRef,
    pub container_or_environment: ContentRef,
}
```

This distinguishes:

1. what was proposed;
2. what it expanded into;
3. what geometry was compiled;
4. what mesh was solved;
5. what numerical model was used;
6. what as-built specimen was tested.

A genome hash alone cannot establish these equivalences.

---

## 4.2 Canonical numerical encoding

Choose one of two policies per field:

### Exact binary policy

Serialize IEEE-754 bits:

```rust
pub struct ExactF64(u64);
```

### Declared fixed-point policy

```rust
pub struct QuantizedLength {
    pub integer_units: i64,
    pub quantum_nm: u64,
}
```

Do not silently round arbitrary `f64`s inside normalization.

The `NumericContract` must declare:

- storage representation;
- quantization;
- rounding mode;
- transcendental math implementation;
- CPU/GPU reproducibility expectation;
- acceptable numerical equivalence tolerance.

---

## 4.3 Graph canonicalization

To prevent semantically identical graphs from receiving different identities:

1. canonicalize typed node and edge labels;
2. normalize rigid transforms;
3. sort commutative field operands by child digest;
4. remove unreachable nodes;
5. remove semantically inactive parameters;
6. canonicalize graph automorphisms;
7. distinguish genotype identity from phenotype identity.

Store both:

```text
genotype_hash
expanded_ir_hash
phenotype_hash
analysis_geometry_hash
```

Two genotypes may legitimately produce the same phenotype. That is useful information about redundancy and evolvability, but it must not create duplicate solver work.

---

# 5. Phenotype compiler specification

The compiler is the heart of the redesign. It should be treated as a scientific instrument, not merely a mesh exporter.

## 5.1 Compilation stages

```text
1. Parse and canonicalize
2. Validate schema and units
3. Expand developmental rules
4. Type-check graph and ports
5. Compile modules to material/void fields
6. Enforce immutable external interfaces
7. Perform conservative geometric analysis
8. Generate exact or high-accuracy boundary representation
9. Generate fluid and solid domains
10. Generate boundary patches, face zones, and cell zones
11. Generate analysis meshes
12. Run cross-representation equivalence tests
13. Emit a signed compilation receipt
```

Each stage must return structured diagnostics rather than a string error.

---

## 5.2 Multi-region geometry

The compiler must represent at least:

```rust
pub enum RegionClass {
    Fluid(FluidRegionId),
    Solid(MaterialRef),
    Porous(PorousModelRef),
    Void,
    ManufacturingSupport,
}
```

A candidate can then contain:

- several communicating fluid regions;
- internal walls;
- porous interfaces;
- multiple materials;
- nested voids;
- removable manufacturing supports;
- machining allowances.

The final CFD representation must include dynamic semantics:

```text
terminal inlet patches
terminal outlet patches
external walls
internal guide walls
moving interfaces, if any
porous cell zones
measurement planes
periodic interfaces
fluid-fluid interfaces
```

The current assumption of exactly five patches should be replaced with:

- five immutable **terminal interface roles**;
- any number of internal compiler-generated regions and patches.

---

## 5.3 TPMS handling

A TPMS feature needs two scientifically distinct compilation modes.

### Resolved mode

The actual sheet or solid is represented and meshed. Requirements include:

- minimum sheet thickness;
- minimum pore diameter;
- surface curvature bounds;
- fluid connectivity;
- solid connectivity;
- powder evacuation;
- mesh resolution relative to pore scale.

### Homogenized mode

The TPMS becomes a porous cell zone with:

- permeability tensor;
- inertial resistance tensor;
- orientation field;
- porosity field;
- calibration evidence;
- scale-separation applicability limit.

The permeability and inertial coefficients must come from a separately verified unit-cell study or experiment. They cannot be inferred from the enum name.

---

## 5.4 Conservative geometric gates

Use adaptive spatial subdivision with conservative bounds for:

- minimum wall thickness;
- minimum passage;
- interface clearance;
- self-intersection;
- Jacobian sign;
- trapped fluid cavities;
- trapped powder cavities;
- unreachable supports;
- disconnected solids;
- disconnected fluid paths;
- curvature;
- surface normal discontinuity;
- feature scale.

Sampling alone can miss narrow violations. Interval bounds or conservative Lipschitz estimates should be used where possible.

---

## 5.5 Cross-representation equivalence

For each candidate, compare:

- field representation;
- CAD/B-rep;
- surface mesh;
- volume mesh.

Required checks include:

\[
\epsilon_V =
\frac{|V_{\mathrm{field}} - V_{\mathrm{mesh}}|}
     {V_{\mathrm{field}}},
\]

surface-area discrepancy, bidirectional Hausdorff distance, boundary-patch area, port centroid and normal, connectivity, and region labels.

A mesh can be manifold and still represent the wrong design. Therefore:

> `watertight == true` is necessary but nowhere near sufficient.

---

# 6. Variation operators for actual alien invention

The operator system should contain both local and radical emitters.

| Operator | Purpose | Required invariant |
|---|---|---|
| Scalar perturbation | Fine local search | Typed bounds and units |
| Basis coefficient mutation | Smooth field variation | Feature-scale bounds |
| Add/remove basis term | Increase or reduce shape complexity | Continuity preserved |
| Split channel | Introduce topology | Conservation and port compatibility |
| Merge channels | Simplify topology | Connectivity preserved |
| Branch/rejoin | Create network motifs | No illegal cycles unless allowed |
| Duplicate and diverge | Produce repeated modules with variation | New unique IDs and lineage |
| Insert module | Add guide, chamber, porous region, collector | Typed ports match |
| Delete module | Simplification and ablation | Required terminal connectivity retained |
| Rewire typed subgraph | Radical topology exploration | Type system passes |
| Symmetry mutation | Add, remove, or partially break symmetry | Interface remains fixed |
| Warp composition | Organic freeform change | Jacobian lower bound |
| Rewrite-rule mutation | Change developmental logic | Expansion remains bounded |
| Pattern-network mutation | New spatial regularity | Output and Lipschitz bounds |
| Subgraph transplantation | Exaptation from an archive design | Typed boundary compatibility |
| Homologous subgraph crossover | Recombine functioning modules | Port signature match |
| Gradient refinement | Optimize continuous parameters | No topology change |
| Explicit repair | Recover near-feasible proposals | Repair receipt and distance |

## 6.1 Multi-scale mutation

Each operator should declare a `MutationScale`:

```rust
pub enum MutationScale {
    Micro,
    Local,
    Module,
    Topology,
    Developmental,
}
```

The search should maintain emitters at all scales. Otherwise, selection will gradually suppress radical changes because local perturbations have a higher immediate survival rate.

---

## 6.2 True homologous crossover

The existing crossover copies named layers and blends scalar values. That is useful, but it is not graph crossover.

True crossover should:

1. identify subgraphs with matching typed boundary signatures;
2. calculate structural correspondence;
3. exchange complete subgraphs;
4. reconcile referenced fields and parameters;
5. canonicalize IDs;
6. type-check the result;
7. preserve full parentage.

A subgraph signature might contain:

```text
number and types of boundary ports
flow directions
section shapes
material domains
continuity requirements
bounding envelope
terminal reachability
```

---

## 6.3 Operator adaptation

Maintain per-operator statistics:

```text
attempt count
compile rate
F0 validity rate
distinct-phenotype rate
behavioral novelty
performance improvement
archive insertion rate
computational cost
repair distance
```

Use a bandit or adaptive scheduler to allocate proposal budget. Do not optimize only immediate fitness; otherwise the engine will suppress operators responsible for rare but profound jumps.

---

# 7. Measuring the genome itself

A sophisticated design engine should qualify not only designs but also the **representation**.

## 7.1 Genome quality metrics

### Viable mutation yield

\[
Y_v =
\frac{\text{distinct F0-valid offspring}}
     {\text{variation attempts}}.
\]

Report separately for local and radical operators.

### Phenotype uniqueness

\[
Y_u =
\frac{\text{unique phenotype hashes}}
     {\text{compiled offspring}}.
\]

This detects neutral-gene bloat and baseline collapse.

### Topological reach

Measure coverage over:

- graph motifs;
- branch/rejoin counts;
- cycle structure;
- connected components;
- Betti numbers;
- Reeb-graph signatures;
- internal-surface topology.

### Locality

For micro mutations, measure the relationship between genotype edit magnitude and phenotype distance. A good representation should support:

- high locality for fine optimization;
- occasional controlled discontinuities for innovation.

### Evolvability

For a parent \(g\), sample viable offspring and calculate the dispersion of measured behaviors:

\[
E(g) =
\log \det
\left(
\operatorname{Cov}
[
b(g_1),\ldots,b(g_n)
]
+\epsilon I
\right).
\]

A design can be valuable as a stepping stone even before it is the best performer.

### Causal gene coverage

For each active gene or graph element, perturb or remove it and test whether the phenotype changes measurably. Track:

- inactive-gene fraction;
- duplicate-expression fraction;
- hidden epistasis;
- phenotype sensitivity.

### Scientific innovation yield

\[
Y_{\mathrm{inv}} =
\frac{\text{new F2-qualified mechanism clusters}}
     {\text{total high-fidelity cost}}.
\]

This is ultimately more important than “number of candidates evaluated.”

---

# 8. Alienness must be a vector, not a hype score

Define:

\[
\mathbf{A}(x) =
\left[
N_{\mathrm{topology}},
N_{\mathrm{morphology}},
N_{\mathrm{development}},
N_{\mathrm{mechanism}}
\right].
\]

## 8.1 Topology novelty

Distance from known designs using:

- typed graph edit distance;
- graph kernels;
- motif distributions;
- homology signatures;
- channel-network spectra.

## 8.2 Morphology novelty

Distance using:

- canonical SDF samples;
- spherical or cylindrical spectral descriptors;
- curvature distributions;
- medial-axis structure;
- thickness fields;
- surface-area-to-volume structure.

## 8.3 Developmental novelty

Distance between derivation programs:

- rewrite-rule sequences;
- module ancestry;
- duplication patterns;
- developmental depth;
- operation motifs.

## 8.4 Mechanism novelty

Distance in **measured physical behavior**, such as:

- axial swirl-number profile;
- helicity spectrum;
- recirculation topology;
- reverse-flow-core continuity;
- residence-time distribution;
- turbulent-dissipation exposure;
- droplet-breakup hazard;
- pressure-recovery distribution;
- separation grade curve.

A design is genuinely interesting when it is novel in mechanism, not merely visually strange.

Alienness should control exploration and archive coverage. It must not relax:

- mass conservation;
- structural safety;
- numerical convergence;
- uncertainty;
- physical validation;
- promotion criteria.

A bizarre failure is still a useful curiosity-archive entry. It is not an engineering elite.

---

# 9. Search architecture

Novelty search exists precisely because direct optimization can be deceptive, while open-ended systems such as POET demonstrate the value of diverse challenges, solution transfer, and stepping stones. These ideas are highly relevant—but JOIL must apply them inside a frozen physical and safety envelope. [Swarthmore Computer Science](https://www.cs.swarthmore.edu/~meeden/DevelopmentalRobotics/lehman_ecj11.pdf?utm_source=chatgpt.com)

## 9.1 Three archives

### Archive A: morphology

Candidate placement derives only from compiled geometry.

### Archive B: mechanism

Placement derives only from solver-measured behavior.

### Archive C: robust utility

Placement and dominance use:

- hard feasibility;
- uncertainty;
- operating-envelope robustness;
- true engineering objectives.

A candidate can occupy all three archives with the same identity.

---

## 9.2 Adaptive rather than fixed bins

Replace fixed bins with one of:

- CVT-MAP-Elites;
- adaptive centroid splitting;
- learned behavior embeddings frozen per campaign;
- hierarchical archives.

The descriptor model must be versioned and frozen for a campaign. Changing it mid-campaign changes archive meaning.

---

## 9.3 Emitter ensemble

```rust
pub enum EmitterKind {
    LocalParameter,
    TopologyMutation,
    DevelopmentalMutation,
    Novelty,
    RepairBoundary,
    HomologousCrossover,
    SubgraphTransplant,
    GradientRefinement,
    RandomGrammar,
    AdversarialScenario,
}
```

Each emitter should compete for budget based on:

- archive contribution;
- information gain;
- novelty gain;
- robust-performance gain;
- computational cost.

Multi-emitter QD is better suited than one mutation distribution to combining fine engineering optimization with rare topology jumps.

---

## 9.4 Outer topology loop and inner continuous loop

Use two nested search modes:

### Outer loop

Evolutionary or grammar-based search changes:

- topology;
- developmental rules;
- module composition;
- symmetry;
- field basis.

### Inner loop

Within a fixed topology, optimize continuous values using:

- CMA-ES;
- Bayesian optimization;
- local gradients;
- adjoints or differentiable solvers where trustworthy.

Differentiable simulation can provide useful local gradients, but its role should be continuous refinement, not a replacement for global topology exploration. [arXiv](https://arxiv.org/abs/1910.00935?utm_source=chatgpt.com)

---

## 9.5 Adversarial scenario coevolution

A second population can search for operating conditions that expose weaknesses:

- flow rate;
- reject pressure;
- viscosity;
- density contrast;
- interfacial tension;
- droplet distribution;
- roughness;
- dimensional variation;
- partial obstruction;
- inlet imbalance.

The scenario generator must remain inside a frozen requirement envelope.

It should optimize failure discovery, not move goalposts. Every scenario must be:

- physically valid;
- bounded;
- content-addressed;
- interpretable;
- retained in the evidence ledger.

Transfer successful designs between scenario niches. This can uncover robust stepping stones without permitting unconstrained “game invention.”

---

# 10. Hard-science evaluator architecture

## 10.1 Evaluator result type

```rust
pub struct Observation {
    pub observable: ObservableId,
    pub value: f64,
    pub units: UnitId,

    pub aleatory_uncertainty: Option<Uncertainty>,
    pub numerical_uncertainty: Option<Uncertainty>,
    pub model_discrepancy: Option<Uncertainty>,

    pub fidelity: FidelityLevel,
    pub applicability: ApplicabilityAssessment,
    pub scenario: ContentRef,
    pub evidence: ContentRef,
}
```

Every evaluator returns an observation bundle, not a fitness scalar.

---

## 10.2 Failure taxonomy

```rust
pub enum EvaluationState {
    Valid(ObservationBundle),

    RequirementRejected {
        violations: Vec<Violation>,
    },

    OutOfApplicability {
        reasons: Vec<ApplicabilityViolation>,
    },

    NumericalFailure {
        solver: ContentRef,
        diagnostics: NumericalDiagnostics,
    },

    InfrastructureFailure {
        diagnostics: InfrastructureDiagnostics,
    },

    Indeterminate {
        reasons: Vec<String>,
    },
}
```

A numerical failure is not poor engineering performance. An infrastructure failure is not infeasibility. An indeterminate result cannot be promoted.

---

## 10.3 Constraint handling

Do not reduce heterogeneous violations to one arbitrary scalar.

Use lexicographic constrained dominance:

1. identity completeness;
2. interface and geometry validity;
3. manufacturability;
4. numerical credibility;
5. hard physical requirements;
6. robust/chance constraints;
7. Pareto performance;
8. novelty and complexity.

Where uncertainty matters, use interval decisions:

- pass when the conservative bound passes;
- fail when the optimistic bound fails;
- otherwise indeterminate.

For an upper-bound requirement \(y \le y_{\max}\):

```text
PASS          if UCB(y) <= y_max
FAIL          if LCB(y) >  y_max
INDETERMINATE otherwise
```

---

## 10.4 Proposed fidelity ladder

### F−1: formal and static checks

- graph typing;
- dimensional consistency;
- conservation at declared junctions;
- interface immutability;
- expansion-budget compliance;
- lower-bound passage estimates.

### F0-G: geometric qualification

- exact connectivity;
- wall and passage bounds;
- cavities;
- manifold properties;
- mesh fidelity;
- patch semantics.

### F0-M: manufacturing qualification

- minimum feature;
- overhang;
- support access;
- powder removal;
- machining access;
- orientation sensitivity;
- tolerance morphology.

### F0-S: structural screen

Retain the current analytical evaluator here.

### F1: generic reduced physics

Use phenotype-derived generic calculations:

- 1D network flow;
- cross-sectional resistance;
- streamline curvature;
- reduced vortex models;
- low-resolution CFD.

No logic may branch on a genome feature name.

### F1.5: surrogate

Used for acquisition and uncertainty-aware prescreening, never final promotion.

### F2-C: converged carrier CFD

- RANS/RSM;
- mesh convergence;
- closure sensitivity;
- mass and momentum balance;
- pressure and energy accounting.

### F2-D: dispersed phase

- Lagrangian tracking or Eulerian population balance;
- full residence inventory;
- mass closure;
- turbulence dispersion;
- breakup and coalescence sensitivity;
- wall interactions;
- grade-efficiency uncertainty.

### F3: transient multiphysics

As applicable:

- LES/DES;
- transient gas or air core;
- VOF;
- cavitation;
- coupled population balance;
- transient inlet disturbances.

### F4: controlled physical rig

The existing blinded A/B/A, calibration, power, holdout, and as-built metrology logic is an excellent foundation and should be preserved. 

### F5: independent replication

- independent lab;
- independent specimen preparation;
- frozen protocol;
- commercial comparator where a commercial claim is made.

---

# 11. Physics-derived behavior descriptors

The current `MechanismDescriptors` should be renamed to `HypothesisDescriptors` and excluded from scoring. They may remain useful for human explanation or stratified diagnostics.

The mechanism archive should instead use quantities measured from simulations.

## Hydraulic behavior

- pressure-drop curve;
- specific energy;
- pressure recovery by axial region;
- absolute pressure minimum;
- cavitation margin;
- reject-split stability;
- inlet maldistribution.

## Flow topology

- swirl-number profile \(S(z)\);
- axial and tangential momentum flux;
- reverse-flow-core radius and continuity;
- recirculation component count;
- stagnation and saddle structures;
- helicity distribution;
- vortex-core trajectories;
- precessing-vortex frequency;
- flow-topology persistence across duty points.

## Residence behavior

- tracer residence-time distribution;
- bypass fraction;
- short-circuit fraction;
- dead-volume fraction;
- overflow and underflow first-passage distributions.

## Droplet experience

- turbulent-dissipation exposure distribution;
- integrated strain exposure;
- maximum pressure-gradient exposure;
- breakup hazard;
- coalescence opportunity;
- wall-impact probability;
- residence-conditioned capture probability.

## Separation

- complete grade-efficiency curve;
- cut size with uncertainty;
- reduced-efficiency curve;
- treated concentration;
- oil mass closure;
- sensitivity to inlet DSD.

These descriptors allow the system to discover a new mechanism without first naming it.

---

# 12. Causal mechanism discovery

An alien shape is scientifically valuable only when JOIL can establish **why** it works.

For every elite, automatically create matched counterfactuals:

1. delete a module;
2. neutralize its field amplitude;
3. replace it with a smooth matched-volume control;
4. phase-randomize a periodic structure;
5. preserve pressure drop while removing topology;
6. preserve mass while removing topology;
7. transplant the module into another parent;
8. test pairwise module interactions.

A mechanism claim should look like:

```rust
pub struct MechanismClaim {
    pub candidate: ContentRef,
    pub claimed_mechanism: String,
    pub observable: ObservableId,

    pub intervention: ContentRef,
    pub control: ContentRef,

    pub paired_effect: Estimate,
    pub confidence_interval: Interval,
    pub fidelity: FidelityLevel,

    pub cross_duty_replication: Vec<ContentRef>,
    pub status: ClaimStatus,
}
```

This prevents post hoc storytelling.

A claimed “TPMS coalescence mechanism,” for example, would require evidence that:

- the actual resolved or homogenized TPMS exists;
- removing it reduces the relevant response;
- the result is not merely caused by increased pressure drop;
- the effect persists across mesh and closure choices;
- the effect survives relevant duty changes;
- the droplet model supports the proposed causal pathway.

---

# 13. Surrogate redesign

JOIL’s current GP is a useful prototype, but its twelve fixed features are derived from the same human mechanism ontology as the fast evaluator. It also substitutes baseline-derived descriptors when descriptor extraction fails.  

## 13.1 Surrogate inputs

Use a combination of:

### Graph features

- typed production counts;
- graph spectral features;
- motif frequencies;
- branch/rejoin statistics;
- path lengths;
- typed Weisfeiler-Lehman embeddings.

### Geometric features

- SDF spectral representation;
- cross-sectional profiles;
- medial-axis features;
- curvature and thickness distributions;
- surface-area and volume moments.

### Low-fidelity physical fields

- coarse velocity and pressure modes;
- low-resolution RTD;
- reduced-flow quantities;
- structural-screen results.

No direct feature should state that a named mechanism is beneficial.

---

## 13.2 Multi-fidelity model

Predict high fidelity as a corrected lower-fidelity response:

\[
y_H(x)
=
\rho(x)y_L(x)
+
\delta(x).
\]

Support multiple levels:

```text
analytic screen
coarse CFD
fine CFD
dispersed-phase CFD
transient multiphysics
physical test
```

Use cost-aware acquisition. Surrogate-assisted illumination has already demonstrated that QD-style design maps can be produced with far fewer expensive evaluations when uncertainty is used intelligently. [arXiv](https://arxiv.org/abs/1806.05865?utm_source=chatgpt.com)

---

## 13.3 Required uncertainty controls

Each surrogate must provide:

- leave-one-topology-production-out validation;
- leave-one-mechanism-cluster-out validation;
- held-out duty validation;
- predictive interval coverage;
- calibration curve;
- OOD or support score;
- nearest supporting observations;
- failure-classifier uncertainty.

A candidate beyond support should be routed toward real evaluation, not assigned unjustified confidence.

Surrogate output may decide **what to evaluate next**. It may never be the sole evidence for promotion.

---

## 13.4 Acquisition function

For a candidate \(x\), consider:

\[
a(x)
=
\frac{
\operatorname{EHVI}(x)
\cdot P_{\mathrm{feasible}}(x)
\cdot
\left(
1+\lambda_n N(x)+\lambda_i I(x)
\right)
}{
\operatorname{ExpectedCost}(x)
}.
\]

Where:

- `EHVI` is expected hypervolume improvement;
- \(N(x)\) is novelty;
- \(I(x)\) is expected information gain;
- feasibility is modeled per constraint;
- expected cost includes meshing and solver risk.

Keep performance, novelty, and information gain separately observable even if they participate in acquisition.

---

# 14. Requirements must be executable data

The current code duplicates many thresholds as constants. This guarantees eventual drift.

Create one requirement registry:

```rust
pub struct RequirementSet {
    pub identity: ContentRef,
    pub status: RequirementStatus,
    pub applicability: ApplicabilityEnvelope,
    pub requirements: BTreeMap<RequirementId, Requirement>,
    pub promotion_policy: PromotionPolicy,
}
```

Each requirement should contain:

```rust
pub struct Requirement {
    pub id: RequirementId,
    pub description: String,
    pub observable: ObservableId,
    pub relation: Relation,
    pub threshold: QuantityValue,
    pub uncertainty_policy: UncertaintyPolicy,
    pub applicable_scenarios: Vec<ScenarioSelector>,
    pub minimum_fidelity: FidelityLevel,
    pub severity: RequirementSeverity,
    pub source: ContentRef,
}
```

No evaluator should contain an unexplained constant such as:

```rust
if pdr < 4.5 { ... }
```

It should receive the requirement and produce an observation. A separate compliance engine evaluates the observation against the requirement.

This separation allows the same design evidence to be assessed against:

- research requirements;
- a customer duty;
- a manufacturing requirement;
- a regulatory requirement;
- a different material/process assumption.

---

# 15. Evidence and claim state machine

```text
PROPOSED
   ↓
CANONICAL
   ↓
COMPILED
   ↓
F0_GEOMETRY_VALID
   ↓
F0_MANUFACTURABLE
   ↓
F1_SCREENED
   ↓
F2_NUMERICALLY_VERIFIED
   ↓
F2_DISPERSED_PHASE_VERIFIED
   ↓
F3_MULTIPHYSICS_VERIFIED
   ↓
F4_PHYSICALLY_VALIDATED
   ↓
F5_INDEPENDENTLY_REPLICATED
```

Each transition requires a policy-defined evidence bundle.

A design cannot skip states merely because its score is excellent.

Claims should also have scope:

```rust
pub enum ClaimStatus {
    Exploratory,
    ScreenSupported,
    NumericallySupported,
    PhysicallySupported,
    IndependentlyReplicated,
    Rejected,
    Superseded,
}
```

Public README claims should be generated from the evidence ledger, as Jarambit already demonstrates conceptually. The strongest admissible state should be computed, not manually described.

---

# 16. Genome qualification suite

I would add a dedicated test catalog.

| ID | Test | Required result |
|---|---|---|
| `GEN-001` | Exact legacy baseline import | Same immutable external interfaces |
| `GEN-002` | Exact serialization round trip | Genotype hash unchanged |
| `GEN-003` | No silent fallback | Zero baseline substitution events |
| `GEN-004` | Schema migration identity | Migration receipt always emitted |
| `GEN-005` | Active-gene causality | Every active element changes expanded IR or is flagged |
| `GEN-006` | Local mutation viability | Reported by operator; target set from empirical calibration |
| `GEN-007` | Radical mutation viability | Nonzero sustained yield without repair hiding |
| `GEN-008` | Distinct phenotype yield | No silent duplicate inflation |
| `GEN-009` | Topology reach | Produces valid designs outside the eleven legacy tags |
| `GEN-010` | Module duplication/divergence | Repeated modules can evolve independently |
| `GEN-011` | Typed crossover | Exchanged subgraphs preserve port semantics |
| `GEN-012` | Developmental bound | Expansion always terminates within budget |
| `GEN-013` | Neutral-network audit | Neutral mutations measured, not discarded |
| `GEN-014` | Evolvability audit | Behavioral dispersion reported per parent |
| `GEN-015` | Complexity audit | Complexity costs separated from engineering fitness |
| `CMP-001` | Field-to-mesh volume agreement | Within declared tolerance |
| `CMP-002` | Field-to-mesh surface agreement | Within declared Hausdorff tolerance |
| `CMP-003` | Terminal interface invariance | Exact for all candidates |
| `CMP-004` | Internal-feature realization | Every claimed module appears in region/patch census |
| `CMP-005` | Fluid connectivity | Required terminal paths exist |
| `CMP-006` | Trapped cavity detection | Adversarial corpus detected |
| `CMP-007` | Thin-feature detection | Violations found conservatively |
| `CMP-008` | Dynamic patch semantics | Internal walls and zones correctly labeled |
| `EVAL-001` | Evaluator blindness | Evaluator crate has no genome dependency |
| `EVAL-002` | Phenotype equivalence | Same phenotype gives same result |
| `EVAL-003` | Gene-label permutation | Renaming/re-encoding cannot change physics |
| `EVAL-004` | Manufactured solutions | Solver recovers known results |
| `EVAL-005` | Conservation | Mass, momentum, and dispersed-phase inventories close |
| `EVAL-006` | Mesh convergence | Promotion blocked when unresolved |
| `EVAL-007` | Closure sensitivity | Required cross-model parity reported |
| `EVAL-008` | Numerical-failure quarantine | Failures never become training targets |
| `STAT-001` | True Pareto hypervolume | Verified against analytic cases |
| `STAT-002` | Objective-contract parity | Exact manifest/code agreement |
| `STAT-003` | Bootstrap protocol parity | Resample count and method exact |
| `STAT-004` | Actual LHS baseline | Strata occupancy verified |
| `STAT-005` | Equal-cost comparison | Solver and compute costs conserved |
| `STAT-006` | Holdout single-use | Cryptographically enforced |
| `EVID-001` | Complete scientific identity | No observation without full identity |
| `EVID-002` | Artifact hash verification | All dependencies resolvable |
| `EVID-003` | Requirement admission | Draft requirements block promotion |
| `EVID-004` | Claim derivation | Public status reproducible from evidence |
| `SCI-001` | Module ablation | Elite claims have paired counterfactuals |
| `SCI-002` | Duty robustness | Required duty envelope passes |
| `SCI-003` | As-built robustness | Tolerance geometry included |
| `SCI-004` | Independent replication | Required for strongest claim state |

---

# 17. File-by-file redesign

## `crates/joil-core/src/program.rs`

Split into:

```text
crates/joil-genome/
crates/joil-ir/
crates/joil-variation/
```

Remove:

- fixed eleven-family authority;
- direct mechanism-benefit descriptors;
- silent fallback;
- schema migration inside normalization.

Retain:

- immutable baseline reference;
- deterministic canonicalization;
- inactive-expression removal;
- exact interface constraints.

---

## `crates/joil-core/src/phenotype.rs`

Replace the radial-ring-centric compiler with:

```text
crates/joil-compiler/
  expansion.rs
  typecheck.rs
  fields.rs
  graph_lowering.rs
  multi_region.rs
  topology.rs
  mesh.rs
  patch_semantics.rs
  equivalence.rs
```

The compiler must output actual internal topology.

---

## `crates/joil-core/src/cfd_case.rs`

Change from fixed five-patch surface assumptions to:

- immutable terminal roles;
- dynamic internal patches;
- porous zones;
- face zones;
- material zones;
- mesh-resolution policies per feature;
- candidate-specific meshability receipts.

The OpenFOAM manifest should include:

```text
expanded_ir_hash
fluid_field_hash
surface_hash
volume_mesh_hash
region census
patch census
cell-zone census
mesh-quality distribution
geometry-equivalence report
```

---

## `crates/joil-core/src/evolve.rs`

Replace fixed arrays and scalarized violation logic with:

- manifest-driven objective access;
- named observations;
- constrained dominance;
- validated hypervolume;
- emitter ensemble;
- structured invalid attempts;
- exact cost ledger;
- lineage receipts;
- three archives.

Remove any objective that is not present in the frozen campaign contract.

---

## `crates/joil-core/src/surrogate.rs`

Replace the single twelve-dimensional handcrafted feature vector with:

- pluggable encoders;
- topology-aware features;
- geometry embeddings;
- multi-output models;
- multi-fidelity residuals;
- OOD assessment;
- conformal or otherwise calibrated intervals;
- per-constraint feasibility models.

Never substitute the baseline when feature extraction fails. Return a structured missing-feature or compilation state.

---

## `crates/joil-core/src/redteam.rs`

Convert thresholds into requirement references.

Add:

- cross-representation equivalence;
- module-realization census;
- semantic connectivity;
- adversarial geometry corpus;
- manufacturing morphology;
- field-to-mesh error;
- dynamic region checking.

---

## `crates/joil-core/src/structure.rs`

Rename the existing path to indicate screening fidelity.

Add a generic solver interface:

```rust
pub trait StructuralEvaluator {
    fn evaluate(
        &self,
        solid: &SolidDomain,
        loads: &MappedLoads,
        material: &MaterialModel,
        scenario: &Scenario,
    ) -> EvaluationState;
}
```

---

## `crates/joil-core/src/experiment.rs`

Preserve its strongest features:

- blinded comparisons;
- anti-pseudoreplication;
- power analysis;
- holdout isolation;
- as-built metrology;
- matched-duty comparator requirements. 

Add complete scientific identity to every experimental block and raw observation.

---

## New `crates/joil-evidence`

Port and extend Jarambit’s:

- `ContentRef`;
- exact subject identity;
- canonical wire format;
- duplicate-key rejection;
- requirement/evaluator/scenario binding;
- claim derivation;
- append-only evidence graph.

JOIL already has useful checkpoint hashing and a budget ledger; those should become nodes in this broader evidence graph. 

---

# 18. Migration strategy

## Phase 0: freeze the current semantics

Record the existing implementation as:

```text
ct35-exotic-head-v2-legacy
legacy-radial-surface-compiler-v1
legacy-descriptor-evaluator-v1
```

Do not overwrite its artifacts.

Mark current CFD evidence as applying to the exact legacy compiled geometry. Avoid carrying internal-mechanism claims forward automatically.

---

## Phase 1: scientific-integrity repairs

Before expanding the genome:

1. remove all baseline fallbacks;
2. fix schema identity;
3. bind code to campaign objectives;
4. implement real hypervolume;
5. execute the declared bootstrap protocol;
6. implement actual LHS;
7. enforce requirement admission status;
8. add complete lineage;
9. import Jarambit-style scientific identity.

This makes the current search scientifically interpretable even before v4.

---

## Phase 2: evaluator blindness

1. move phenotype-independent evaluation out of `program.rs`;
2. prevent evaluator imports from genome crates;
3. remove feature-name bonuses;
4. add phenotype-equivalence tests;
5. separate hypothesis descriptors from measured descriptors.

---

## Phase 3: TDFG and legacy lowering

Implement v4 with a legacy adapter:

```text
legacy ramp            → sweep/warp macro
legacy split           → true channel graph
legacy ribbon          → actual guide-surface module
legacy TPMS            → resolved or porous field region
legacy chamber modes   → Fourier field
legacy cone profile    → spline field
legacy skimmer         → internal solid module
```

Preserve the legacy source hash in the migration receipt, but assign a new v4 phenotype hash.

---

## Phase 4: multi-region compiler

The first hard milestone is not a prettier mesh. It is this:

> A legacy six-jet design produces six distinct, inspectable, connected internal jet passages in the CFD domain.

Then add:

- guide surfaces;
- resolved TPMS;
- porous zones;
- arbitrary graph topology;
- geometry-equivalence tests.

---

## Phase 5: open-ended variation and archives

Add structural operators, dynamic archives, novelty, evolvability, and emitter adaptation.

Keep the legacy eleven families only as seeds and regression cases.

---

## Phase 6: multifidelity science

Add:

- generic low-fidelity solvers;
- topology-aware surrogates;
- uncertainty;
- cost-aware acquisition;
- adversarial operating scenarios;
- causal ablation generation.

---

## Phase 7: physical invention qualification

Only at this stage should the strongest term “intelligent invention” be attached to a candidate:

1. representation is open-ended;
2. phenotype contains the proposed topology;
3. evaluator is blind to the genotype;
4. performance survives correct statistics;
5. mechanism survives ablation;
6. result survives operating and manufacturing variation;
7. high-fidelity simulation converges;
8. physical testing validates the gain;
9. independent replication confirms it.

---

# 19. Immediate no-regret changes

I would make these changes before spending another major GPU campaign on the current search:

1. **Stop baseline fallback immediately.**
2. **Stop normalizing v3 into v2.**
3. **Freeze and repair the preregistration/objective mismatch.**
4. **Replace the current hypervolume function.**
5. **Make actual splitter, ribbon, and TPMS geometry appear in the analysis domain.**
6. **Remove all direct gene-name effects from the physics evaluator.**
7. **Make evaluators depend only on phenotype artifacts.**
8. **Import Jarambit’s full scientific identity and lineage pattern.**
9. **Make requirement files the sole source of thresholds.**
10. **Demote the eleven families from ontology to seed recipes.**
11. **Implement a typed graph plus implicit-field IR.**
12. **Build morphology and mechanism archives from measured outputs.**

---

# 20. Definition of success

JOIL becomes an alien-capable intelligent-design process when all of the following are true:

- A candidate can contain a topology that has no pre-existing JOIL family name.
- The design can be encoded compactly through graph, field, and developmental structure.
- Every internal feature exists in the compiled physics domain.
- The evaluator cannot tell which genes or named modules produced the geometry.
- Two genotype encodings of the same geometry receive the same physical result.
- Invalid mutations remain visible and teach the system about feasibility.
- Novelty is measured in topology, morphology, development, and physical mechanism.
- Novelty never substitutes for engineering evidence.
- Requirements, objectives, scenarios, statistics, and evaluators are content-addressed and frozen.
- Every result binds genome, expansion, compiler, CAD, mesh, solver, scenario, material, requirements, and source.
- The design’s benefit survives matched ablation.
- The benefit survives convergence, model sensitivity, operating variation, manufacturing variation, and holdout testing.
- The strongest claims require physical and independent evidence.

The current JOIL architecture already contains an unusually strong scientific backbone. The profound change is not “more exotic features.” It is turning the genome from a **list of clever ideas humans already had** into a **language capable of expressing ideas no human enumerated**, while making the scientific judge completely indifferent to how those ideas were generated.

That is the architecture that can produce something legitimately alien—and still tell you, with evidence, whether it actually works.
