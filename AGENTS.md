# Joil agent contract

Read `docs/MASTER_PLAN.md`, `docs/SCIENCE.md`, `docs/WORK_PACKETS.json`, `docs/STATUS.json`, then the files owned by your packet. This repository is independent of jarambit. Never edit, build in, clean, signal, or change configuration in another project.

Read root `COORD.md`, `docs/COORDINATION.md` and any ignored `.private/coordination/` notices before editing or launching computation when another agent may be active. The user designated root `COORD.md` as the primary live coordination channel: reread it before scoped edits/jobs and append material progress frequently. A proposed allocation is not an acknowledged agreement.

- Use `rtk` for shell commands when available. Isolated Python environments and project-local build caches only. No global installs.
- `.private/`, `runs/`, `artifacts/`, customer documents, host inventories, credentials, licensed standards and raw external papers are never public Git content. Only deliberately selected, reviewed files may be staged. Run `python3 tools/check_public.py` before every push.
- Public does not mean permission to disclose customer identity or proprietary interfaces. Public research geometry is separate from private installation adapters. No contact with external parties without direct user authorization.
- Physical claims require source-bound measurements, numerical verification, independent validation and a declared applicability envelope. Unknown, absent, stale or failed evidence never becomes PASS. Synthetic examples must say synthetic. Rendering is not validation.
- Preserve exact units, concentration basis, geometry/requirements/model hashes and uncertainty components. Numerical failure is not physical infeasibility. Never relax thresholds or change baseline/duty to make a candidate win.
- No large simulation starts until numerical/physics gates and explicit CPU, RAM, disk and GPU leases exist. Shared hosts run other work. Never kill or reprioritize others' processes. Initial joil jobs: at most 4 physical cores, 12 GiB RAM, 5 GiB outputs, no GPU. Increase only after resource coordination, then record the allocation. A new user-authorized allocation supersedes these defaults.
- Claim one packet in an append-only receipt before editing. Each packet has exclusive write paths. Integrator owns root manifests, lockfiles, schemas, requirements and shared contracts. No broad concurrent writers. No subagents unless requested by the user or a packet explicitly activated for delegation.
- Every completed packet supplies commit, changed paths, exact commands/exit codes, log/artifact SHA-256, positive and negative checks, remaining limitations and next dependency. Status is derived from receipts, never self-declared as validated.
- Plans must be worker-ready: scope, ordered reads, exact paths/functions, dependencies, boundaries, small steps, numerical edge cases, commands, evidence, stop conditions and handoff. Follow `docs/MASTER_PLAN.md`; do not fork a competing master plan.

Full project implementation is planned. The initial commit implements only the explicitly listed foundation in `docs/STATUS.json`.

Language migration: read [PYTHON_EXCEPTIONS.md](PYTHON_EXCEPTIONS.md) and
[PY_TO_RUST_PLAN.md](PY_TO_RUST_PLAN.md). Local proof commands and budgets:
[docs/testing.md](docs/testing.md); boundaries: [docs/architecture.md](docs/architecture.md).
