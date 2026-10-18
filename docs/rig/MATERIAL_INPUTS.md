# Material, process and as-built metrology inputs (V5-13 preparation)

Status: preparation only. This file lists what must exist before a physical claim can rely on
the hydrocyclone material passport. It places no order, contacts no vendor and supplies no
measurement. Every value below is unresolved until an owner supplies a sourced record; the
judges in `packs/hydrocyclone/judges` treat unresolved inputs as blockers, never as defaults.

Machine-readable records live in `packs/hydrocyclone/materials/`:

| File | Content |
| --- | --- |
| `ti64-lpbf-as-built.passport.json` | Process context, density and allowables, all unresolved except a legacy density proposal that cannot support a decision. |
| `ti64-lpbf-as-built.process-thresholds.json` | Manufacturing-gate thresholds, all unresolved. |
| `coupon-plan.json` | Six coupon/metrology records with testable rules and accountable external dependencies. |

## Why a material name is not enough

An allowable counts only when its evidence context equals the passport context for alloy and
product form, LPBF parameters, orientation, heat treatment/HIP, finish, temperature and
environment, and defect assumptions. Wrought-bar data, or a matching alloy name, does not
qualify an as-built fatigue or sour-service claim. The legacy constants in
`crates/joil-core/src/structure.rs` (yield, endurance, density) are provisional and are not
admitted by this passport.

## Open inputs and owners

| Input | Owner | Blocks | Needed for |
| --- | --- | --- | --- |
| Alloy grade, powder lot, product form | `materials_engineer` | physical candidate, commercial claim | every material claim |
| Machine, laser and scan parameters, layer thickness | `manufacturing_lead` | physical candidate, commercial claim | process identity |
| Build orientation and support strategy | `manufacturing_lead` | physical candidate | strength, fatigue, finish |
| Stress relief and HIP schedule | `manufacturing_lead` | physical candidate, commercial claim | allowables |
| Per-face finish classes and post-processing | `manufacturing_lead` | physical candidate | roughness, fatigue |
| Service temperature, fluid chemistry, absolute pressure | `process_owner` | physical candidate, commercial claim | fatigue, sour service |
| Defect (porosity, lack-of-fusion) assumptions | `materials_engineer` | physical candidate | fatigue |
| Printer build envelope and process-qualified geometric limits | `manufacturing_lead` | physical candidate | manufacturing gates |
| Tool, reamer and support-removal access limits | `manufacturing_lead` | physical candidate | machined orifice plan |
| Flow-derived equivalent sand roughness per finish class | `fluid_measurement_lead` | physical candidate, commercial claim | `ks` calibration |

## Coupon and metrology records

`coupon-plan.json` holds one record per kind. Each states its purpose, the process condition it
must share with the part, a testable acceptance rule, the uncertainty components that must be
budgeted, and the external dependency with its owner. Thresholds and specimen counts are
fields that stay unresolved: counts follow a prospective plan (V5-04), not a copied constant.

| Record | Establishes | External dependency (owner) |
| --- | --- | --- |
| Tensile | As-built yield per orientation and heat treatment | Accredited tensile test capacity (`materials_engineer`) |
| Fatigue | As-built endurance with the stated finish and environment | Fatigue test capacity in the declared environment (`materials_engineer`) |
| Density | Density bounds used by the mass judge | Calibrated balance and immersion reference (`materials_engineer`) |
| Porosity | Defect population behind the defect assumptions | Metallography or CT access (`materials_engineer`) |
| Roughness | Ra per finish class and the Ra to `ks` calibration | Flow-derived `ks` measurements (`fluid_measurement_lead`) |
| CT calibration | Dimensional accuracy and wall detectability | Traceable calibration artifact and scanner access (`manufacturing_lead`) |

## Rules the judges enforce

- Ra and equivalent sand roughness `ks` are different observables. `ks` follows only an admitted
  calibration for the face's own finish class, with at least three points and a calibrated Ra
  range. A universal ratio such as `ks = 3.1 * Ra` is not a calibration.
- Changing orientation or any face finish changes the surface-variant identity; flow, structure
  and export evaluations that carry another identity are stale.
- CT voxel pitch is a sampling interval, not an accuracy. Dimensional accuracy needs a
  calibration artifact (with a source), edge localization and repeatability, combined in
  quadrature with the voxel quantization term. A wall is detectable only if it spans the
  required number of voxels and exceeds the expanded accuracy.
- A machined feature (for example the overflow orifice) needs its own process and tool-access
  plan. That exception does not qualify any printed channel.
- Mass is admitted density times the volume of the accepted solid that CFD, FEA and the print
  share, cross-checked against a tetrahedral volume mesh. Nominal printed, support/allowance,
  machined-removed and weighed mass are separate quantities; an unplanned term is unknown, not zero.

## Not covered

No physical allowable, density, threshold or tolerance is admitted here. Sour-service
eligibility, as-built fatigue and structural qualification remain blocked until the sourced
records above exist.
