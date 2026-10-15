# Source review and dispositions

All 17 listed Downloads files were found, copied unchanged to ignored `.private/source/`, hashed and studied. Both ZIPs have identical SHA-256; their members, text/code, reports and images were inspected. Raw provenance remains private. Public files contain selected research parameters and original implementation, not the supplied conversations or customer context.

| Input | Disposition |
|---|---|
| Latest baseline engineering Markdown | Primary supplied research intent; adopt CT-35 ratios, head focus, full liner, pressure/split comparison. Correct concentration, breakup shortcut, inference and provisional fit rules below. |
| `ct35_fluid_domain.stl` | Independently inspect triangles, patch identities, topology, normals, dimensions and volume; retain as supplied preview/reference, not an accepted volume mesh. |
| `ct35_baseline_params.json` | Latest reconstruction's parameter/provenance authority; retain exact values and independently derive geometry. |
| `README.md` | Input package usage/provenance; preserve scale, roles and unexecuted STEP caveat. Correct its `locationInMesh` example from mm to solver metres. |
| `ct35_baseline.py` | Read full generator; use analytic/independent surface/SDF comparison and supplied reference geometry. Preserve original privately. |
| `ct35_head_cartridge.stl` | Independently inspect geometry; 0.7 mm voxel preview may alias the 1.4 mm bore. Do not use preview as precision manufacturing definition. |
| `ct35_cadquery_step.py` | Read full script; originally unexecuted, separately hard-coded dimensions. Run in an isolated environment and replace parameter duplication with public JSON-driven builder. |
| `ti1.md` | Early impeller application proposal; no impeller scope. Reject generic titanium chemical/H2S claims as qualification evidence. |
| `ti2.md` | Adopt AM/process/material uncertainty and independent tests in principle. No fatigue knockdown or unrelated load/market value transfers. |
| `ti3.md` | Adopt task-based metrics, expressive connectivity, equal comparators and physical challenge of models. No aesthetic gate. |
| `ti4.md` | Historical opportunity ranking only; no downhole or universal Grade-5 sour-service eligibility. |
| `ti5.md` | Retain pipeline concept and convergence; superseded geometry, solver-name mixture, VOF oil-core and runtime promises. |
| `ti6.md` | Retain throughput measurement intent; superseded 40 mm/vortex-finder geometry and confident scaling claims. |
| `ti7.md` | Adopt light-drop force verification, full domain, common-duty scoring, uncertainty and honest compute. Its dimensions/duty belong to older HC35-B0, not latest CT-35. |
| `HC35_Simulation_HPC_Engineering_Spec.html` | All 26 sections studied. Main methodology source; earlier baseline dimensions, 200 kPa/3%/20 mg/L duty and turnover calculation are not imported. |
| `ct35_baseline_rev1.zip` | Archive as separate circular-inlet/2 mm-orifice/0.75° taper/762.6 g variant; inspect all member code/reports/images/meshes. |
| `ct35_baseline_rev1 (1).zip` | Byte-identical duplicate. Same disposition, no second independent corroboration. |

## Material corrections

