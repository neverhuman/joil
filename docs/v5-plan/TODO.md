# Joil v5 implementation ToDos

Generated from `TASKS.json` and packet-owned `progress/*.json`. Run `rtk proxy python3 tools/v5_plan.py build` after a progress commit.

**Planning is prepared; engineering implementation progress is 1/160. Scientific qualification remains governed by `docs/STATUS.json`.**

| Packet | Milestone | Owner role | Complete | Dependencies |
| --- | --- | --- | ---: | --- |
| [V5-00 — Baseline preservation and reproducible defect inventory](todos/V5-00.md) | M0 | integrator | 1/5 | none |
| [V5-01 — Repair current misleading observations and presentation](todos/V5-01.md) | M0 | truth integrator | 0/5 | V5-00 |
| [V5-02 — Requirements authority and decision profiles](todos/V5-02.md) | M0 | requirements integrator | 0/5 | V5-00 |
| [V5-03 — Typed contracts and enforced blind evaluator boundary](todos/V5-03.md) | M0 | contract integrator | 0/5 | V5-02 |
| [V5-04 — Independent statistics and qualification inference](todos/V5-04.md) | M1 | statistics owner | 0/5 | V5-02, V5-03 |
| [V5-05 — Private content store, exact caches and revocation](todos/V5-05.md) | M1 | store owner | 0/5 | V5-03 |
| [V5-06 — Durable task execution, leases and complete accounting](todos/V5-06.md) | M1 | runtime owner | 0/5 | V5-03, V5-05 |
| [V5-07 — Checkpoint, resume, fork and one-command fixture run](todos/V5-07.md) | M1 | runner owner | 0/5 | V5-06 |
| [V5-08 — One material partition and bounded geometry kernel](todos/V5-08.md) | M2 | geometry kernel owner | 0/5 | V5-03, V5-05 |
| [V5-09 — Independent artifact-only geometry and topology observation](todos/V5-09.md) | M2 | geometry observation owner | 0/5 | V5-08 |
| [V5-10 — Derived surfaces, volume meshes and honest exports](todos/V5-10.md) | M2 | mesh/export owner | 0/5 | V5-08 |
| [V5-11 — MPG schema, bounded development and legacy lowering](todos/V5-11.md) | M2 | genome owner | 0/5 | V5-03, V5-08 |
| [V5-12 — Variation, expressiveness and operator-yield audit](todos/V5-12.md) | M2 | operators owner | 0/5 | V5-09, V5-10, V5-11 |
| [V5-13 — Material/process passports and actual mass](todos/V5-13.md) | M2 | manufacturing owner | 0/5 | V5-02, V5-09, V5-10 |
| [V5-14 — Corrected native carrier evaluator and control](todos/V5-14.md) | M3 | reference CFD owner | 0/5 | V5-02, V5-03, V5-10 |
| [V5-15 — Validation accession and controlled vortex/PDR study](todos/V5-15.md) | M3 | validation owner | 0/5 | V5-14 |
| [V5-16 — Native light-drop forces, face injection and inventory](todos/V5-16.md) | M3 | oil transport owner | 0/5 | V5-04, V5-14 |
| [V5-17 — Dispersion, breakup and transport uncertainty](todos/V5-17.md) | M3 | multiphase model owner | 0/5 | V5-16 |
| [V5-18 — Common-flow capacity, energy, mass-reserve decisions](todos/V5-18.md) | M4 | metrics integrator | 0/5 | V5-04, V5-13, V5-15, V5-17, V5-20 |
| [V5-19 — Candidate-specific structural loads, static FEA and buckling](todos/V5-19.md) | M3 | structures owner | 0/5 | V5-02, V5-10, V5-13, V5-14 |
| [V5-20 — Wet dynamics, finite-life fatigue and as-built uncertainty](todos/V5-20.md) | M3 | dynamics/material owner | 0/5 | V5-04, V5-19 |
| [V5-21 — Deterministic reference-search policy and fair controls](todos/V5-21.md) | M4 | search owner | 0/5 | V5-07, V5-12, V5-18 |
| [V5-22 — Multi-fidelity acquisition, surrogate calibration and audits](todos/V5-22.md) | M4 | acquisition owner | 0/5 | V5-04, V5-15, V5-17, V5-21 |
| [V5-23 — Leased workers, measured benchmarks and scaling](todos/V5-23.md) | M4 | HPC owner | 0/5 | V5-06, V5-07, V5-14 |
| [V5-24 — Optional GPU flow and voxel-FEA admission track](todos/V5-24.md) | M5 | accelerator research owner | 0/5 | V5-15, V5-17, V5-19, V5-23 |
| [V5-25 — Optional frozen-liner response operator](todos/V5-25.md) | M5 | reduced-model owner | 0/5 | V5-15, V5-17, V5-22 |
| [V5-26 — Evidence-bound scenes and complete Blender recipe registry](todos/V5-26.md) | M5 | render owner | 0/5 | V5-03, V5-05, V5-10 |
| [V5-27 — Read-only evidence explorer and live progress](todos/V5-27.md) | M5 | explorer owner | 0/5 | V5-07, V5-26 |
| [V5-28 — Preregistered research, mechanism study and query-built paper](todos/V5-28.md) | M6 | research owner | 0/5 | V5-04, V5-18, V5-21, V5-22, V5-26 |
| [V5-29 — Structural bracket reuse proof](todos/V5-29.md) | M6 | second-pack owner | 0/5 | V5-07, V5-12, V5-19, V5-21, V5-26 |
| [V5-30 — Physical protocol, as-built confirmation and private dossiers](todos/V5-30.md) | M7 | experimental lead | 0/5 | V5-15, V5-17, V5-20, V5-28 |
| [V5-31 — Single-PR integration, migration and final release evidence](todos/V5-31.md) | M7 | integrator | 0/5 | V5-01, V5-02, V5-03, V5-04, V5-05, V5-06, V5-07, V5-08, V5-09, V5-10, V5-11, V5-12, V5-13, V5-14, V5-15, V5-16, V5-17, V5-18, V5-19, V5-20, V5-21, V5-22, V5-23, V5-26, V5-27, V5-28, V5-29, V5-30 |

## Progress commands

```sh
rtk proxy python3 tools/v5_plan.py update V5-00-01 --status in_progress --owner NAME --note "Baseline review started"
rtk proxy python3 tools/v5_plan.py update V5-00-01 --status done --owner NAME --receipt docs/v5-plan/receipts/UNIQUE.json
rtk proxy python3 tools/v5_plan.py build
rtk proxy python3 tools/v5_plan.py check
```

Only use `done` after the receipt is real and reviewed. Browser draft marks and notes stay local and never update this progress source.
