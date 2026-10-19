# Python to Rust execution plan (`docs/` index)

The canonical root plan is [`/PY_TO_RUST_PLAN.md`](../PY_TO_RUST_PLAN.md), and the
per-file exception inventory is [`/PYTHON_EXCEPTIONS.md`](../PYTHON_EXCEPTIONS.md).
Receipt-linked wave handoffs are also indexed in
[`docs/v5-plan/PYTHON_TO_RUST_MIGRATION.md`](v5-plan/PYTHON_TO_RUST_MIGRATION.md).

## Current additive Rust CLI port status (`crates/crucible-cli/src/bin/`)

Through Wave I021 (`origin/main`), **24** Python CLI/control tools have parity-verified
native Rust binaries in `crates/crucible-cli/src/bin/` with integration parity suites
in `crates/crucible-cli/tests/`:

1. `check-public` (`tools/check_public.py`)
2. `check-portability` (`schemas/v5/check_portability.py`)
3. `check-foundation` (`tools/check.py`)
4. `v5-private-dossier` (`tools/v5_private_dossier.py`)
5. `v5-d28-ownership-control` (`tools/v5_d28_ownership_control.py`)
6. `v5-explorer-manifest` (`tools/v5_explorer_manifest.py`)
7. `v5-plan` (`tools/v5_plan.py`)
8. `v5-d29-blind-judge-control` (`tools/v5_d29_blind_judge_control.py`)
9. `v5-baseline-audit` (`tools/v5_baseline_audit.py`)
10. `gen-status` (`tools/gen_status.py`)
11. `v5-d21-trajectory-control` (`tools/v5_d21_trajectory_control.py`)
12. `v5-d18-scheduler-control` (`tools/v5_d18_scheduler_control.py`)
13. `verify-environment` (`tools/verify_environment.py`)
14. `v5-fixture-smoke` (`tools/v5_fixture_smoke.py`)
15. `bench-cfd` (`tools/bench_cfd.py`)
16. `baseline-report` (`analysis/baseline_report.py`)
17. `mesh-reference-plane` (`physics/openfoam/mesh/reference_plane.py`)
18. `mesh-extend-outlet` (`physics/openfoam/mesh/extend_outlet.py`)
19. `mesh-audit` (`physics/openfoam/mesh/audit.py`)
20. `carrier-configure` (`physics/openfoam/carrier/configure.py`)
21. `carrier-point` (`physics/openfoam/carrier/point.py`)
22. `mesh-prepare` (`physics/openfoam/mesh/prepare.py`)
23. `carrier-snapshot` (`analysis/carrier_snapshot.py`)
24. `propose-wall-field` (`cad/program/propose.py`)

See [`../PY_TO_RUST_PLAN.md`](../PY_TO_RUST_PLAN.md) for the full 113-file (`26,275` baseline LOC)
inventory and Wave P1–P5 schedule.
