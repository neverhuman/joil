# JOIL Xenogen v4
## Engineering specification for open-ended, science-gated alien invention

**Status:** Architecture and implementation feedback based on a code-level comparison of `neverhuman/joil` and `neverhuman/jarambit` as accessed on 2026-10-15.  
**Purpose:** Transform JOIL from a strong but curated exotic-parameter optimizer into an open-ended intelligent design system whose inventions are represented faithfully, evaluated independently, and promoted only by hard scientific evidence.

---

## 0. Executive decision

JOIL should **not** be improved by adding another twenty exotic genes or another list of named topology families.

The present JOIL implementation is already sophisticated: it has a compositional 3D parameterization, multi-island constrained NSGA-II, MAP-Elites, a GP screen, Monte Carlo perturbations, structural and modal proxies, OpenFOAM tooling, PBM verification utilities, and unusually honest documentation about qualification boundaries. Its limitation is architectural:

> The implemented genome is a fixed catalog of mechanisms, while the project vision requires a language that can invent mechanisms not anticipated by the programmer.

Jarambit provides the complementary lesson. Its geometry is also fundamentally bounded by fixed representations, but its lineage, scientific identity, typed observations, hard-gate hierarchy, and separation between search fitness and promotion evidence are much more mature.

The recommended synthesis is:

1. **JOIL's multiphysics ambition and full-system duty definition**
2. **Jarambit's evidence custody, exact subject binding, lineage, and hard-gate discipline**
3. **A new variable-length typed developmental graph genome**
4. **One canonical GeometryIR and one identity chain across B-rep, SDF, mesh, solver, objective, and verdict**
5. **An independent scientific adjudicator that cannot be influenced by the generator**
6. **Quality-diversity search over actual topology and measured physical behavior, not hand-authored family names**
7. **Alienness as an exploration and discovery signal, never as permission to violate physics**

This document calls the target architecture **JOIL Xenogen v4**.

---

# 1. What the repository comparison actually shows

## 1.1 JOIL's strongest qualities

JOIL already contains several things that should be preserved:

- An explicit baseline identity.
- Frozen interface and downstream geometry intent.
- A staged F0–F4 evidence philosophy.
- Constrained multi-objective search.
- Quality-diversity and multi-island search infrastructure.
- Physics-oriented mechanism descriptors.
- Monte Carlo and held-out duty concepts.
- A serious PBM and light-droplet verification library.
- A true CadQuery/B-rep path for a subset of the design language.
- Clear qualification caveats in the status and master-plan documents.
- A project-level understanding that novelty is not evidence.

These are rare strengths. The required change is not a rewrite of the scientific mission. It is a rewrite of the **representation-to-evidence spine**.

## 1.2 Jarambit's strongest transferable qualities

Jarambit should not be copied mechanically. Its fixed vectors and aesthetic envelope are application-specific. The pieces worth transplanting are:

- Content-bound scientific identity.
- Typed observations that distinguish value, not evaluated, and failed.
- Requirement-scoped verdicts.
- Exact parent/operator/version lineage.
- Append-only durable event history.
- Hard gate first, objective second.
- Search evidence separated from certification evidence.
- Honest non-promotion when the current requirement set is not met.
- Native-CAD subject binding.
- Explicit evaluator semantics and requirement versions.

JOIL should generalize these ideas to CFD, geometry, topology, manufacturing, uncertainty, and experimental evidence.

## 1.3 Neither project yet has open-ended invention

Jarambit has evolved from a small fixed genotype to a much richer spline, thickness, lattice, and machining vector, but it remains an indexed fixed vector.

JOIL's current Rust `DesignProgram` is broader but still a fixed product type containing:

- one inlet ramp,
- one inlet splitter,
- one ribbon cascade,
- one TPMS insert,
- a bounded chamber-mode list,
- one cone-mode set,
- one skimmer.

The eleven topology families are labels inferred from combinations of those fields. They are not arbitrary graph topologies.

The target must therefore be a new class of representation, not a bigger vector.

---

# 2. Release-blocking findings in the current JOIL implementation

These findings should be treated as **P0 engineering issues** before claims about open-ended alien design.

## P0-01 — Rust search genomes and canonical B-rep geometry are not equivalent

The Rust Wave-2 representation can encode:

- four branches per port,
- helical ribbon cascades,
- TPMS inserts,
- additional schema-v3 fields.

The CadQuery compiler path accepts an older, narrower program and does not compile the same Wave-2 structures. Some branch counts are coerced, and ribbon/TPMS fields are absent from that schema.

### Consequence

A candidate may be:

1. generated by the Rust genome,
2. scored by the Rust reduced-order path,
3. labeled as a TPMS/ribbon/four-way topology,
4. exported in results,

without an equivalent canonical B-rep subject.

That breaks scientific identity. The object being scored is not guaranteed to be the object being manufactured, meshed, or shown.

### Required fix

- Define one shared `GeometryIR`.
- Compile every supported genome feature into that IR.
- Make Rust and CadQuery consume the same IR.
- Refuse qualification if any feature is unsupported.
- Emit `NotEvaluated(UnsupportedGeometryFeature)` rather than an approximate pass.

---

## P0-02 — Invalid offspring silently become the baseline

Several mutation and crossover paths end with the equivalent of:

```rust
candidate.normalize().unwrap_or_else(|_| DesignProgram::baseline_ct35())
```

### Consequence

This causes five failures:

1. Operator defects are hidden.
2. Invalid proposals are counted as valid baseline-like candidates.
3. Population statistics and archive occupancy are biased.
4. Parent/operator lineage becomes false.
5. Search can appear more robust than it is.

### Required fix

All variation APIs must return an explicit result:

```rust
pub enum BirthOutcome {
    Accepted {
        raw: RawGenome,
        normalized: NormalizedGenome,
        repair: Option<RepairRecord>,
    },
    Rejected {
        proposal: RawGenome,
        reason: RejectionReason,
        diagnostics: Vec<Diagnostic>,
    },
}
```

