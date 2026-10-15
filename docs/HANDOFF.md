# Research pilot handoff

The public research repository is `neverhuman/joil`. Primary working copy is xbabe2 `/home/ubuntu/joil`; local mirror is `/Users/bentaylor/joil`. Jarambit was read only. All 17 supplied files and archive contents were studied; raw source, original meshes and installation/customer context remain ignored and private.

Read `AGENTS.md`, live `COORD.md`, `MASTER_PLAN.md`, `SCIENCE.md`, `STATUS.json`, `BASELINE_RUN.md` and `PILOT_RUNBOOK.md`. P01–P05/P09 have partial work; none of those science packets is complete. Resolve mesh/carrier/oil qualification before performance-selected evolution.

Implemented: baseline genome/analytic contract; 24 Rust arithmetic regressions; 19 Python checks; exact baseline and nine proposal CAD builds; immutable evidence/inventory/pressure helpers; pinned Foundation14 container; native channel and light-drop-rise checks; full-liner mesh/carrier factories; three completed SST diagnostic points; exact mechanical-flux extraction; actual CAD/field views; publication guard and React evidence explorer. See receipts for executed commands/hashes and scope. The 66 catalogued challenge families are planned, not passed-test counts. Gemini's separate oil/rotation, exotic geometry and search files require scoped review and handoff before integration.

```bash
cd /home/ubuntu/joil
python3 tools/check.py --cad
```

The check runner selects the project-local Node 22 runtime, available Rust toolchain and unique output paths. It archives logs/receipts under `.private/checks/<run-id>/`; CAD goes under `artifacts/<run-id>/ct35/`. Raw generated artifacts stay out of Git. Canonical STEP declares mm; metre STL is unitless with explicit manifest, glTF uses metres. Do not use superseded coordinate-scaled STEP experiments in older ignored artifact directories.

To regenerate the geometry plate with the isolated CAD environment:

```bash
.venv/bin/python visualization/render_baseline.py --out artifacts/ct35/overview.png
```

Independent source audit deliberately returns exit 1 for the **supplied** inward-wound latest head STL. The source is unchanged; new B-rep previews are positive-volume/outward. Preserve this negative finding. Exact B-rep sharp-corner head mass is ~570.74 g at assumed density 4.43 g/cm³; SDF preview describes a slightly different offset solid. Neither is a manufactured measurement.

The current useful saved state is `runs/ct35-sst-extended-000/600`: feed4m³/h, reject1.98596%, dp29.468kPa, device energy33.710kJ/m³, external outlet backflow0, PDR7.108. `docs/results/baseline-hydraulics-v1.json` binds the observations and native integrals; oil/fitness remain null. Raw fields stay on xbabe2. `runs/ct35-hydraulic-diagnostic-001` diverged and must never seed oil transport. Both unextended SST points had external reject backflow and are also excluded from naive capture interpretation.

Remaining science: acceptable mesh and boundary layers, matched outlet extension/grid/time/model qualification, stabilized transient Reynolds-stress flow, full-liner tracer and force/dispersion/breakup/parcel tests, independent rig/holdout data, full graph/implicit CAD, calibrated performance-selected search, solid FEA and manufacturing/service eligibility. No separation gain, field suitability or pressure rating is claimed.

Blocking campaign inputs: actual vessel/port/seal drawing; pressure/flow/absolute-pressure duty and required band; measured feed properties/DSD/chemistry/gas/solids; AM process/material state; comparator and rig/metrology access. The assumed radial envelope conflicts with baseline ports. Do not change it quietly or tune duty limits to make a baseline pass.

Compute remains a small CPU-only pilot under recorded leases. A completed600-iteration SST run took621.45s on four cores; this is not a physical-time F2/LES ETA. Read live `COORD.md` and the expiring private lease for current allocations. No GPU or fleet reservation is granted. No sudo installation was needed: Docker is isolated; Gmsh's missing GLU dependency was unpacked under `.private/gmsh-libs/root`, without a global install. Use the recorded `LD_LIBRARY_PATH` if running the optional Gmsh challenge. Conditional system packages and GPU parity experiments are in `COMPUTE.md`.

Prior bootstrap iterations caught and corrected an extra plenum overlap, missing private publication rules on the remote copy, an inappropriate STEP scaling check and plotting API incompatibilities. They did not change scientific thresholds; final accepted source/artifact receipts identify the corrected implementation. Detailed unsuccessful attempt logs/receipts remain private where retained.

Rollback: this repository is independent; stop only owned joil processes and check out a prior joil commit. Preserve immutable source and result identities. Do not reset, clean or signal another project. Future workers return tested scoped commits and receipts; the integrator serializes shared contracts and publication.
