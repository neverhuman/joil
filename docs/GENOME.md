# Design language and geometry contract (as implemented)

This file describes the genome that the code **implements**. It reflects the integration branch
`feat/joil-v4-truth-evidence` at merge `bf9155b` (TR-05/TR-06 merged). The open-ended successor (typed
developmental field graph, DesignIR, dual backends) is a separate design marked **PROPOSED** in
[`docs/design/GENOME_V4.md`](design/GENOME_V4.md). Nothing in that file is implemented unless this file says so.

If this file and the code disagree, the code is authoritative and this file is wrong.
`tests/test_doc_genome.py` checks that every serialized field, family label, descriptor and operator named
here exists in the code, and the reverse. A **Pending** cell naming a workplan item marks a change in flight.

## 1. What exists

- **Baseline seed** `ct35-parametric-v1` ([`baselines/ct35/genome.json`](../baselines/ct35/genome.json)) is the
  latest supplied CT-35 reconstruction. Every station and area derives from named physical ratios.
  [`crates/joil-core/src/geometry.rs`](../crates/joil-core/src/geometry.rs) and
  [`cad/ct35/build.py`](../cad/ct35/build.py) derive and check its dimensions independently.
- **`DesignProgram`** ([`crates/joil-core/src/program.rs`](../crates/joil-core/src/program.rs)) is a fixed Rust
  struct with seven optional layers: inlet ramp, inlet split, chamber modes, cone profile, skimmer, helical
  ribbon, TPMS insert. It is a **bounded macro language**:
  - The number and kind of features are fixed by the type.
  - The jet count is 2, 4, 6 or 8.
  - The topology label is computed from which layers are active (§4).
  - It cannot express branch/rejoin networks, internal solids, non-star-shaped cavities, or any feature not
    already a field.
  - The review intake ([synthesis](reviews/2026-10-15-SYNTHESIS.md), S1 §3.1, S2 §4.1, S4 §2.1) treats it as
    the legacy lane.
- **Accepted schema pairs**: `(2, "ct35-exotic-head-v2")`, `(3, "ct35-exotic-head-v3")` and
  `(4, "ct35-legacy-macro-v4")`.
  - Since TR-05, `normalize()` keeps the declared pair instead of rewriting it to v2.
  - `migrate_v2_to_v4()` is explicit and returns a `MigrationReceipt`.
  - The v4 label changes the envelope only. Its receipt states that no developmental graph is synthesized and
    that geometric equivalence is not audited. It is **not** the proposed v4 genome.
- **Exact CAD** ([`cad/program/exotic_graph.py`](../cad/program/exotic_graph.py)) implements only schema v2.
  It rejects `ribbon_cascade` and `tpms_insert` as unknown keys and coerces `branches_per_port = 4` to 2.
  [`cad/program/wall_field.py`](../cad/program/wall_field.py) compiles only the older
  `ct35-chamber-wall-field-v1` chamber modes. One genome can therefore mean different geometry on different
  paths. IR-02/IR-03 own the support matrix (`docs/IR_SUPPORT.md` once it lands).

## 2. Field reference

Columns:

- **Accepted**: what `normalize()` accepts. Anything else is a hard error.
- **Inactive**: the canonical value written when the layer is off.
- **Ring mesh**: the gene changes the Rust watertight fluid mesh `Phenotype3D::build_fluid_mesh` (axial rings of
  `R(θ,z)`).
- **CFD surface**: the gene changes `MultiPatchSurface::from_program`, the five-patch STL actually meshed in
  OpenFOAM (`wall`, `inlet1`, `inlet2`, `overflow`, `underflow`).
- **Python B-rep**: what `exotic_graph.py` does with the gene.
- **Descriptors**: the gene feeds the gene-derived `MechanismDescriptors` (§5). Those descriptors feed the F1
  score, the MAP-Elites key and the CUDA evaluator.

Continuous values are rounded to 4 decimals in field units (§3).

