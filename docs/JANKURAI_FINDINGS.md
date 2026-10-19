# Jankurai remediation register

This register complements the language migration annex to [MASTER_PLAN.md](MASTER_PLAN.md).
It does not grant audit conformance or scientific qualification. An advisory CI
success does not close a standard-mode blocker. Exact findings, fingerprints,
source hashes and command results belong to the additive
[I023 receipt](receipts/i023-rust-ci-20261019.json).

## I023 changes

`carrier-snapshot` now supplies a fully initialized `libc::tm` to `gmtime_r`.
UTC golden cases cover epoch, leap day, the non-leap century boundary and
milliseconds. Actual `proptest` cases check timestamp ordering and fractional
seconds over the supported four-digit UTC year range. Existing positive and
negative snapshot parity tests continue to run against the Python oracle.

[The scan policy](../agent/audit-policy.toml) excludes only ignored private,
run, build and nested worktree output roots. No tracked file is excluded; all
Python migration sources, tests, workflows and historical receipts remain in
scope. Retained synthetic fixtures are not duplicated product implementations.
The policy changes no score floor, severity or cap. Adding real property tests
closes the missing-test finding that scratch output previously masked.

## Open source findings and review dispositions

These entries remain open in the scanner. Review dispositions preserve the
evidence and explain the next action; they do not suppress findings or promote
the repository to conformance. Targets are review deadlines, not completion
claims. The captain owns integration and review of each handoff.

| ID & Work Slice | Rule, path and fingerprint | Owner | Evidence, action and review target |
| --- | --- | --- | --- |
| `F-01` (`I024-S2..S6` / `I025`) | `HLT-001-DEAD-MARKER:shape`, repository (`sha256:650992209a6a9fc1ce75b3b767d951b003a6e074e4148caf363e2504a1bfc3bf`) | rust-integrator | Slices `I024-S2..S6` retired all 16 Rust `.rs` files `> 1,000` LOC across both `src/` and `tests/` (`11` authored `src/*.rs` files and `5` integration `tests/*.rs` files `pack-hydrocyclone-judges/tests/{liner_operator,manufacturing,structure}.rs`, `joil-evidence/tests/decision.rs`, `joil-core/tests/regression.rs`; **0** tracked `.rs` files `> 1,000` LOC remain in the repository, max `973` LOC). Largest remaining authored code file is `physics/openfoam/oil/oil_grade.py` (`1,201` LOC), scheduled for Wave P2 retirement; remaining `> 500` LOC Rust modules follow I025. Review 2026-10-22; Python replacements follow P2–P4. |
| `F-02` (`I025-CI`, deduplicated with `F-03..F-04`) | `HLT-034-CI-BAD-BEHAVIOR`, `.github/workflows/foundation.yml` (`sha256:289aadc737421172264e369bfacbb491d0f578b1b1ebad8b5c260a9060eb9da4`) | captain / integrator | The workflow denies fork PR jobs and uses the disposable VM pool described in [CI.md](CI.md). The detector treats the mere presence of PR events and self-hosted labels as privileged access. Retain guard and pool contract; captain must verify runner isolation and dispatch policy before considering a scoped exception. Review 2026-10-20. |
| `F-03` (`I025-CI`, deduplicated with `F-02, F-04`) | `HLT-034-CI-BAD-BEHAVIOR`, `.github/workflows/jankurai.yml` (`sha256:4498ca8caf0370654bf5852f8311b08875398c37342f5823d20996c29d0a0c4a`) | captain / integrator | Same same-repository guard and disposable runner contract. Do not remove PR auditing or move to an unauthorized runner to clear the detector. Isolation/dispatch review 2026-10-20. |
| `F-04` (`I025-CI`, deduplicated with `F-02..F-03`) | `HLT-034-CI-BAD-BEHAVIOR`, `.github/workflows/security.yml` (`sha256:bb0285ec8321b389835b36888940ec7c9e95a3c42a3cc29d2cb0a25e4a9d76d0`) | captain / integrator | Same same-repository guard and disposable runner contract; read-only token and disabled checkout credential persistence remain enforced. Isolation/dispatch review 2026-10-20. |
| `F-05` (`Wave P1b..P5`, deduplicated with `F-09..F-10`) | `HLT-000-SCORE-DIMENSION:stack`, `analysis/__init__.py` (`sha256:e3de85f3008b1365bb07a60e5128a1d76810d431e0db1b8c60475c828edcb1ae`) | migration-integrator | Real legacy Python surface. Follow [PY_TO_RUST_PLAN.md](../PY_TO_RUST_PLAN.md): enumerate callers, prove Rust parity, cut over consumers and only then retire sources. P1b review 2026-10-22; subsequent waves retain their published dates. |
| `F-06` (`I025-EV12` / `Wave P2`) | `HLT-008-FALSE-GREEN-RISK`, `crates/joil-core/tests/truth_ev12_capture.py` (`sha256:4222c3c09585852c3f62c1e6cfcb4513c8eb98e2524c2834b8df89f5c016c55b`) | rust-integrator / integrator | This is an input-retention CLI, not a skipped test. `SystemExit(main())` propagates its failure status. Its separate `truth_ev12_capture_test.py` checks missing identity, tampered bytes, symlinks, nested/existing destinations and CLI failure. Historical handoff pins the helper's bytes; preserve them. Review the replay evidence by 2026-10-20, with retirement subject to migration parity. |
| `F-07` (`I025-EVIDENCE`, deduplicated with `F-08`) | `HLT-027-HUMAN-REVIEW-EVIDENCE-GAP`, `crates/joil-core/tests/truth_numtrace_audit.json` (`sha256:0e1590eaf0329df73c02b3dd314b2f0f5ef45a65c1b8a5b3ebbb55359500a5fc`) | integrator | Frozen audit distinguishes authored declarations from measured facts, lists source hashes and a pinned integration commit, and embeds `recheck_python` / `recheck_command`. Retain its negative findings. Recheck source hashes, arithmetic and planted negative controls; never rewrite historical claims or hashes to clear a detector. Evidence review 2026-10-20. |
| `F-08` (`I025-EVIDENCE`, deduplicated with `F-07`) | `HLT-040-REPO-ROT-BAD-BEHAVIOR`, `crates/joil-evidence/registry/legacy-status-before-v4.json` (`sha256:5aed644888e43b8d9be6553cfcaed944edcf713f1c8cc162de74b761c39a9ba1`) | integrator | Hash-bound historical input referenced by `status-registry.json`; generated current status labels it historical. It is not an alternate active implementation. Keep bytes and reference intact; verify tampered evidence is rejected through the existing contract tests. Evidence review 2026-10-20. |
| `F-09` (`Wave P1b..P5`, deduplicated with `F-05, F-10`) | `HLT-005-PYTHON-PRODUCT-TRUTH`, `python/` (`sha256:a9ed62b3058bf6eb5f67d2e7da62834ba9e15f5c2d9c204386637655aaed707e`) | migration-integrator | Real active Python ownership remains; 24 additive Rust CLI ports do not establish consumer cutover or source retirement. I024 starts P1b consumer work; review 2026-10-22. Preserve scientific and failure semantics throughout P2–P5. |
| `F-10` (`Wave P1b..P5`, deduplicated with `F-05, F-09`) | `HLT-005-PYTHON-PRODUCT-TRUTH`, `python/ai-service` (`sha256:7a9fc469103242b8868da3eb64c804abdc4a1c176d2b6d9baac44aa6b616e1fc`) | migration-integrator | Companion containment score for the same legacy surface. No new Python implementation or invented ML exception is authorized. Close only after source retirement and a fresh standard audit. P1b review 2026-10-22; final P5 target 2026-11-22. |