The baseline must never be an error fallback.

---

## P0-03 — The structural convergence-order calculation is wrong

The current observed-order expression uses floating-point Euclidean division rather than ordinary division.

Required expression:

```rust
let p_obs = (e32 / e21).ln() / refinement_ratio.ln();
```

Not:

```rust
(e32 / e21).ln().div_euclid(refinement_ratio.ln())
```

### Required tests

Synthetic data with exact error laws:

- `E(h) = C h^1.5`
- `E(h) = C h^2.0`
- `E(h) = C h^3.2`
- oscillatory/non-monotone convergence negative controls
- near-zero-difference conditioning tests

No GCI verdict should be issued when asymptotic assumptions fail.

---

## P0-04 — Load-transfer conservation is self-confirming

The current structural screen computes a fluid load and then directly assigns the structural reaction to the same value. The resulting relative error is necessarily zero.

### Required fix

Use two independent integrations:

- **Fluid-side source:** pressure and viscous traction integration over the CFD interface.
- **Solid-side reaction:** constrained-node reaction integration from the structural solve.
- Compare force and moment vectors in a common frame.
- Include sign, transform, and unit tests.
- Preserve both raw fields and integration receipts.

A conservation gate that uses the same value twice is not a gate.

---

## P0-05 — Several F0 fields are asserted, not measured independently

The fast Rust phenotype currently hardcodes or self-reports values such as:

- zero trapped cavities,
- zero frozen-tail difference,
- unchanged terminal geometry,
- approximate TPMS volume,
- approximate ribbon volume.

The red-team gate then trusts those fields.

### Required fix

An independent geometry auditor must recompute:

- connected components of the fluid domain,
- inlet-to-outlet reachability,
- trapped fluid cavities,
- trapped powder volumes,
- blind channels,
- frozen-tail symmetric difference,
- terminal face area/centroid/normal/shape,
- minimum local solid thickness,
- minimum fluid hydraulic diameter,
- minimum clearance to immutable interfaces,
- support-removal accessibility,
- powder evacuation paths.

The compiler may report these values for debugging, but the adjudicator must not trust them.

---

## P0-06 — The fast Rust mesh is not a general representation of the claimed mechanisms

The current fast mesh is primarily a sampled radial surface over axial/azimuthal rings. Internal features are often represented as:

- radial wall perturbations,
- scalar volume deductions,
- scalar pore estimates,
- proxy dimensions.

A TPMS insert is not faithfully represented by a sinusoidal wall modulation. A manifold graph is not faithfully represented by branch-count metadata. A splitter that does not exist as internal mesh faces cannot be cleared by a mesh manifold census.

### Required fix

Use the fast path only as a preview or low-fidelity field evaluation until it is proven equivalent for a declared subset. Every unsupported feature must remain unqualified.

---

## P0-07 — Candidate-specific structural loads are not tied to the hard constraint

The early red-team structural check uses fixed load values. A later candidate-specific structural report is attached, but the main constraint-violation calculation does not consistently turn a candidate-specific structural failure into hard infeasibility.

### Required fix

- Define a candidate load envelope from candidate CFD or a conservative bound.
- Pass the same content-bound load case into structural qualification.
- Add every structural/manufacturing failure to constraint dominance.
- Store the load identity in the structural verdict.
- Reject stale structural results after geometry or load changes.

---

## P0-08 — Geometry metrics conflate different physical meanings

A single "minimum wall clearance" concept is used across quantities that can include:

- solid feature thickness,
- flow throat width,
- annular gap,
- vane thickness,
- distance to an outer wall.

These are not interchangeable.

### Required split

At minimum:

```text
minimum_solid_ligament_mm
minimum_shell_thickness_mm
minimum_fluid_hydraulic_diameter_mm
minimum_local_flow_gap_mm
minimum_clearance_to_frozen_interface_mm
minimum_tool_access_diameter_mm
minimum_powder_exit_diameter_mm
minimum_support_removal_access_mm
```

Each has a separate measurement algorithm and gate.

---

## P0-09 — The reduced-order evaluator is circular

The current mechanism descriptors are deterministic algebraic functions of the genes. The reduced-order performance model then uses those same descriptors to predict swirl, shear, pressure loss, short-circuiting, and oil capture.

### Consequence

The search is largely optimizing the programmer's formula about a topology, not discovering the topology's behavior.

This is useful as a heuristic prior. It is not independent physics evidence.

### Required split

- `GenotypeDescriptors`: cheap, known before geometry/flow evaluation.
- `GeometryDescriptors`: measured from the compiled subject.
- `PhysicsDescriptors`: measured from simulation or experiment.
- `MechanismHypotheses`: unverified causal claims.

Only measured physics descriptors may support scientific mechanism claims.

---

## P0-10 — "Exoticness" is currently a hand-weighted feature-activation score

The current exoticness score rewards gene categories such as:

- branch count,
- TPMS basis choice,
- ribbon count/twist,
- skimmer flutes,
- formula-derived wall-rifling values.

That measures distance from the current baseline schema. It does not measure whether the design is topologically, physically, or causally novel.

### Required fix

Replace it with four separately reported quantities:

1. **Topological novelty**
2. **Geometric novelty**
3. **Flow-mechanism novelty**
4. **Causal/ablation novelty**

Never combine them into a promotion score.

---

## P0-11 — Tail quantiles are over-interpreted at small Monte Carlo counts

A nominal `p95` based on twelve realizations is effectively controlled by the largest one or two samples and has very wide uncertainty.

### Required fix

Use sequential sampling until one of these is satisfied:

- a distribution-free quantile confidence interval is narrower than the campaign threshold,
- a fitted distribution passes predeclared diagnostics and the parameter uncertainty is propagated,
- the candidate is labeled only as a coarse screen.

Use common random numbers for paired search comparisons and a separate independent random stream for final audit.