| Field | Accepted | Inactive | Ring mesh | CFD surface | Python B-rep | Descriptors | Pending |
|---|---|---|---|---|---|---|---|
| `schema_version` | 2, 3 or 4, paired with `family` | - | - | - | v2 only | - | - |
| `family` | `ct35-exotic-head-v2` / `-v3` / `ct35-legacy-macro-v4` | - | - | - | v2 only | - | - |
| `topology_class` | any string; overwritten by `classify_topology()` | - | - | - | accepted | - | IR-06 (derive taxon, drop from serialized subject) |
| `baseline_file_sha256` | must equal SHA-256 of `baselines/ct35/genome.json` | - | - | - | checked | - | - |
| `inlet_ramp.axial_drop_mm` | [0, 8] mm | 0 | scales inlet-boss bump (×(1−0.04·drop/8)) | no | yes | yes | - |
| `inlet_ramp.radial_slot_factor` | [0.72, 1.05] | 1.0 | scales inlet-boss bump | no | yes | yes | - |
| `inlet_ramp.superellipse_exponent` | [2, 4] | 2.0 (also forced when ramp inactive) | no | no | accepted, not used | yes | - |
| `inlet_split.enabled` | bool | false | no | no | yes | yes | - |
| `inlet_split.vane_thickness_mm` | [1.0, 1.8] mm when enabled | 1.2 | no (arithmetic clearance/volume only) | no | yes | no | - |
| `inlet_split.vane_z_fraction` | [0.35, 0.65] when enabled | 0.5 | no | no | used only when 2 branches | no | - |
| `inlet_split.branches_per_port` | 1–4; enabled requires 2–4 (4, 6, 8 jets) | 1 | no (arithmetic hydraulic diameter only) | **no: always two inlet ducts** | 2 or 3; 4 coerced to 2 | yes | IR-02, IR-05 |
| `inlet_split.azimuthal_phase_deg` | [0, 45] deg when enabled | 0 | no | no | accepted, not used | yes | - |
| `chamber_modes[].azimuthal_order` | 2–8; ≤ 8 modes | list empty | yes (z ∈ [20, 65] mm) | yes | yes | yes | - |
| `chamber_modes[].cos_mm` | finite; Σ amplitude ≤ 1.8 mm | - | yes | yes | yes | yes | - |
| `chamber_modes[].sin_mm` | finite; Σ amplitude ≤ 1.8 mm | - | yes | yes | yes | yes | - |
| `chamber_modes[].twist_rad` | \|twist\| ≤ 2π | - | yes | yes | yes | yes | - |
| `cone_profile.mode1_mm` | Σ\|mode\| ≤ 1.6 mm | 0 | yes (z ∈ [72, 166] mm) | yes | yes | yes | - |
| `cone_profile.mode2_mm` | Σ\|mode\| ≤ 1.6 mm | 0 | yes | yes | yes | yes | - |
| `cone_profile.mode3_mm` | Σ\|mode\| ≤ 1.6 mm | 0 | yes | yes | yes | yes | - |
| `skimmer.length_mm` | [0, 10] mm; 0 = off | 0 | no (arithmetic volume/clearance only) | no | yes | yes | IR-02 |
| `skimmer.outer_radius_mm` | [1.8, 4.0] mm when on | 2.4 | no | no | yes | no | - |
| `skimmer.capture_flutes` | 0, 4, 6 or 8 | 0 | no | no | accepted, not used | yes | - |
| `ribbon_cascade.vane_count` | 2, 3, 4 or 6; 0 = off (omitted from JSON) | 0 | wall-radius lobes (cos⁴), not vane surfaces | wall-radius lobes | **rejected (unknown key)** | yes | IR-02, IR-03 |
| `ribbon_cascade.axial_span_mm` | [8, 48] mm | 0 | lobe extent z ∈ [12, 12+span] | same | rejected | yes | - |
| `ribbon_cascade.helical_twist_rad` | [−4.5, 4.5] rad | 0 | lobe phase | same | rejected | no | - |
| `ribbon_cascade.thickness_mm` | [1.10, 1.80] mm | 1.2 | no (clearance and arithmetic volume only) | no | rejected | no | - |
| `ribbon_cascade.radial_chord_mm` | [1.50, 4.20] mm | 2.0 | lobe depth 0.35·chord | same | rejected | no | - |
| `tpms_insert.basis` | `None`, `Gyroid`, `SchwarzP`, `Diamond`, `CylindricalGyroid`; `None` = off (omitted) | `None` | outer-wall corrugation (4θ × axial wave), not a lattice | same | **rejected (unknown key)** | yes | IR-02, IR-03 |
| `tpms_insert.radial_inner_fraction` | [0.58, 0.85] | 0.68 | **no** | **no** | rejected | yes | IR-02 |
| `tpms_insert.axial_start_mm` | [15, 55] mm; end − start ≥ 16 | 24 | corrugation extent | same | rejected | no | - |
| `tpms_insert.axial_end_mm` | [50, 95] mm | 64 | corrugation extent | same | rejected | no | - |
| `tpms_insert.radial_wavelength_mm` | [6, 14] mm | 8.5 | no (arithmetic pore estimate only) | no | rejected | no | - |
| `tpms_insert.axial_wavelength_mm` | [6, 14] mm | 9.0 | corrugation wavelength | same | rejected | no | - |
| `tpms_insert.sheet_thickness_mm` | [1.20, 2.20] mm | 1.35 | corrugation amplitude 0.45·t | same | rejected | no | - |

