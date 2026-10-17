# Single-PR preparation and execution handoff

Base: public `main@cc86edc90d06f0eac69022f84354abc94bb453c4`. Branch: `codex/v5-engineering-plan`. This request prepares the complete engineering plan and tracker; it does not execute 160 future tasks. Keep one draft integration PR. The owner may continue implementation on that PR or direct a later branch strategy; do not open separate lane PRs automatically.

## Prepared content

- Master-plan addendum and work-packet pointer preserve P00–P10 and the original master snapshot.
- Ten-source manifest, section coverage, substantive reconciliation, code-anchor audit and all-31-defect traceability.
- Detailed engineering workplan, inputs/risks, validation/receipt protocol and 32 individual TODO files with 160 ordered tasks.
- Machine-readable task/progress/catalogue data and deterministic standard-library build/check/update tool.
- Standalone searchable offline HTML dashboard with canonical progress, dependencies, release boundaries and browser-local draft notes.
- Scope claim and immutable preparation/handoff receipt(s), with checks and SHA-256 records.

## Existing work to preserve

The xbabe2 primary checkout started on `feat/joil-v4-truth-evidence@fdff5bad9475277a9bed5e3a4612fd5e990580dc` with an unrelated modified `COMMS.md`. The Mac `~/joil` mirror started on older `codex/baseline-physics` with untracked prototype/review files. No reset, clean, stash, broad stage, forced pull, automatic branch deletion or modification of those prototypes is permitted. Use a separate branch/checkout and additive mirrors; preserve the pre-existing journal bytes. Root COORD remains primary.

If primary `~/joil` cannot safely switch because another owner is active, leave its branch unchanged and save the package additively there; use the isolated prepared checkout for Git operations. Record exact path/branch in the delivery receipt. Do not claim the primary tree is clean when its pre-existing journal remains dirty.

## Exact review and push procedure

```sh
rtk git status --short --branch
rtk proxy python3 tools/v5_plan.py check
rtk proxy python3 -m unittest discover -s tests -p test_v5_plan.py -v
rtk proxy python3 tools/gen_status.py --check
rtk proxy python3 tools/check_public.py
rtk git diff --check
rtk git diff --stat
```

Stage only the reviewed plan paths: `docs/v5-plan/`, `docs/WORKPLAN-2026-10-17.md`, master/work-packet pointers, `tools/v5_plan.py`, `tools/v5_dashboard.html`, `tests/test_v5_plan.py` and named `docs/receipts/P00-v5-plan-*`. Inspect the cached diff before committing. Source input copies stay under `.private/v5-plan-inputs/`; never stage them. Run the public scan before every push. Push this one branch without force, create one draft PR against main with `docs/v5-plan/PR_DESCRIPTION.md`, and attach its returned URL to the chat.

After push, inspect checks for the exact pushed SHA. The existing foundation workflow exercises Rust, Python, public checks, generated scientific status and explorer build. This planning PR does not change the workflow. If an unrelated baseline issue appears, document it rather than silently claiming all checks passed. A green software workflow does not establish scientific qualification. Leave merge to the owner.

## Worker continuation

Start V5-00-01 after reading root rules/coordination and claiming scope. Complete tasks in order; dependencies require accepted packet receipts. New workspace packages and shared types land through integrator commits before workers execute the planned commands. Keep packet progress files disjoint; integrator rebuilds shared views. Avoid a “complete all milestones” autonomous run before requirements, data and leases have been admitted.

Every handoff names tested commit, changed files, exact commands/exit codes, log/artifact hashes, positive/negative checks, actual resource allocation, evidence eligibility, limitations and next dependency. Preserve unavailable/stale evidence and negative results. Do not create a self-referential receipt hash: a receipt can refer to the preceding tested source commit; its own final inclusion is verified by the later delivery/CI head and Git history.
