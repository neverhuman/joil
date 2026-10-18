# Vortex and PDR validation status (V5-15)

Status: **blocked**. No vortex or PDR claim is admitted. The software judges and the registers below exist. No CT-35 measurement, literature comparison or reference-data admission has been completed.

Judges: `packs/hydrocyclone/judges/src/validation.rs` (tests: `packs/hydrocyclone/judges/tests/validation.rs`, all SYNTHETIC fixtures). Artifacts: `experiments/validation/v5/`.

## Per-task status

| Task | State | What exists | What blocks it |
|---|---|---|---|
| V5-15-01 accession | blocked | Register of Bai, Young, Zhou, Hsieh with unresolved inputs (`accession_register.json`); accession judge | Original Bai 2009 and Young 1994 apparatus/data, uncertainty and diameter definitions are not accessed. Zhou and Hsieh have no source in the repository. |
| V5-15-02 preregistration | blocked | Frozen-item and one-factor-arm preregistration (`preregistration_v1.json`); preregistration judge | Frozen values for CT-35 geometry, taps and analysis need measured or admitted inputs. Swept-factor baselines and arm values are unresolved. |
| V5-15-03 observables | blocked | Coverage cards (`coverage_cards_v1.json`): no observable supported | No measurement of any observable exists. A discrepancy split cannot be quantified yet. |
| V5-15-04 branches | blocked | Branch plan with calibration and held-out roles (`branch_study_plan_v1.json`) | No branch has been run. Startup/control condition and duty ladder are not admitted. |
| V5-15-05 admission and protocol | blocked | G-VTX record `not_admitted` (`gvtx_admission_v1.json`); CT-35 protocol defined, not launched (`ct35_protocol_v1.json`) | No dataset, no matching evidence, and CT-35 taps are not frozen. |

Flow archives stay disabled. The archive gate returns true only for a `Pass` admission verdict, and none exists.

## Open scientific question: PDR

The executed numerical CT-35 baseline reports PDR 7.1083 (extended outlet run, `docs/BASELINE_RUN.md`) and 4.48 (F1 initializer, `docs/results/baseline-hydraulics-v1.json`). Both are above the draft 1.4–2.5 range. The commercial 1.4–2.6 range (`docs/REFERENCES.md`, R02) is for other liners. It is diagnostic context only and cannot pass or tune a case (`judge_pdr` returns `Unresolved` for a borrowed range).

Competing explanations, to be tested one factor at a time, are listed in `preregistration_v1.json`: H1 a physical stable reverse-core vortex; H2 a wall-layer or outlet/orifice artefact; H3 an initialization-selected branch. The absence of a vortex is not assumed.

## Owned CT-35 measurement protocol (defined, not launched)

Source: `experiments/rig/protocol_v1.md`. Three replicates per head, drift gates no looser than 10 % concentration and 5 % pressure, and frozen taps. The tap positions are not frozen, so the protocol judge returns `Unresolved` until they are. The protocol may not be launched under V5-15, and no rig, pump or flowmeter run is authorized by this document.

## Limits

- No SYNTHETIC fixture in the tests is a physical result. Passing judge tests establish only that the admission rules behave as stated.
- A matched literature comparison requires admitted geometry, fluids, boundary conditions, stations, methods and uncertainty. None is admitted, so every overlay is a literature comparison.
- A version change of the evaluator or a withdrawn dataset revokes ranking authority (`judge_admission`).
- The packet's `docs/rig/PDR_STUDY.md` and `../ENGINEERING_WORKPLAN.md` do not exist in the repository. `experiments/rig/protocol_v1.md` was used in their place.