Consequences for current claims:

- The CFD surface reflects only the chamber modes, the cone profile, and the ribbon/TPMS wall perturbations,
  on CT-35's two inlet ducts.
- A `trifurcated-6jet-vortex-head` or `octa-branch-helical-manifold` result is therefore a CFD result for a
  two-inlet geometry. IR-05 owns the relabel.
- The "peripheral TPMS shell" is a corrugation of the outer wall. `radial_inner_fraction` has no geometric
  effect, so "the vortex core is unobstructed" holds trivially: no internal structure exists.
- Inlet split, skimmer, `superellipse_exponent`, `azimuthal_phase_deg` and `capture_flutes` change the score
  through the descriptors without changing the CFD subject. These are the "label-only genes" of S1 §13.2.

## 3. Canonical identity and quantization

- `canonical_sha256()` is the SHA-256 of `serde_json::to_vec(normalize(p))`.
  - Field order is the struct order.
  - Inactive ribbon and TPMS layers are omitted (`skip_serializing_if`).
  - The derived `topology_class` label and the schema/family pair are part of the hash.
- Continuous genes are rounded by `round4` to 1e-4 field units (mm, rad, deg or dimensionless) during
  `normalize()`. Descriptors are rounded to 1e-6. This is a **declared quantization**, not exact binary64
  identity. The code has no machine-readable numeric contract yet (synthesis G-04).
- Inactive genes are canonicalized: a disabled split resets vane parameters to defaults, a zero-length skimmer
  resets radius and flutes, and an inactive ramp forces `superellipse_exponent = 2.0`. Inactive copies
  therefore share one hash.
- The genotype hash binds the program only. It does not bind the compiled mesh, the CFD surface, the
  compiler version or the solver case. EV-02 introduces that identity chain.

## 4. Derived label: `topology_class`

`classify_topology()` assigns one of eleven labels from gene activation. It measures no geometry or topology
(S1 §3.3, S3 P0-12). The rules are evaluated top to bottom; the first match wins.

| Label | Rule (first match) |
|---|---|
| `cylindrical-gyroid-coalescer` | `tpms_insert.basis` is `Gyroid` or `CylindricalGyroid` |
| `schwarz-p-rectifier-head` | `tpms_insert.basis` is `SchwarzP` or `Diamond` |
| `octa-branch-helical-manifold` | split enabled and `branches_per_port == 4` |
| `counter-fluted-recovery-vortex` | ribbon active with `vane_count ≥ 3` and `capture_flutes ≥ 4` |
| `baseline-ct35` | none of ramp (drop > 0.25 or \|slot − 1\| > 0.01), split, chamber modes, cone (any \|mode\| > 0.05), skimmer (length > 0.25) is active |
| `trifurcated-6jet-vortex-head` | split active with `branches_per_port == 3` |
| `hybrid-omni-exotic` | ≥ 3 of those five active, otherwise |
| `bifurcated-quad-manifold` | split active with `branches_per_port == 2` and fewer than 3 of the five layers active |
| `spiral-rifled-vortex-head` | chamber modes active |
| `contoured-s-cone-skimmer` | cone or skimmer active |
| `helical-pitched-ribbon` | ramp only (despite the name, no ribbon) |

Known artefacts:

- Ribbon cascades are not counted among the five layers. A ribbon-only design is labelled `baseline-ct35`.
- The 6-jet label is applied on gene values alone, whatever the CFD surface contains (§2).
- Earlier WHITEPAPER and README revisions used family names that do not exist in code (synthesis DOC-F03):
  `baseline-colman-thew`, `quad-spiral-manifold`, `helical-ribbon-core`, `s-cone-fluted-finder`,
  `double-vortex-skimmer`, `scalloped-parabolic-chamber`, `axisymmetric-stepped`, `twin-tangential-volute`
  and `quad-entry-involute-head`.