## V5-00-03 audit-hold and defect-gate reconciliation (`I024-S1`)

Slice `I024-S1` reconciles the V5 software ledger against current `main`
without altering historical receipts, frozen source digests or scientific
admission ([v5-00-03-reconciliation-20261019.json](receipts/v5-00-03-reconciliation-20261019.json)):

- **Recorded complete (raw plan ledger)**: `160/160` tasks (`32/32` packets recorded `done` in `docs/v5-plan/progress/V5-*.json`).
- **Independently verified software-complete**: `150/160` tasks (`22/32` packets without an audit-hold note or synthetic-fixture substitute).
- **Audit-held / baseline-qualified (`1/160` task in `1/32` packet: `V5-00-03` in `V5-00`)**: All `16/16` `source_only_inventory` rows in `docs/v5-plan/execution/DEFECT_ACCEPTANCE_GATES.json` now bind their committed post-baseline software positive/red control receipts (`12/16` runnable inside `tools/v5_baseline_source_controls.py` including `D-07` and `D-25`; `4/16` rows `D-18`, `D-21`, `D-28`, `D-29` remain `kind: "blocked"` inside `v5_baseline_source_controls.py` on the literal frozen `e1e9203` component and are covered by dedicated software control harnesses `v5_d18_scheduler_control.py`, `v5_d21_trajectory_control.py`, `v5_d28_ownership_control.py`, `v5_d29_blind_judge_control.py` and owner disposition `v5-00-owner-disposition-D28-D29-20261018.json`). `V5-00-03` retains its explicit audit-hold qualification while `D-21` awaits a real native subject/time/field-bound trajectory manifest (`owner: codex-v5-integrator`) and `D-29` oversized-module design debt remains open under `F-01` (`owner: rust-integrator`, review `2026-10-22`).
- **Deferred physical / synthetic-fixture (`9/160` tasks across `9/32` packets under `SC-DEC-071` in `docs/release/v5/FLEET_ADMIRAL_DISPOSITIONS_20261018.json` + `12` v4 carry-forward deferrals in `docs/v5-plan/execution/V4_DEFERRALS.json` including `TR-16`, `TR-16-threshold-audit`, `EV-09`, `EV-12`; `scientific_admission = false`)**: All physical/scientific release gates remain `incomplete` in `docs/STATUS.json`.

## Replay and acceptance

From the checkout root, run compute through the acknowledged fleet cap with
the normal lane limits. These are the `Justfile` score and required recipes;
calling their shell entry points directly avoids the Snap launcher moving
processes out of the capped scope. Pass the caller's tool path explicitly:

```sh
/home/ubuntu/xbabe2_kicker/fleetcap/run-capped --name joil-score -- env PATH="$PATH" bash ops/ci/audit.sh
/home/ubuntu/xbabe2_kicker/fleetcap/run-capped --name joil-required -- env PATH="$PATH" bash scripts/ci-local.sh
```

For conformance use `jankurai audit . --mode standard`, retaining fresh JSON,
Markdown and repair-queue files in ignored storage. Standard mode must still
fail while blockers remain. Compare findings by rule, path and fingerprint,
and separately report local scratch findings and tracked-source findings.

Before the next packet, read this register, the I023 receipt, the migration
annex and live coordination. Claim exact consumer paths before editing; retain
all existing negative controls and frozen-source identities. An unresolved
parity result, scope collision, exceeded lease or scientific-admission change
stops the affected work. A passing source audit alone never validates physics.
