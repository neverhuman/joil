# Compute and setup runbook

Primary checkout: xbabe2 `/home/ubuntu/joil`; local mirror `/Users/bentaylor/joil`. Separate Git history/environments from jarambit. Detailed host/user/process inventory stays private. Hardware was inspected during this source study; availability changes.

| Host | Physical cores / threads | RAM | GPU | Initial role |
|---|---:|---:|---|---|
| xbabe1 | 64 / 128 | ~64 GiB | RTX 4090, 24 GiB | optional surrogate/rendering, smaller CPU cases after lease |
| xbabe2 | 64 / 128 | ~128 GiB | RTX 3090, 24 GiB | development, CAD, CPU reference pilot after lease |
| xbabe3 | 64 / 128 | ~128 GiB | RTX 3090, 24 GiB | packed CPU cases only after disk/resource lease |

Existing jobs remain untouched. No host is reserved by this document. Bootstrap cap: 4 physical cores, 12 GiB, 5 GiB disk, no GPU. Check actual physical core IDs using `lscpu -e=CPU,CORE,SOCKET,ONLINE`; do not assume 0–63 excludes SMT siblings. Set unrelated math-library threads to one. Reserve 30% memory headroom using measured RSS, not guessed per-cell values. Current nonzero swap use is a reason to measure pressure, not to clear caches/swap or kill work.

## Executable foundation

```bash
cd /home/ubuntu/joil
rtk cargo test --workspace --locked
rtk cargo run -p joil-cli -- baseline
rtk proxy python3 -m unittest discover -s tests -v
rtk proxy python3 tools/check.py --cad
rtk proxy sh -c 'cd apps/explorer && npm ci && npm run build'
```

When remote `rtk` is unavailable, run through local `rtk proxy ssh ...`, or use the underlying command on that host. Rust is available under `~/.cargo/bin`; explicitly add that directory to PATH for noninteractive SSH. The initial CAD environment is project-local Python 3.12 with CadQuery 2.7.0. `requirements-cad-lock.txt` records the installed dependency resolution for the tested Linux environment; exact platform compatibility must be checked before reuse. UI uses Node 22+ via project-local runtime on xbabe2; the host's Node18 is not silently upgraded.

## Reference solver environment

A pinned Foundation14 CPU container is now installed and tested on xbabe2, without sudo or global MPI/driver changes. Exact image/package/build/compiler/MPI identities and the installed package manifest are in `physics/openfoam/environment/` and `docs/receipts/environment-20261015.json`. Use `tools/verify_environment.py --case runs/new-environment-check` for an immutable startup/version receipt. Rebuilding the Dockerfile requires a new checked image identity because transitive apt packages are not a distribution snapshot. See `PILOT_RUNBOOK.md` for executed setup and run commands. The conditional system-install instructions below remain fallback guidance.

P03 first inspects shell environments, existing images and `docker info`; tools absent from PATH are not proof that nothing is installed. Prefer a pinned isolated container, recording image digest, source commit, compiler and MPI ABI. Test MPI startup, rank communication, restart and failure before CFD. Docker permissions must be established; do not add users to groups or change the daemon automatically.

If a system install is necessary, the administrator can use these **conditional** packages on Ubuntu 24.04, after confirming repository/version compatibility:

```bash
sudo apt-get update
sudo apt-get install --no-install-recommends build-essential cmake ninja-build \
  git ca-certificates curl gnupg python3-venv python3-dev \
  openmpi-bin libopenmpi-dev libscotch-dev libptscotch-dev \
  libreadline-dev libncurses-dev libxt-dev zlib1g-dev calculix-ccx gmsh
```

This is a prospective dependency list, not an executed request or a guaranteed complete Foundation build recipe. Use the [official Foundation-14 Ubuntu instructions](https://openfoam.org/download/14-ubuntu/) to add its **signed** repository and select `openfoam14`; do not substitute Ubuntu's differently packaged `openfoam`. Never pipe a network script directly into sudo. Check actual package candidates, pin versions, then smoke-test P03. Apptainer/Slurm are optional later infrastructure, not prerequisites for the first benchmark. Do not install a new scheduler on shared hosts for this project.

## Performance experiment and ETAs

Measured on 15 October 2026: the full-liner seed contains 302,460 cells; the extended mesh contains 313,260 cells. A 600-iteration steady SST run took 621.45 s on four cores. A warm-start 300-iteration point took 163.02 s; an independent 500-iteration startup took 395.55 s. These are single diagnostic timings under shared-host load, not p95 estimates, physical-time CFD rates or oil/LES ETAs. No GPUs were allocated. A mutated CAD proposal took 108–242 s, with exact-baseline export taking 3.9 s. Retained runs/artifacts were about 2.6 GiB at the environment check. See `BASELINE_RUN.md` and receipts for rejected meshes, SSG divergence, actual KPIs and scope.

Actual minimum bore is 1.4 mm, much smaller than earlier HTML's bore. Resolve geometry/wall treatment and conduct convergence; 1–4 M F2 and 8–30 M F3 cells are initial planning ranges only. They may be inadequate. Measure physical seconds per wall hour, steps/s, pressure solve fraction, parcel updates, RSS, disk and retries at fixed numerical acceptance.

Start low-core isolated pilot; then test 8/16/32/64 ranks where allocations permit. For a reserved 64-physical-core host compare 4×16, 2×32, 1×64 packing. Choose qualified results/core-hour within latency, not minimum isolated wall time. Three repeats and realistic I/O/packing expose variability. Cross-host independent jobs precede multi-node MPI.

Before measurements only: F2 100–10,000+ core-hours/point, F3 1,000–100,000+; broad uncertainty includes unknown stable timestep and statistical duration. Two F2 evaluations/design at 32 allocated cores imply 6.25–625+ h/design, before mesh/control/queue/robustness. Divide aggregate core-hours by **allocated available** physical cores, not fleet advertised threads; include efficiency and retries. The initial kernel's arithmetic/CAD timings are not CFD timings.

Foundation CPU reference first. Benchmark v2606 offloading as a separate release-family lane because the current official announcement makes it relevant; porting/model support and consumer-GPU FP64/memory limitations need actual measurement. PETSc/Ginkgo algebra is also a separate adapter experiment. Require equivalent observables/uncertainty and ≥1.3× full-job speed/cost benefit. Otherwise use GPUs for training and Blender. No driver upgrade or speculative hardware purchase.

Durability: independent attempt IDs, immutable manifests, atomic/fsynced receipt/checkpoint writes, bounded retries and scratch budgets. Restart particle/controller/statistical state together. Stop owned jobs on allocated disk/RAM pressure, preserve minimal logs and quarantine incomplete output. No hidden lower-fidelity retry to obtain PASS.

Continuity stress test: at 4 m³/h and 2% reject, a 1.4 mm bore carries ~14.4 m/s. At 32 cells/bore and CFL .5, a flow-aligned-cell estimate is ~1.5 µs; ten nominal turnovers would then require ~6.6 million steps. This is cost arithmetic, not an observed timestep, a required duration or an ETA. Profile before allocating a campaign.