IR-06 replaces the label with a `PhenotypeTaxon` derived from compiled geometry. The eleven labels become seed
recipes and regression fixtures (S4 §2.1).

## 5. Gene-derived mechanism descriptors (priors, not observations)

`descriptors()` computes eight dimensionless numbers **from gene values**, using hand-chosen coefficients.
For example, an active TPMS layer multiplies `shear_mitigation_factor` by 0.86, and branch count raises
`azimuthal_symmetry_order`. These numbers are the author's hypotheses about each mechanism. Uses:

- the F1 reduced-order evaluator (`evolve.rs`), which computes Cout, Δp, shear and short-circuit from them;
- the MAP-Elites cell key `(topology_class, swirl_bin, shear_bin, shield_bin)`;
- the CUDA candidate evaluator inputs;
- the surrogate feature vector.

Results that depend on them are a search over a prior-laden screen, not physical evidence (S1 §3.4, S5 P0.5).
IR-07 moves them to an inventor-only module as `PriorDescriptors`.

| Descriptor | Gene inputs |
|---|---|
| `swirl_flux_index` | `radial_slot_factor`, `superellipse_exponent`, ribbon `vane_count`/`axial_span_mm` |
| `axial_pitch_sin` | `axial_drop_mm` |
| `azimuthal_symmetry_order` | split `enabled`/`branches_per_port`/`azimuthal_phase_deg`, chamber-mode amplitude, TPMS `basis` |
| `shear_mitigation_factor` | `axial_drop_mm`, `mode1_mm`, `radial_slot_factor`, `superellipse_exponent`, `branches_per_port`, TPMS and ribbon activity |
| `cone_accel_persistence` | `mode1_mm`, `mode2_mm`, `mode3_mm` |
| `core_shield_ratio` | skimmer `length_mm`, `capture_flutes` |
| `wall_rifling_stabilization` | chamber-mode amplitude/twist sign/order, TPMS `radial_inner_fraction`, ribbon `axial_span_mm` |
| `hydraulic_resistance_factor` | `axial_drop_mm`, `mode1_mm`, chamber-mode amplitude, `branches_per_port`, `radial_slot_factor`, TPMS `basis`/`radial_inner_fraction`, ribbon `vane_count` |

## 6. Variation operators

At `bf9155b`, every operator ends with `normalize().unwrap_or_else(|_| baseline_ct35())`. A failed edit
silently returns the CT-35 baseline: 24 sites in `program.rs`, plus `surrogate.rs` and `evolve.rs`.

- TR-01 (lane commit `9118908`, not yet merged) changes the operators to return
  `Result<VariationOutcome, VariationError>` with edit scripts and repair records.
- TR-02 removes the remaining fallbacks.

This table updates when those merge.

| Operator | What it does | Pending |
|---|---|---|
| `seed_archetypes` | hand-authored Wave-1 archetype programs | - |
| `seed_wave2_archetypes` | hand-authored Wave-2 archetype programs (ribbon/TPMS/8-jet) | - |
| `mutate_diffeomorphic_spline` | clamped Gaussian steps on cone modes (rescaled to Σ ≤ 1.48 mm) and chamber modes; adds one mode with p = 0.55 if none. Clamping, not a diffeomorphism check | TR-01 |
| `mutate_branch_topology` | steps ramp genes; toggles split (p = 0.25); redraws `branches_per_port` ∈ {2, 3, 4} (p = 0.3); adds, removes or steps the ribbon layer | TR-01 |
| `mutate_tpms_grading` | activates a random basis if inactive, deactivates with p = 0.2, otherwise steps fraction, wavelengths and sheet thickness | TR-01 |
| `crossover_homologous_graph` | per-layer copy from either parent plus scalar blending of fixed fields; **not** graph crossover (S2 §4.7) | TR-01, TR-12 |
| `crossover` | `crossover_homologous_graph` followed by `mutate` | TR-01 |
| `mutate` | chains `mutate_diffeomorphic_spline`, `mutate_branch_topology`, `mutate_tpms_grading` (p = 0.22) and a skimmer step | TR-01 |
| `mutate_conventional_axisymmetric` | mutation restricted to axisymmetric genes (arm B2) | TR-01 |
| `sample_random_exotic` | independent uniform draws per field with per-layer activation probabilities (arm `RandomLhsExotic3d`; not a Latin hypercube, synthesis G-03) | TR-01 |
| `sample_stratified_11family` | uniform draw targeting one of the eleven labels | TR-01 |
| `init_wave3_de_novo_gen0` | Wave-3 generation 0: the baseline plus mild ramp/cone/skimmer perturbations; every 5th member has a 2-branch split (4 jets), every 7th a chamber mode | TR-01 |
| `mutate_wave3_progressive` | clamped zero-mean steps on continuous genes, plus single-step split transitions off ↔ 2 ↔ 3 ↔ 4 branches; ribbon and TPMS layers can toggle only while the split is enabled | TR-01 |