---

## P0-12 — Topology names are programmer-authored labels, not topology

`topology_class` is derived from gene combinations and names such as "schwarz-p-rectifier-head" or "hybrid-omni-exotic."

### Required fix

Derive topology from the compiled graph and geometry:

- canonical graph hash,
- branch and rejoin structure,
- cycle rank,
- connected-component count,
- Betti numbers,
- nested-region relationships,
- fluid-solid adjacency graph,
- inlet-to-outlet path signature.

Human-readable names may be generated afterward, but they must never determine archive placement or evidence identity.

---

# 3. Target architecture

## 3.1 End-to-end pipeline

```text
Immutable Interface Contract
          |
          v
Raw Developmental Genome
          |
          v
Typed Validation + Explicit Expansion
          |
          v
Canonical Expanded Design Graph
          |
          v
GeometryIR with Semantic Regions/Faces
          |
          +-------------------+
          |                   |
          v                   v
Exact B-rep Backend      SDF/Octree Backend
          |                   |
          +---------+---------+
                    |
                    v
Independent F0 Geometry/Manufacturing Audit
                    |
                    v
Mesh Family + Mesh Identity
                    |
                    v
Multi-fidelity Physics Evaluators
                    |
                    v
Content-bound Evidence Graph
             /                  \
            v                    v
 Search Archive             Promotion Judge
```

The generator and judge must be separate crates/processes with separate input contracts.

---

# 4. Genome v4: variable-length typed developmental graph

## 4.1 Top-level contract

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenomeV4 {
    pub schema: ContentRef,
    pub semantics: ContentRef,
    pub immutable_interface: ContentRef,

    pub modules: BTreeMap<ModuleId, ModuleDefinition>,
    pub instances: Vec<ModuleInstance>,
    pub fluid_graph: FluidGraphProgram,
    pub solid_program: SolidProgram,
    pub fields: BTreeMap<FieldId, FieldProgram>,

    pub material_regions: Vec<MaterialRegion>,
    pub manufacturing: ManufacturingProgram,

    pub strategy: StrategyGenes,
    pub hypothesis_tags: Vec<HypothesisTag>,
}
```

`hypothesis_tags` are explanatory metadata only. They do not alter the score.

## 4.2 Immutable interface contract

The installation contract is not a gene.

```rust
pub struct InterfaceContract {
    pub fixed_ports: Vec<FixedPort>,
    pub frozen_regions: Vec<FrozenRegion>,
    pub allowed_design_envelope: BoundedVolume,
    pub pressure_containment_role: ContainmentRole,
    pub process_envelope: ProcessEnvelope,
    pub service_scenarios: Vec<ScenarioRef>,
    pub prohibited_features: Vec<Prohibition>,
}
```

Every fixed port includes:

- exact frame,
- area/shape,
- normal,
- flow sense,
- tolerance,
- semantic role,
- downstream extension,
- mating-surface specification.

## 4.3 Typed fluid graph

```rust
pub struct FluidGraphProgram {
    pub nodes: BTreeMap<NodeId, FlowNode>,
    pub edges: BTreeMap<EdgeId, FlowEdge>,
    pub root_ports: Vec<PortBinding>,
}

pub struct FlowNode {
    pub innovation_id: InnovationId,
    pub kind: FlowNodeKind,
    pub local_frame: FrameProgram,
    pub ports: Vec<NodePort>,
}

pub enum FlowNodeKind {
    FixedInterface { contract_port: ContractPortId },
    Junction { law: JunctionLaw },
    Rejoin { law: RejoinLaw },
    Chamber { boundary: RegionProgram },
    SeparatorCell { cell: SeparatorCellProgram },
    Collector { collector: CollectorProgram },
    PorousExchange { medium: PorousMediumProgram },
    AnnularStage { stage: AnnularStageProgram },
    Cleanout { access: AccessProgram },
    PassiveRecirculator { constraints: RecirculationConstraints },
}
```

Nodes and edges expose typed ports. A liquid-flow port cannot be connected to a solid boundary port. A branch cannot violate conservation semantics.

## 4.4 Edge types

```rust
pub struct FlowEdge {
    pub innovation_id: InnovationId,
    pub from: PortRef,
    pub to: PortRef,
    pub path: CurveProgram,
    pub section: SectionProgram,
    pub frame_law: FrameLaw,
    pub wall: WallProgram,
    pub roughness: RoughnessProgram,
    pub semantic_role: FlowRole,
}

pub enum CurveProgram {
    Line { a: PointExpr, b: PointExpr },
    Arc { ... },
    Helix { ... },
    BSpline { control: Vec<PointExpr>, degree: u8 },
    StreamlineSeeded { field: FieldId, seed: PointExpr },
    Compose { segments: Vec<CurveProgram> },
}

