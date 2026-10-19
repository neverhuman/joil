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

| Rule and path | Owner | Evidence, action and review target |
| --- | --- | --- |
| `HLT-001-DEAD-MARKER:shape`, repository | rust-integrator | Largest authored module is `crates/joil-ir/src/lib.rs`. I024–I025: split IR types, compilation, observation and corpus responsibilities into semantic modules with existing API and fixture parity; then address other oversized modules. Review 2026-10-22; Python replacements follow P2–P4. |
| `HLT-034-CI-BAD-BEHAVIOR`, `.github/workflows/foundation.yml` | captain / integrator | The workflow denies fork PR jobs and uses the disposable VM pool described in [CI.md](CI.md). The detector treats the mere presence of PR events and self-hosted labels as privileged access. Retain guard and pool contract; captain must verify runner isolation and dispatch policy before considering a scoped exception. Review 2026-10-20. |
| `HLT-034-CI-BAD-BEHAVIOR`, `.github/workflows/jankurai.yml` | captain / integrator | Same same-repository guard and disposable runner contract. Do not remove PR auditing or move to an unauthorized runner to clear the detector. Isolation/dispatch review 2026-10-20. |
| `HLT-034-CI-BAD-BEHAVIOR`, `.github/workflows/security.yml` | captain / integrator | Same same-repository guard and disposable runner contract; read-only token and disabled checkout credential persistence remain enforced. Isolation/dispatch review 2026-10-20. |
| `HLT-000-SCORE-DIMENSION:stack`, `analysis/__init__.py` | migration-integrator | Real legacy Python surface. Follow [PY_TO_RUST_PLAN.md](../PY_TO_RUST_PLAN.md): enumerate callers, prove Rust parity, cut over consumers and only then retire sources. P1b review 2026-10-22; subsequent waves retain their published dates. |
| `HLT-008-FALSE-GREEN-RISK`, `crates/joil-core/tests/truth_ev12_capture.py` | rust-integrator / integrator | This is an input-retention CLI, not a skipped test. `SystemExit(main())` propagates its failure status. Its separate `truth_ev12_capture_test.py` checks missing identity, tampered bytes, symlinks, nested/existing destinations and CLI failure. Historical handoff pins the helper's bytes; preserve them. Review the replay evidence by 2026-10-20, with retirement subject to migration parity. |
| `HLT-027-HUMAN-REVIEW-EVIDENCE-GAP`, `crates/joil-core/tests/truth_numtrace_audit.json` | integrator | Frozen audit distinguishes authored declarations from measured facts, lists source hashes and a pinned integration commit, and embeds `recheck_python` / `recheck_command`. Retain its negative findings. Recheck source hashes, arithmetic and planted negative controls; never rewrite historical claims or hashes to clear a detector. Evidence review 2026-10-20. |
| `HLT-040-REPO-ROT-BAD-BEHAVIOR`, `crates/joil-evidence/registry/legacy-status-before-v4.json` | integrator | Hash-bound historical input referenced by `status-registry.json`; generated current status labels it historical. It is not an alternate active implementation. Keep bytes and reference intact; verify tampered evidence is rejected through the existing contract tests. Evidence review 2026-10-20. |
| `HLT-005-PYTHON-PRODUCT-TRUTH`, `python/` | migration-integrator | Real active Python ownership remains; 24 additive Rust CLI ports do not establish consumer cutover or source retirement. I024 starts P1b consumer work; review 2026-10-22. Preserve scientific and failure semantics throughout P2–P5. |
| `HLT-005-PYTHON-PRODUCT-TRUTH`, `python/ai-service` | migration-integrator | Companion containment score for the same legacy surface. No new Python implementation or invented ML exception is authorized. Close only after source retirement and a fresh standard audit. P1b review 2026-10-22; final P5 target 2026-11-22. |

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