1. **Three geometry families.** Latest CT-35: 70 mm head/D2=35, rectangular feeds, Do=1.4 mm, 770 mm fine taper, full liner 1.639 m. ZIP: circular Ø12.37 feeds, Do=2 mm, ~668.4 mm taper, 1.538 m wetted liner. Earlier HTML: 35 mm chamber, 6° cone, 2.45 mm reject/insertion and ~0.503 m full device. Never combine their reference values.
2. **Concentration basis.** Latest Markdown omits water-split denominator when G is raw captured oil mass. Use `Cu=Ci*(1-E)/(1-r)` or reduced-efficiency equivalent. Passive tracer must give Cu=Ci.
3. **Pressure fairness.** Constant-Euler flow rescaling is an initial guess; solve at matched pressure and split. Do not skip a confirming solve merely because flow correction is <2%.
4. **Capacity is not monotonic.** Swirl and breakup compete. Common-flow scenario intersection and tested duty band replace independently maximized or monotone-assumed capacity.
5. **Breakup history.** A Hinze size ceiling at the inlet does not represent rate, exposure, daughter mass, downstream breakup/coalescence or uncertain dissipation. Treat as diagnostic until calibrated against inlet/outlet spectra and holdouts.
6. **Insufficient loose accuracy floors.** Five percentage points grade agreement at Ci=500 and r=.02 implies ~25.51 mg/L potential Cu discrepancy. That cannot prove a small gain near the research limit. Allocate observable-specific error budgets.
7. **Statistics.** A point gain≥10% plus confidence lower bound>0 does not establish gain≥10% with that confidence. Three repeats are a variance pilot, not automatic statistical adequacy. Use independent preparations/builds and endpoint uncertainty.
8. **Nominal V/Q.** Latest fluid V/Q at 4 m³/h is ~0.9952 s. Earlier 0.258 s applies to another geometry/flow. Actual residence distribution must be measured/tracked.
9. **Coordinate sign.** Latest feeds produce negative swirl around +z; ZIP produces positive swirl. Each case derives injection signs from actual outward normals, never a label copied between variants.
10. **Fit conflict.** Assumed radial envelope 45 mm cannot contain latest inlet port at x=47 and y approaching35 mm, even before shell thickness. Fit is unresolved until a real envelope and port keep-in are provided. No silent exemption.
11. **STEP/preview distinction.** Supplied B-rep was not executed, shell geometry differs from voxel offset at corners, and CFD extensions are not hardware. STEP volume/validity/solid count/reimport and precision bore checks are actual required work, not inherited PASS labels.
12. **Mesh coordinate error.** Input README's `locationInMesh (0 0 35)` is in CAD mm. A metre solver requires `(0 0 0.035)` for the intended point. Scale all surface vertices and dimensional settings consistently.
13. **Regulatory context.** OSPAR's averaging/method and US oil-and-grease analyte differ from a simulated dispersed-oil snapshot. No site requirement inferred. Research 30/20 mg/L duties stay explicitly provisional.
14. **Sour service/AM.** Alloy name, wrought listing or corrosion anecdote cannot qualify an LPBF build/process/product form for actual chemistry/stress. Acquire applicable standard/operator requirements and material/process evidence.
15. **Software/GPU.** Foundation 14 and OpenCFD v2606 differ. v2606 officially announces GPU offloading; it deserves a separate capability/parity benchmark, not a presumed drop-in Foundation oil-workflow acceleration. Pinned CPU reference remains independently runnable.

## Executed CAD findings

Independent trimesh audit measured latest supplied head STL signed volume **−128,341.9426 mm³**. It is watertight and consistently wound, but inward. The supplied generator reverses marching-cubes faces and reports `abs(volume)`, concealing orientation. Original bytes remain unchanged. The mesh-audit command returns nonzero on this known-negative source; detecting it is not an original-input PASS.

The previously unexecuted supplied STEP generator ran successfully in isolated CadQuery 2.7.0. Fluid volume agrees with the analytic reconstruction; head volume is ~128.835 cm³, approximately 0.36% above the 128.372 cm³ SDF convergence reference because its sharp boss corners define a different solid. New public builder derives dimensions from the genome, reproduces this head, and audits fluid volume, terminal faces, preview winding, single solids, STEP reimport and metre scaling. New head preview is outward and positive-volume. Final export audit checks STEP declares millimetres; SI STL and glTF are separate outputs with explicit metre manifests. A prior coordinate-only scaled STEP check was superseded because the default writer still declares millimetres. Previews are not accepted labelled CFD boundaries. Hardware 3 mm end wall and CFD 2.8 mm bore are recorded separately.

## Jarambit study boundary

Read-only inspection of xbabe2 repo status, coordination rules, technology/status docs, genotype/search/MC/evidence implementations and source snapshot. Its existing dirty coordination/todo files belong to others and were untouched. Local science release/test plans supplied additional evidence/ownership methodology. Joil uses these ideas with new application contracts; no shared source dependency, copied knife physics or new knife objective.