pub enum SectionProgram {
    Circle { radius: ScalarExpr },
    Ellipse { a: ScalarExpr, b: ScalarExpr },
    Superellipse { a: ScalarExpr, b: ScalarExpr, n: ScalarExpr },
    RoundedPolygon { ... },
    SplineLoop { control: Vec<Point2Expr> },
    Morph { start: Box<SectionProgram>, end: Box<SectionProgram>, law: FieldId },
}
```

The number of nodes, edges, branches, loops, stages, and nested regions is variable but bounded by a campaign complexity budget.

## 4.5 Field language

Alien geometry requires more than fixed primitives, but arbitrary black-box functions are scientifically dangerous. Use a typed, bounded field language.

```rust
pub enum FieldProgram {
    Constant { value: Quantity },
    Coordinate { axis: Axis },
    DistanceTo { region: RegionRef },
    BSpline1D { knots: Vec<f64>, control: Vec<Quantity> },
    BSpline3D { lattice: ControlLattice },
    FourierCylinder {
        axial_modes: Vec<AxialMode>,
        azimuthal_modes: Vec<AzimuthalMode>,
    },
    RadialBasis {
        centers: Vec<Point3>,
        weights: Vec<Quantity>,
        width: Length,
    },
    Cppn {
        graph: TypedExpressionGraph,
        output: FieldOutputType,
    },
    Warp {
        source: FieldId,
        displacement: VectorFieldId,
    },
    Compose {
        op: SmoothFieldOp,
        inputs: Vec<FieldId>,
    },
    Clamp {
        source: FieldId,
        min: Quantity,
        max: Quantity,
    },
}
```

Every field declares:

- unit,
- domain,
- range,
- smoothness class,
- maximum spatial frequency,
- Lipschitz bound,
- evaluation cost,
- deterministic canonical form.

No field is allowed to hide a neural model whose weights are not content-addressed.

## 4.6 Developmental grammar

A developmental grammar is what allows a compact genome to produce unfamiliar but coherent structures.

```rust
pub enum DevelopmentRule {
    Instantiate { module: ModuleId, at: FrameExpr },
    Repeat { count: UIntExpr, transform: TransformExpr, child: RuleId },
    Mirror { plane: PlaneExpr, child: RuleId },
    RotateCopies { count: UIntExpr, axis: AxisExpr, phase: ScalarExpr, child: RuleId },
    Grade { parameter: ParamRef, field: FieldId, child: RuleId },
    Split { edge: EdgeSelector, pattern: SplitPattern },
    Rejoin { ports: PortSelector, law: RejoinLaw },
    Subdivide { target: RegionSelector, rule: SubdivisionRule },
    Graft { donor: ModuleId, interface_map: InterfaceMap },
}
```

Expansion has explicit budgets:

- maximum nodes,
- maximum edges,
- maximum boolean operations,
- maximum field nodes,
- minimum feature size,
- maximum genus/cycle count if required,
- maximum compiler cost.

Exceeding a budget is an explicit rejection, not truncation.

## 4.7 Material and manufacturing genome

Initially, material can remain fixed, but the schema should support a manufacturing realization separate from nominal geometry.

```rust
pub struct ManufacturingProgram {
    pub process: ProcessRef,
    pub build_orientation: OrientationProgram,
    pub support_policy: SupportPolicy,
    pub machining_allowances: Vec<AllowanceRegion>,
    pub surface_finish_regions: Vec<SurfaceFinishRegion>,
    pub tolerance_model: ToleranceModelRef,
    pub inspection_plan: InspectionPlanRef,
}
```

Nominal geometry and manufacturing realization must have different hashes.

## 4.8 Strategy genes

The design genome may carry self-adaptive search parameters, but these must not affect physical identity:

```rust
pub struct StrategyGenes {
    pub operator_logits: BTreeMap<OperatorId, f64>,
    pub continuous_step_scales: BTreeMap<ParameterClass, f64>,
    pub topology_edit_rate: f64,
    pub grammar_growth_rate: f64,
    pub repair_budget: f64,
}
```

Strategy genes are excluded from the manufactured-subject hash and included in lineage identity.

---

# 5. GeometryIR: the center of truth

## 5.1 Required structure

```rust
pub struct GeometryIR {
    pub schema: ContentRef,
    pub units: UnitSystem,
    pub regions: BTreeMap<RegionId, Region>,
    pub surfaces: BTreeMap<SurfaceId, SemanticSurface>,
    pub transforms: BTreeMap<TransformId, Transform>,
    pub invariants: Vec<GeometricInvariant>,
    pub provenance: Vec<ExpansionEvent>,
}

pub enum Region {
    Fluid(FluidRegion),
    Solid(SolidRegion),
    Porous(PorousRegion),
    Void(DeclaredVoid),
}

