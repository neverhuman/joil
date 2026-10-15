# Foundation handoff

The public research repository is `neverhuman/joil`. Primary working copy is xbabe2 `/home/ubuntu/joil`; local mirror is `/Users/bentaylor/joil`. Jarambit was read only. All 17 supplied files and archive contents were studied; raw source, original meshes and installation/customer context remain ignored and private.

Read `AGENTS.md`, `MASTER_PLAN.md`, `SCIENCE.md`, `SOURCE_REVIEW.md`, `GENOME.md`, `WORK_PACKETS.json` and `STATUS.json`. Next packet is **P01**, not an evolutionary CFD campaign.

Implemented: named baseline genome and independent analytic dimensions; Rust concentration, inventory and common-tested-point arithmetic; 24 Rust regression checks; six Python foundation checks; exact CAD generation/audit; SI surface/glTF exports; publication guard; React/Vite/TypeScript research explorer; pinned package resolutions and CI foundation workflow. See receipts for actually executed commands, hashes and scope. The 66 catalogued scientific challenge families are planned, not executed physics tests.

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

Remaining science: pinned OpenFOAM capability build, conformal labelled CFD surface, mesh/time/parcel/model qualification, correct light-drop dispersion/breakup, physical rig and holdout data, flexible graph/implicit CAD, search algorithms, structural/manufacturing/service eligibility and evidence explorer fields. No simulation, performance gain, field suitability or pressure rating is claimed.

Blocking campaign inputs: actual vessel/port/seal drawing; pressure/flow/absolute-pressure duty and required band; measured feed properties/DSD/chemistry/gas/solids; AM process/material state; comparator and rig/metrology access. The assumed radial envelope conflicts with baseline ports. Do not change it quietly or tune duty limits to make a baseline pass.

Compute starts at small CPU-only pilot under a resource lease. All hosts have 64 physical cores, existing work and finite disk; no fleet allocation is granted. No sudo installation was needed for the foundation. Conditional solver packages and GPU parity experiments are in `COMPUTE.md`.

Prior bootstrap iterations caught and corrected an extra plenum overlap, missing private publication rules on the remote copy, an inappropriate STEP scaling check and plotting API incompatibilities. They did not change scientific thresholds; final accepted source/artifact receipts identify the corrected implementation. Detailed unsuccessful attempt logs/receipts remain private where retained.

Rollback: this repository is independent; stop only owned joil processes and check out a prior joil commit. Preserve immutable source and result identities. Do not reset, clean or signal another project. Future workers return tested scoped commits and receipts; the integrator serializes shared contracts and publication.
