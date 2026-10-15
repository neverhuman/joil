# joil

Evidence-driven evolutionary design of deoiling hydrocyclone cartridges.

Goal: more compliant treated water per liner at a fixed pressure budget and reject fraction, with performance established through numerical verification and paired physical tests. Explore unconventional internal geometry without rewarding simulation errors.

**Current status: full-liner baseline + promoted exotic 3D elite hydraulic pilots and 3-node (`xbabe1`/`xbabe2`/`xbabe3`) Rust/CUDA multi-island evolutionary screening (`119,235` 3D evaluations) executed; `F2` multi-grid/full-residence oil fitness and `F4` physical rig validation remain open.** The extended-outlet `CT-35` SST initializer reaches 4 m³/h and 1.986% reject with 29.47 kPa static pressure drop and 33.71 kJ/m³ device mechanical loss, while the promoted 3D `trifurcated-6jet-vortex-head` (`xbabe2-gen043-isl10-ind030`) OpenFOAM 14 SST pilot (`305,522` cells) reaches 4.000 m³/h and 1.971% reject at 19.53 kPa static pressure drop (`-33.74%`) and 23.12 kJ/m³ hydraulic loss (`-31.40%`). Multi-grid RSM, full-residence oil closure and independent physical validation gates remain open; these are not qualified industrial performance claims.

- [Master implementation plan](docs/MASTER_PLAN.md)
- [Physics, scoring and validation contract](docs/SCIENCE.md)
- [Representation and CAD contract](docs/GENOME.md)
- [Source review and corrections](docs/SOURCE_REVIEW.md)
- [Primary references](docs/REFERENCES.md)
- [Work packets](docs/WORK_PACKETS.json) · [Status](docs/STATUS.json)
- [Compute and installation runbook](docs/COMPUTE.md)
- [Actual baseline KPIs, failures and qualification limits](docs/BASELINE_RUN.md)
- [Reproduce the pinned pilot](docs/PILOT_RUNBOOK.md)

![CT-35 exact CAD cutaway and head fluid domain; geometry only](docs/media/overview.png)

The research baseline is CT-35: the Colman–Thew patent example scaled to D2 = 35 mm, with explicit reconstruction assumptions. It is not an exact commercial liner or a validated experimental apparatus. Geometry dimensions are in mm; solver adapters must convert to SI explicitly.

```bash
rtk cargo test --workspace --locked
rtk cargo run -p joil-cli -- baseline
rtk proxy python3 -m unittest discover -s tests -v
rtk proxy python3 tools/check_public.py
rtk proxy sh -c 'cd apps/explorer && npm ci && npm run build'
```

Optional exact CAD, inside a project environment with **Python 3.12** (on macOS, `uv venv --python 3.12 .venv` can create it):

```bash
rtk proxy python3 -m venv .venv
rtk proxy .venv/bin/pip install -r requirements-cad.txt
rtk proxy python3 tools/check.py --cad
```

CAD outputs include canonical mm STEP, mm and metre STL previews, metre glTF, terminal patch definitions and an audit report. Each check run uses a fresh output directory. Labelled conformal CFD tessellation is a later acceptance gate. See the report before using geometry downstream; scaling STEP coordinates without changing declared units is not a unit conversion.

Stack: Rust (`joil-core`, `joil-cli`) compositional 3D geometry/mesh/B-rep compiler, constrained multi-island NSGA-II + 4D MAP-Elites search engine, GP surrogate with Constrained Expected Improvement (`P07`), structural/modal/LPBF qualification (`P08`), PBM & blinded A/B/A power verifiers (`P05`/`P06`); CUDA C++20 3D volumetric SDF & Lagrangian droplet tensor evaluator (`sm_86`/`sm_89`); Python/OpenCASCADE CAD and scientific tools; pinned OpenFOAM Foundation 14 CPU/MPI solver; Vite/TypeScript/React interactive 3D hydrocyclone & droplet-vortex explorer (`apps/explorer`).

No jarambit code or shared environment is modified or required at runtime. Raw supplied material and installation details stay in ignored private storage. No project software license has been selected yet; dependency licenses remain their own.