pub struct SemanticSurface {
    pub left: RegionId,
    pub right: RegionId,
    pub role: SurfaceRole,
    pub construction: SurfaceProgram,
    pub immutable_binding: Option<ContractSurfaceId>,
}
```

## 5.2 Why an IR is mandatory

Without a shared IR, each backend interprets the genome independently and creates different designs.

The IR must:

- be fully expanded,
- contain no random choices,
- contain no unresolved names,
- use one internal unit system,
- carry semantic face IDs,
- preserve feature provenance,
- be deterministic under node-ID permutation,
- be canonicalizable and hashable.

## 5.3 Dual geometry backends

### Exact B-rep backend

Use for:

- fixed-interface proof,
- volume and mass,
- native STEP,
- solid FEA,
- manufacturing geometry,
- machining and inspection planning,
- exact boolean relationships.

### SDF/octree backend

Use for:

- connected-component analysis,
- local feature size,
- cavity detection,
- medial-axis thickness,
- support accessibility,
- morphology descriptors,
- fast collision and clearance,
- robust visualization,
- GPU batch screening.

Neither backend is authoritative alone. Qualification requires their agreement within declared tolerances.

## 5.4 Geometry identity chain

Every candidate receives distinct identities:

```text
raw_genome_sha256
normalized_genome_sha256
expanded_graph_sha256
geometry_ir_sha256
brep_sha256
sdf_octree_sha256
surface_mesh_sha256
volume_mesh_sha256
solver_case_sha256
objective_contract_sha256
descriptor_contract_sha256
verdict_sha256
```

A result is invalid if any upstream identity changes.

---

# 6. Independent F0 adjudicator

The F0 adjudicator must be in a different module and may not call convenience metrics emitted by the compiler without recomputing them.

## 6.1 Geometry gates

- Exact frozen-interface equality within declared tolerance.
- Exact terminal port shape, area, centroid, normal, and frame.
- Fluid-domain connectedness.
- Required inlet-to-terminal reachability.
- No unintended inlet-to-inlet shortcut.
- No trapped fluid cavity.
- No disconnected solid island.
- No non-manifold surface.
- Positive oriented volume.
- Minimum shell thickness.
- Minimum ligament.
- Minimum hydraulic diameter.
- Minimum gap.
- Curvature and Jacobian bounds.
- No self-intersection.
- No feature below mesh/manufacturing resolution.
- Consistent semantic face labels.

## 6.2 Manufacturing gates

- Full-surface overhang evaluation.
- Support generation and removal path.
- Powder evacuation connectivity.
- Drain and cleanout access.
- Tool reach and cutter radius where machining is required.
- Minimum as-built feature after tolerance erosion.
- Distortion allowance.
- Surface-finish reachability.
- Inspection accessibility.
- Build-orientation-specific trapped-volume audit.
- B-rep sew/heal report with no silent topology change.

## 6.3 Robust geometry

Run geometry under manufacturing perturbations:

- wall offset,
- shrinkage,
- port misregistration,
- surface roughness envelope,
- powder sintering closure,
- machining allowance,
- anisotropic build distortion.

A candidate must survive the declared tolerance distribution or worst-case envelope.

---

# 7. Variation operators for genuine invention

## 7.1 Continuous operators

- B-spline control-point displacement.
- Local centerline bend and twist.
- Cross-section morph.
- Area redistribution under volume/pressure constraints.
- Field coefficient perturbation.
- Frequency insertion/removal.
- TPMS wavelength, phase, and grading mutation.
- Local wall-thickness redistribution.
- Build-orientation mutation.
- Roughness or finish-region mutation when allowed.

## 7.2 Topology operators

- Split one edge into `n` typed branches.
- Rejoin branches with a selectable collector law.
- Insert a bypass.
- Insert a recirculation loop.
- Delete a low-value branch.
- Duplicate a module with phase shift.
- Nest an annular stage.
- Convert a chamber to a porous-exchange region.
- Add a counter-rotating stage.
- Add a flow shield.
- Insert a cleanout path.
- Graft a compatible module from another lineage.
- Replace a subgraph with an interface-equivalent subgraph.
- Fuse adjacent stages.
- Factor repeated geometry into a developmental module.
- Break a symmetry.
- Introduce controlled chirality reversal.

## 7.3 Developmental operators

- Add/delete a grammar rule.
- Change repeat count.
- Change transform sequence.
- Grade a module parameter with a field.
- Mutate CPPN node/function/connection.
- Add a modulation input.
- Duplicate a field subgraph.
- Compose two fields by smooth union/intersection.
- Reduce field bandwidth as a simplification mutation.

## 7.4 Crossover

Crossover must be typed and homologous.

1. Match nodes by innovation ID and semantic role.
2. Match subgraphs by compatible boundary signatures.
3. Exchange only interface-compatible subgraphs.
4. Preserve fixed-port bindings.
5. Perform field-graph crossover separately.
6. Record unmatched-gene policy.
7. Re-run canonical expansion and independent F0.
8. Never repair by substituting the baseline.

## 7.5 Repair

Repair is a separate, named operator.

```rust
pub struct RepairRecord {
    pub repair_operator: OperatorRef,
    pub original_sha256: String,
    pub repaired_sha256: String,
    pub normalized_distance: f64,
    pub modified_constraints: Vec<ConstraintId>,
    pub diagnostics: Vec<String>,
}
```

Rules:

- No repair of non-finite values.
- No repair of fixed interfaces.
- No repair that changes topology unless explicitly classified as topology repair.
- A large repair creates a new candidate identity.
- Repair rate and magnitude are campaign metrics.
- Promotion candidates must be re-generated without hidden repair.

---

# 8. Search architecture

## 8.1 Replace one mutation policy with a portfolio of emitters

Recommended emitters:

1. **Parametric CMA emitter** — local continuous refinement.
2. **Topology emitter** — split/merge/graft/loop edits.
3. **Developmental emitter** — grammar and field growth.
4. **Novelty emitter** — maximize distance in behavior space.
5. **Uncertainty emitter** — target epistemically uncertain regions.
6. **Model-disagreement emitter** — target disagreement among physics/surrogate models.
7. **Boundary emitter** — map feasible/infeasible transition surfaces.
8. **Robustness emitter** — optimize worst-case or lower-confidence performance.
9. **Adjoint emitter** — local continuous improvement where gradients are available.
10. **Simplification emitter** — preserve performance while reducing complexity.
11. **Ablation emitter** — create matched controls for causal understanding.
12. **Recovery emitter** — produce low-repair variants near promising invalid designs.

Use a bandit/controller to allocate evaluation budget by archive contribution, information gain, and cost.

## 8.2 Islands should have roles, not only different mutation scales

Example islands:

- conventional control,
- graph-topology exploration,
- field/developmental exploration,
- robust-design island,
- manufacturability-first island,
- mechanism-disagreement island,
- simplification island,
- adversarial-duty island.

Migration must preserve candidate identity and evidence status.

## 8.3 Archive design

Do not use the eleven hand-authored family names as the primary archive axis.

Use three archives:

### A. Structural/topological archive

Descriptors:

- graph cycle rank,
- branch/rejoin count,
- path-count entropy,
- mean and variance of path tortuosity,
- nested-region depth,
- topology signature/Betti vector,
- fluid-solid interface area ratio.

### B. Measured mechanism archive

From simulation:

- axial swirl-number profile,
- helicity magnitude and sign reversals,
- residence-time distribution,
- short-circuit fraction,
- dissipation-exposure distribution,
- pressure-work distribution,
- core-precession coherence,
- droplet Weber-number exposure,
- coalescence-opportunity integral,
- separation impulse by droplet size.

### C. Learned behavior archive

Train a frozen encoder on normalized flow/droplet fields and use its latent coordinates for illumination.

The encoder, scaling, and centroids are content-addressed and frozen before the main campaign.

## 8.4 Uncertain archive entries

An archive entry is not a single score. It contains:

- posterior objective distribution,
- feasibility probability,
- descriptor uncertainty,
- fidelity level,
- OOD status,
- re-evaluation count,
- evidence tier.

Cell replacement must account for uncertainty and not reward a lucky noisy realization.

## 8.5 Coevolving challenges

A POET-like challenge lane can generate difficult but physically valid duty scenarios inside the frozen service envelope. This is useful for robustness and stepping-stone discovery.

Rules:

- challenge generation may not expand the claim domain,
- final promotion uses predeclared duties,
- adversarial duties are kept separate from customer-weighted probabilities,
- scenario lineage and validity are content-addressed.

---

# 9. Scientific evaluator and evidence model

## 9.1 Typed observation

```rust
pub enum Observation<T> {
    Value {
        value: T,
        uncertainty: UncertaintyBudget,
        provenance: EvidenceRef,
    },
    NotEvaluated {
        reason: NotEvaluatedReason,
    },
    InvalidGeometry {
        reasons: Vec<GeometryFailure>,
    },
    NumericalFailure {
        stage: SolverStage,
        diagnostics: Vec<ArtifactRef>,
    },
    InfrastructureFailure {
        diagnostics: Vec<ArtifactRef>,
    },
    OutOfDomain {
        detector: ContentRef,
        distance: f64,
    },
    Inconclusive {
        reason: String,
        missing: Vec<EvidenceRequirement>,
    },
}
```

No missing value may be replaced by zero or a favorable default.

## 9.2 Evidence tiers

### E-2 — Syntax and identity

- strict schema,
- finite units,
- immutable interface binding,
- canonical hash,
- deterministic expansion.

### E-1 — Geometry and manufacturing

- dual-backend geometry agreement,
- F0 gates,
- tolerance perturbations,
- build and access checks.

### E0 — Conservative analytical bounds

- hydraulic network bounds,
- area/velocity bounds,
- pressure-loss bounds,
- structural envelope,
- dimensional checks.

### E1 — Coarse coupled screen

- carrier-flow CFD,
- conservation,
- short transient,
- coarse droplet parcels,
- cheap structural solve,
- uncertainty classified as screen.

### E2 — Qualified engineering simulation

- mesh/time/parcel convergence,
- turbulence-model challenge,
- full liner and controller,
- size-resolved particles,
- oil inventory closure,
- actual solid FEA,
- candidate-specific loads.

### E3 — High-fidelity challenge

- selected LES,
- PBM/breakup/coalescence,
- boundary/model envelope,
- manufacturing realization,
- independent solver or implementation cross-check.

### E4 — Physical validation

- measured as-built geometry,
- blinded paired comparator,
- independent preparations,
- held-out conditions,
- preregistered statistics.

Search may use E0/E1. Promotion claims require the campaign's declared E2–E4 contract.

## 9.3 Search score versus promotion verdict

Never let a search score imply qualification.

```rust
pub struct SearchAssessment {
    pub utility_posterior: UtilityPosterior,
    pub novelty: NoveltyVector,
    pub acquisition_value: f64,
    pub evidence_tier: EvidenceTier,
}

