# Validation and progress maintenance

The current PR changes planning documents, progress data and standalone dashboard tooling. It does not modify scientific source, install solver software or start a simulation.

## Checks executable for this preparation

Run from the selected checkout (on xbabe2, `/home/ubuntu/joil` after using the prepared branch):

```sh
rtk proxy python3 tools/v5_plan.py build
rtk proxy python3 tools/v5_plan.py check
rtk proxy python3 -m unittest discover -s tests -p test_v5_plan.py -v
rtk proxy python3 tools/gen_status.py --check
rtk proxy python3 tools/check_public.py
rtk git diff --check
```

Expected: 32 packet IDs, 160 unique ordered tasks, acyclic known dependencies, ten exact source identities, D-01 through D-31 mapped, every current catalogue ID mapped, valid status records and byte-current generated TODO/dashboard views. `done` needs a hashed structured receipt and dependency/order clearance. Invalid/stale state returns nonzero. The helper verifies receipt integrity/structure, not the scientific truth of its assertions.

`test_v5_plan.py` exercises planted dependency cycles, duplicate IDs, unknown/missing progress, unowned active tasks, premature completion, missing/stale receipts, path escape, missing catalogue/defect coverage, duplicate JSON keys/nonfinite values, stale generated output, safe script embedding and rollback on invalid update. These test actual plan integrity boundaries instead of duplicating engineering physics.

Verify source identities against the original `final/` and `xfinal/` directories (or their ignored private copy) independently using Python `hashlib` and the manifest; log each compared file and result. The public manifest carries names/hashes and section metadata, not original source bytes. The planning receipt records this verification and generated artifact hashes.

Serve `docs/v5-plan/` locally and open `dashboard.html`. Check desktop and mobile widths, search/task/path filters, ready-only behavior, tabs/keyboard navigation, expanded packets, local note persistence, draft export and absence of browser console errors or external requests. A representative HTTP check plus script parse is insufficient evidence that interactions work. Browser marks must not change official 0/160 progress, repository files or scientific STATUS.

## Canonical progress model

`TASKS.json` owns definitions/dependencies. Each worker owns only their packet’s `progress/V5-XX.json`. Allowed states: `todo`, `in_progress`, `blocked`, `review`, `done`. Active/review/done tasks need owner; blocked tasks need a reason. All earlier tasks in the same packet and all dependency packets must be complete before active/review/done. Integrator acceptance is still required before downstream scientific use.

```sh
rtk proxy python3 tools/v5_plan.py update V5-00-01 --status in_progress --owner NAME --note "Reviewing pinned baseline"
rtk proxy python3 tools/v5_plan.py update V5-00-01 --status blocked --owner NAME --note "Exact source differs; waiting for integrator disposition"
rtk proxy python3 tools/v5_plan.py update V5-00-01 --status done --owner NAME --receipt docs/v5-plan/receipts/UNIQUE.json
rtk proxy python3 tools/v5_plan.py build
rtk proxy python3 tools/v5_plan.py check
```

These are usage examples, not instructions to mark current tasks done. The package deliberately starts 0/160. A receipt’s required fields are: `task_ids`, full `tested_commit`, `changed_paths`, `commands` (argv, exit_code, log_sha256), `positive_checks`, `negative_checks`, `artifacts` (path/identifier and sha256), `resource_allocation`, `limitations`, `next_dependency`. Use immutable uniquely named receipts; negative tests should themselves exit successfully after verifying rejection. Record expected failed invocations separately inside negative-check descriptions.

Dashboard working notes live only in browser storage. Export creates `local_draft_notes_not_official_progress`; it is an untrusted handoff, not a completion receipt. Review it, execute checks, then update canonical progress. Generated Markdown/HTML is integrator-owned so concurrent workers do not fight over one large file. Run `check` after each rebuild and commit progress plus generated views together.

## Future engineering validation tiers

| Tier | Commands / acceptance | What failure means |
| --- | --- | --- |
| Current foundation | `rtk proxy python3 tools/check.py` uses the existing 8-suite CI matrix and aggregate foundation gate | Reproduce named failing suite; do not alter unrelated science to make a planning PR pass |
| Contract/architecture | Packet-specific Rust targets, strict schema/golden identity, generated TS, dependency-graph and unit fixtures | No consumer implementation until compatible shared contract is accepted |
| Geometry | Baseline analytic/B-rep bounds, negative corpus, volume topology, independently changed artifacts, resolved minimum features and enactment | Unsupported/invalid artifact; no score credit or assumed printability |
| Native physics | Production carrier/oil/structural adapters, independent positive and deliberately wrong-model/units/sign controls, refinement and restart | No corresponding qualified observation even if process exits zero |
| Validation | Source-accessioned matched data by observable, uncertainty and domain coverage; held-out calibration separation | Unqualified or narrowed applicability; model agreement is not physical validation |
| Runtime/recovery | Nonempty crash matrix, fencing, full disk/corrupt/missing objects, ledger/replay and restored backup | No unattended campaign release; retain actual costs and last verified checkpoint |
| Search | Known constrained fronts and fair random/conventional controls, independent seeds, random promotion/OOD/false-omission audits | Software algorithm result only until evaluator admission; negative scientific outcome is legitimate |
| Visual/publication | Wrong subject/unit/time/field, scalar-color and frame tests; 66-family recipe coverage; exact query provenance/revocation | No publication of affected result/scene; unavailable card replaces missing evidence |
| Physical | Independently built as-built subjects, calibrated synchronized blinded paired experiment and preregistered inference | No physical-candidate claim outside actually supported envelope |

Native, broad mutation, GPU and physical checks require a current explicit resource/build allocation. Source-spec benchmark arithmetic is not such an allocation. Recheck OpenFOAM/model, Blender and kernel versions at implementation; exact available builds and capabilities belong in their receipts. Do not claim to execute future packet commands during this planning task.