The "expressiveness" audits `audit_11family_genome_expressiveness` and `audit_11family_operator_random_walk`
(220/220) are closed-catalog validity tests. They show that samples of the authored families pass the authored
gates; they do not show open-ended expressiveness (S5 P0.8). TR-14 renames them.

## 7. Fixed contract for the first research lane

- z=0 is the inner end-wall face; +z is downstream. CAD uses mm; solver geometry uses metres with a recorded transform.
- D2=35 mm, D1=70 mm, L1=70 mm, reducing-cone half angle=10°, fine taper length=770 mm, D3=17.5 mm, tail length=700 mm.
- Baseline overflow bore 1.4 mm, CFD bore length 2.8 mm and plenum R6×20 mm. The plenum is not printed hardware.
- Twin rectangular inlets, each 120.26409377023427 mm², width=7.754485597711633 mm and height=15.508971195423266 mm. A baseline inlet roof at z=0 is a reconstruction assumption.
- Inlet1 CFD face x=-60, outward -x, y in [R1-w,R1], inward velocity +x. Inlet2 is a 180° copy with outward +x and inward -x. In this latest file both generate negative angular momentum about +z. The older ZIP has the opposite convention; do not infer sign from the older drawing.
- Hardware inlet faces are at x=±47 mm. CFD feed extensions and printed port faces are distinct surfaces.
- The reducing cone ends at z=169.24743184330993 mm and the head at z=186.74743184330993 mm. Freeze the final 17.5 mm collar and the downstream taper/tail; evolve only the upstream interior and the compatible solid.
- Full fluid inventory, including CFD extensions/plenum, is about 1,105,802.5227442954 mm³. V/Q at 4 m³/h is about 0.9952 s. This is not an RTD, nor the earlier HTML's 0.258 s value.

Fluid and solid domains are complementary only within the actual insert envelope. CFD extensions are tagged
synthetic hydraulic boundary devices and are excluded from mass, fit and structural hardware. Printed head and
CFD bore lengths in the supplied files differ slightly; distinguish them rather than asserting identical wet
geometry. Mapped loads require the shared hardware wetted faces and the exact external pressure environment.

## 8. F0 checks: required versus implemented

| Required check | Status at `bf9155b` | Owner |
|---|---|---|
| Closure, orientation, positive volume | implemented on the ring mesh (edge census, signed volume) | - |
| Exact terminal area, normal, position | **asserted**: copied from baseline dimensions, not measured | IR-08 |
| Frozen collar/tail unchanged | **asserted** `frozen_collar_and_tail_diff_mm3: 0.0` | IR-08 |
| Trapped cavities | **asserted** `trapped_cavities: 0`, trusted by `redteam.rs` | IR-08 |
| Minimum wall, gap, hydraulic diameter | arithmetic over gene values; one conflated `minimum_wall_clearance_mm` | IR-08, IR-09 |
| Section Jacobian | sampled over chamber modes only | IR-09 |
| Self-intersection | not checked independently (ring mesh cannot fold radially) | IR-08 |
| Overhang, span, powder removal | scalar overhang estimate in `structure.rs`; removal paths not checked | §6 follow-on |
| Independent B-rep/voxel agreement | not implemented | IR-08 |

Gate errors must report geometry invalid, manufacturing infeasible, kernel failure or unknown separately.
Never repair a changed interface without declaring a new subject.

## 9. Representation challenge corpus (planned)

Planned cases, none implemented yet (IR-12 builds the negative corpus):

- baseline zero-mutation round trip;
- translated/rotated SI-equivalent subject;
- asymmetric dual feed;
- smooth helical entry;
- branch/rejoin manifold;
- obstructed dead pocket;
- trapped powder cavity;
- near-touching passages;
- nonsewable B-rep;
- spline fold;
- tiny aperture;
- unknown schema/operator;
- inactive-gene duplicates.

Use independent B-rep and voxel/SDF measurements, with convergence where discretized. Novel geometry must earn
an applicability check before search trusts its CFD prediction.