pub struct PromotionVerdict {
    pub requirement_set: ContentRef,
    pub subject: ScientificSubjectRef,
    pub outcome: PassFailIncomplete,
    pub supporting_evidence: Vec<EvidenceRef>,
}
```

## 9.4 Robust objective vector

Recommended promoted vector:

1. Maximize lower confidence bound of common compliant treated-water capacity.
2. Minimize upper confidence bound of energy per compliant treated volume.
3. Minimize upper confidence bound of outlet concentration across required duties.
4. Minimize worst-case regret over declared manufacturing and feed scenarios.
5. Minimize maintenance/cleaning burden.
6. Minimize manufacturing cost or complexity only as a secondary objective.

Novelty and alienness are not in the promoted objective.

## 9.5 Uncertainty budget

Every reported metric decomposes:

- input aleatory uncertainty,
- manufacturing uncertainty,
- geometry reconstruction uncertainty,
- mesh/time/parcel numerical uncertainty,
- model-form uncertainty,
- surrogate uncertainty,
- stochastic sampling uncertainty,
- experimental repeatability,
- between-build variability.

Promotion uses a predeclared conservative combination rule.

---

# 10. Surrogate and active-learning redesign

## 10.1 Current limitation

A small fixed feature vector and one main GP are insufficient for variable topology and multi-output physics.

## 10.2 Required model family

Use separate models for:

- each objective,
- each hard constraint,
- solver success,
- geometry compilation success,
- fidelity discrepancy,
- physical descriptor,
- runtime/cost,
- OOD probability.

## 10.3 Inputs

Combine:

- graph encoder of the typed fluid graph,
- field-program embedding,
- geometry descriptors,
- low-fidelity physics descriptors,
- scenario variables,
- fidelity indicator.

Never use human topology labels as the only topology representation.

## 10.4 Validation

Mandatory validation splits:

- leave-one-topology-cluster-out,
- leave-one-mechanism-cluster-out,
- leave-one-duty-out,
- temporal split by campaign wave,
- random split only as a weak diagnostic.

Calibrate uncertainty separately in-domain and near the feasibility boundary.

## 10.5 Acquisition

Use cost-aware constrained multi-objective acquisition:

```text
acquisition =
  expected Pareto/archive improvement
  × probability of hard feasibility
  × information gain
  ÷ expected evaluation cost
