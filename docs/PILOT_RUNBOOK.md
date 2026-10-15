# Reproduce the research pilot

Read `AGENTS.md`, `MASTER_PLAN.md`, `SCIENCE.md`, `BASELINE_RUN.md`, then live root `COORD.md`. The primary case store is `/home/ubuntu/joil/runs`; the source mirror is `/Users/bentaylor/joil`. Read the current coordination file before editing or launching anything. Keep raw runs, site data and credentials private. Every command below uses a fresh case name; never overwrite a previous run to obtain a different result.

## Environment and limits

The existing tested Docker image is recorded in `physics/openfoam/environment/lock.json`. Solver commands use its immutable local ID, read-only root filesystem, no network, an unprivileged UID, no GPU, four pinned CPUs and a 12 GiB memory limit. The runner monitors the aggregate 5 GiB `runs` + `artifacts` allowance and stops only its named container on timeout/output exhaustion. Another agent needs an acknowledged disjoint CPU lease. Do not inspect or signal unrelated processes.

On a fresh host, first inspect Docker access and resources. Build with the tested Dockerfile/key/package version, then record and qualify the resulting **new** image identity; do not silently replace `lock.json` with a mutable tag. The OpenFOAM package and Ubuntu base are pinned, but transitive apt packages are not a fully reproducible distribution snapshot. A rebuild is not asserted byte-identical to the tested image.

```bash
cd /home/ubuntu/joil
docker build -t joil-openfoam:14-20261008 physics/openfoam/environment
```

Native verification (use new case names if these already exist; drivers refuse overwrites):

```bash
.venv/bin/python physics/openfoam/verification/poiseuille.py --cells 32 --iterations 2000
.venv/bin/python physics/openfoam/verification/poiseuille.py --cells 64 --iterations 8000
.venv/bin/python physics/openfoam/verification/droplet.py --dt 1e-7
.venv/bin/python physics/openfoam/verification/droplet.py --dt 5e-8
```

Use the exact tested arguments and command receipts in `docs/receipts/native-elementary-20261015.json`, together with the driver CLI. Two resolution/timestep values matter; a single passing result is not a convergence study.

## Full-liner mesh and initializer

Generate baseline CAD with the pinned project environment:

```bash
.venv/bin/python cad/ct35/build.py --out artifacts/new-baseline/ct35
.venv/bin/python physics/openfoam/mesh/prepare.py \
  --case runs/new-mesh --surface artifacts/new-baseline/ct35/fluid_preview_m.stl
python3 tools/bench_cfd.py mesh --case runs/new-mesh
.venv/bin/python physics/openfoam/mesh/audit.py --case runs/new-mesh
```

The current seed is rejected by `checkMesh -allTopology -allGeometry`. The independent auditor returns exit 1 when any gate fails, even though native `checkMesh` may return exit 0 with a `Failed ... checks` log. Preserve that failure; it cannot seed a qualified evaluator.

A small failed-mesh diagnostic requires explicit `--diagnostic`. The point factory otherwise refuses it. Append a separate numerical outlet extension only with a declared sensitivity-study identity:

```bash
.venv/bin/python physics/openfoam/mesh/extend_outlet.py \
  --source runs/new-mesh --case runs/new-extension --length .12 --layers 60
.venv/bin/python physics/openfoam/mesh/audit.py --case runs/new-extension
python3 physics/openfoam/carrier/point.py \
  --mesh-case runs/new-extension --case runs/new-carrier \
  --model kOmegaSST --flow 4 --reject-pressure-pa -180000 --iterations 600 --diagnostic
python3 tools/bench_cfd.py water --case runs/new-carrier
```

The auditor’s expected failure must be examined before deliberately continuing to the diagnostic command. No shell pipeline should mask it. The case factory binds mesh, fields, requirements/genome and model settings by exact bytes. A warm start is allowed only from a successful saved state with identical mesh and turbulence model. `SSG` and `LRR` are supported configuration choices but are not qualified; the first SSG attempt diverged. A pressure setting is a measured operating point, not automatic split control or a validated pressure curve.

## Trusted extraction

After the solver **finishes successfully**, recover the original device plane, reconstruct the explicit saved time, compile a fresh native metrics binary, and extract:

```bash
.venv/bin/python physics/openfoam/mesh/reference_plane.py \
  --case runs/new-carrier --source runs/new-mesh
python3 tools/bench_cfd.py reconstruct --case runs/new-carrier --time 600
mkdir runs/new-metrics-build
python3 tools/bench_cfd.py build-metrics --case runs/new-metrics-build
python3 tools/bench_cfd.py metrics --case runs/new-carrier --time 600 \
  --metrics-bin runs/new-metrics-build/bin/joilHydraulicMetrics
python3 analysis/carrier_snapshot.py \
  --case runs/new-carrier --out runs/new-carrier/snapshot-final
python3 analysis/baseline_report.py --point runs/new-carrier snapshot-final \
  --out runs/new-carrier/public-safe-report.json
```

The report rejects changed case inputs, changed raw snapshots, failed native receipts, wrong saved times and inconsistent native fluxes. It reports static taps separately from pressure-plus-kinetic mechanical flux. Original-plane energy uses native conservative `phi` and linearly interpolated internal-face `p,U`; an independent energy verification and discretization-error estimate remain required. Oil results and qualified fitness remain absent until actual reviewed transport evidence exists.

Render only source-bound saved fields:

```bash
OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 taskset -c 0-3 .venv/bin/python \
  visualization/render_carrier.py --case runs/new-carrier --time 600 \
  --out artifacts/new-carrier-view
```

Scene manifests bind the exact p/U/mesh and extractor. VTK interpolation is for display; image samples do not replace native conservative KPI integrals. Geometry-only proposal builds and renders use `cad/program/{propose,build_proposals}.py` and `visualization/render_proposals.py`; no oil score is inferred from shape or appearance.

## Handoff and stop conditions

Append exact paths, hashes, commands/exit codes, observed timings and remaining gates to `COORD.md`; never overwrite another agent’s entry. Preserve failed attempts. Integrate only reviewed, explicitly staged source/summary files, run `python3 tools/check.py` and `python3 tools/check_public.py`, then publish. Raw fields and the live coordination file are ignored.

Stop the affected lane for input/hash mismatch, nonfinite fields, failed inventory, external backflow without a qualified interpretation, mesh/force qualification failure, changed interfaces or resource pressure. Missing measured fluid/absolute-pressure/duty/rig data restricts the claim; it does not authorize inventing performance values. No pressure/energy/duty limits, rig trials, FEA allowables or material certification have been supplied by these runs.