```

Add explicit quotas for:

- low-ranked audit candidates,
- solver-failure boundary,
- novel topology,
- model disagreement,
- conventional controls.

## 10.6 Failure handling

Numerical and infrastructure failure are not negative physical labels.

Train:

- a physical-performance model on valid observations,
- a solver-success classifier on all attempts,
- a recovery policy on diagnostic features.

---

# 11. Defining "alien invention"

## 11.1 Alienness is not visual weirdness

A design is not alien merely because it contains a gyroid, many branches, or an unusual name.

A meaningful alien invention must be novel in at least one of these spaces:

### A. Topological novelty

Distance in canonical graph/topology space:

- graph edit distance,
- cycle and path signatures,
- Betti numbers,
- adjacency spectrum,
- nested-region relationships.

### B. Geometric novelty

Distance in shape space:

- SDF embedding,
- curvature spectrum,
- thickness-field spectrum,
- surface-area distribution,
- morphometric descriptors.

### C. Physical novelty

Distance in measured behavior:

- helicity field,
- dissipation exposure,
- residence-time distribution,
- pressure-work map,
- droplet transport/collision behavior,
- separation mechanism profile.

### D. Causal novelty

Distinct ablation signature:

- removing feature A destroys benefit,
- phase reversal changes the expected mechanism,
- topology-matched controls do not reproduce the effect,
- the mechanism survives model and duty challenges.

## 11.2 Alien Invention Certificate

A candidate can receive an internal discovery label only when:

1. It lies outside the confidence hull of the reference design library in topology or measured mechanism space.
2. It passes geometry/manufacturing gates.
3. It has conservative performance improvement.
4. The improvement survives independent evaluator/fidelity challenge.
5. At least one causal ablation supports a distinct mechanism.
6. Complexity alone does not explain the novelty score.
7. The complete subject/evaluator/evidence chain is reproducible.

This certificate is separate from industrial promotion.

---

# 12. Intelligent agent layer

The intelligent design process should use agents, but agents must operate through typed contracts.

## 12.1 Roles

### Inventor agent

- proposes a typed graph/grammar patch,
- states a falsifiable mechanism hypothesis,
- identifies expected observables.

### Compiler agent

- expands and canonicalizes,
- reports unsupported semantics,
- never repairs silently.

### Red-team scientist

- proposes counterexamples,
- generates matched ablations,
- identifies confounders,
- chooses model challenges.

### Evaluation scheduler

- selects fidelity using evidence value per cost,
- preserves quotas and frozen budgets.

### Mechanism analyst

- compares fields and ablations,
- writes causal evidence,
- cannot change requirements.

### Claim auditor

- verifies every public statement against scoped evidence.

## 12.2 Agent output contract

An LLM may not emit arbitrary CAD and call it a design. It emits:

```rust
pub struct ProposedEdit {
    pub parent_subjects: Vec<SubjectRef>,
    pub operator_semantics: ContentRef,
    pub patch: TypedGenomePatch,
    pub hypothesis: FalsifiableHypothesis,
    pub expected_observables: Vec<ObservableSpec>,
    pub proposed_ablations: Vec<TypedGenomePatch>,
}
```

The deterministic pipeline decides whether it is valid.

## 12.3 Self-play

Use inventor-versus-falsifier self-play:

1. Inventor proposes a mechanism.
2. Falsifier proposes the cheapest decisive test.
3. Evaluator runs the test.
4. Evidence updates the operator policy.
5. Failed mechanisms remain in a searchable negative-results archive.

The judge is never an LLM.

---

# 13. Example of a design the v4 genome could represent

## Counter-rotating nested percolation separator

A truly open-ended genome could encode:

1. Two fixed feed ports.
2. Each feed splits into three typed helical branches.
3. Alternating branches inject opposite-sign axial pitch while preserving the required net angular momentum direction.
4. A nested annular chamber creates a low-shear protected core.
5. A graded diamond/gyroid peripheral region provides large interfacial area for coalescence without blocking the central vortex core.
6. A downstream rejoin collector suppresses short-circuit paths.
7. A phase-shifted recovery vane stage reverses local secondary flow but not the primary separation direction.
8. Dedicated powder/drain paths keep the geometry manufacturable and cleanable.

The current fixed JOIL genome can name or approximate pieces of this idea. Xenogen v4 would represent the actual path topology, regions, fields, interfaces, and causal modules.

Required adjudication:

- F0 graph reachability and trapped-volume audit.
- Dual B-rep/SDF agreement.
- Local feature/tolerance erosion.
- Water conservation and pressure controller.
- Helicity and dissipation fields.
- Oil mass closure.
- Size-resolved grade curve.
- Structural load transfer from candidate CFD.
- Ablations:
  - remove porous region,
  - reverse its phase,
  - remove counter-rotation,
  - collapse nested annulus,
  - preserve pressure loss with a conventional control.

Only the surviving mechanism receives scientific credit.

---

# 14. Implementation sequence

## PR-XG-001 — Freeze and expose identity divergence

- Add a report comparing Rust genome features with CadQuery support.
- Mark unsupported candidates `NotEvaluated`.
- Block qualification/export claims for unmatched subjects.

## PR-XG-002 — Remove baseline fallbacks

- Change all mutation/crossover APIs to `Result<BirthOutcome, Error>`.
- Log every failed attempt.
- Add CI assertion: `silent_baseline_fallbacks == 0`.

## PR-XG-003 — Correct structural verification

- Fix observed-order division.
- Add manufactured convergence tests.
- Replace tautological load-transfer check.
- Bind candidate-specific loads.
- Add structural pass/fail to constraint dominance.

## PR-XG-004 — Port Jarambit-style evidence core

- Typed observations.
- Scientific identity.
- Requirement-scoped verdict.
- Append-only event log.
- Operator semantics version.
- Parent lineage.

Generalize it to JOIL subjects and solvers.

## PR-XG-005 — Define GeometryIR v1

- Typed regions.
- Semantic surfaces.
- immutable interface bindings.
- exact units.
- canonical serialization.
- graph/permutation canonicalization.
- expansion provenance.

## PR-XG-006 — Build exact and SDF backends

- CadQuery/OpenCascade B-rep backend.
- Rust SDF/octree backend.
- differential agreement tests.
- content hashes.

## PR-XG-007 — Independent F0 adjudicator

- connectivity,
- cavities,
- frozen tail,
- terminal audit,
- thickness,
- flow gaps,
- support/powder/tool access,
- tolerance erosion.

## PR-XG-008 — Variable-length fluid graph genome

- typed nodes,
- typed edges,
- canonical graph hash,
- fixed-interface bindings,
- graph validity checks.

## PR-XG-009 — Field DSL and developmental grammar

- bounded B-spline/Fourier/RBF/CPPN fields,
- repeat/mirror/rotate/grade/split/rejoin,
- complexity budgets,
- exact expansion trace.

## PR-XG-010 — Topology-aware variation

- split/merge/graft/loop/nesting operators,
- innovation IDs,
- typed crossover,
- explicit repair,
- operator success statistics.

## PR-XG-011 — Physics-derived descriptors

- separate genotype/geometry/physics descriptors,
- field snapshot extraction,
- frozen learned descriptor encoder,
- CVT archive.

## PR-XG-012 — Multi-emitter QD scheduler

- emitter portfolio,
- bandit allocation,
- uncertain archive,
- cost ledger,
- reserved audit budget.

## PR-XG-013 — Multi-output, multi-fidelity surrogate

- graph/field encoder,
- objective and constraint heads,
- solver-success model,
- discrepancy model,
- OOD detector,
- topology-held-out validation.

## PR-XG-014 — Causal invention workflow

- automatic matched ablations,
- module attribution,
- mechanism challenge reports,
- Alien Invention Certificate.

## PR-XG-015 — Promotion gate

- common-duty robust objectives,
- full uncertainty budget,
- model challenge,
- as-built geometry,
- experiment bindings,
- claim admission.

---

# 15. Required test program

## 15.1 Representation tests

- Exact CT-35 raw-to-normalized-to-IR round trip.
- Node-ID permutation gives identical canonical hash.
- Module order permutation gives identical canonical hash.
- Inactive fields do not create duplicate identities.
- Distinct active geometry cannot collide under canonicalization.
- Unit conversions preserve geometry.
- Non-finite values always reject.
- Unsupported feature always reports `NotEvaluated`.

## 15.2 Operator tests

- No operator may emit the baseline as an error fallback.
- Every birth has parents, operator ID, and semantics version.
- Every repair has magnitude and changed constraints.
- Random-walk topology transition matrix is reported.
- Operators preserve fixed ports.
- Crossover rejects incompatible subgraph interfaces.
- Simplification does not change geometry above tolerance.

## 15.3 Geometry tests

Curated negative fixtures:

- trapped fluid cavity,
- trapped powder pocket,
- disconnected branch,
- non-manifold edge,
- zero-thickness sheet,
- self-intersecting sweep,
- tiny hidden throat,
- wrong terminal normal,
- shifted port,
- modified tail,
- unsupported B-rep healing,
- mesh geometry mismatch.

## 15.4 Differential tests

- B-rep vs SDF volume.
- B-rep vs SDF connected components.
- local thickness agreement.
- terminal-face agreement.
- surface mesh vs B-rep area.
- volume mesh boundary labels vs semantic faces.
- independent compiler implementation on a small oracle corpus.

## 15.5 Metamorphic tests

- rigid translation and rotation.
- graph node renaming.
- equivalent module factoring.
- duplicate inactive fields.
- unit rescaling.
- symmetry permutation.
- mesh refinement.
- parcel-count refinement.
- random-seed replay.

## 15.6 Physics tests

- mass, momentum, and energy closure.
- hydrostatics and straight-pipe oracles.
- rotating-flow analytical cases.
- light-droplet acceleration.
- terminal rise.
- tracer following.
- PBM mass conservation.
- Smoluchowski coalescence.
- breakup daughter conservation.
- turbulence dispersion well-mixed behavior.
- controller reachability.
- outlet recrossing handling.
- wall-hit and in-domain inventory.
- full oil closure.

## 15.7 Search tests

- equal budget across algorithms.
- all failed attempts counted.
- surrogate training cost counted.
- re-evaluation and retries counted.
- no hidden cache leakage.
- held-out topology ranking.
- fidelity rank-reversal monitoring.
- uncertain archive replacement tests.
- conventional-control retention.
- low-ranked audit quota.

## 15.8 Claim tests

- every number maps to evidence.
- every evidence packet maps to exact subject.
- stale evaluator invalidates verdict.
- stale objective invalidates verdict.
- stale descriptor invalidates archive identity.
- changed requirement set cannot reuse PASS.
- missing evidence yields INCOMPLETE, never PASS.

---

# 16. Acceptance criteria for Xenogen v4 foundation

The v4 foundation is ready for broad search only when all are true:

1. The CT-35 baseline round-trips exactly through the canonical IR.
2. Every active genome feature has an exact B-rep and SDF realization.
3. Unsupported features cannot receive performance or F0 PASS.
4. Every candidate has complete lineage and operator semantics.
5. No mutation/crossover path silently returns the baseline.
6. F0 independently measures all hard geometry metrics.
7. Candidate-specific structural loads are content-bound.
8. B-rep/SDF/mesh identities are chained.
9. Search descriptors are separated into genotype, geometry, and measured physics.
10. Archive entries carry uncertainty and fidelity.
11. Numerical failures are not physical labels.
12. Final audit uses independent seeds and frozen duties.
13. Tail metrics have declared statistical precision.
14. Promotion verdicts are requirement-scoped and machine-checkable.
15. Alienness cannot offset any hard constraint.

---

# 17. Immediate priority order

The highest-return order is:

1. **Stop identity divergence and silent baseline fallback.**
2. **Fix structural verification defects and candidate-load binding.**
3. **Port Jarambit's evidence/lineage core.**
4. **Create the canonical GeometryIR and independent F0 judge.**
5. **Move current mechanisms onto that IR.**
6. **Introduce the variable-length typed graph genome.**
7. **Add field/developmental programs.**
8. **Replace family labels with topology and measured-behavior descriptors.**
9. **Deploy multi-emitter uncertain QD.**
10. **Add causal ablation and an invention certificate.**

The most important sentence for the project is:

> JOIL should evolve executable physical programs, not a menu of exotic-looking parameter combinations.

That architecture can produce concepts the original developers did not anticipate while making it much harder for the system to fool itself.
