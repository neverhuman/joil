# V5 closure audit — 19 October 2026

## Decision

The software task ledger records **160/160 tasks** and **32/32 packets** done.
This is not completion of the original scientific programme. The existing
qualification state in `docs/STATUS.json` remains authoritative: sandbox only,
no qualified physical capacity gain, and no qualified industrial candidate.
Do not replace missing measurements, native solver evidence or owner signatures
with software test results, proxy approvals or synthetic fixtures.

This audit started on receiving main
`5716c0a0e36bc69b21bf24b58dd20779cd0fbe9f`, after PR #48 was rebase-merged
with its exact head approved and all eleven hosted checks successful.
PR #48's head `6d85850efcd7ef0bed899cb2b843021e4091896e` and receiving main
have the identical tree `cbe27d40d1c1a4e1383dfb0736744e1d89517212`.

## Evidence reconciliation

- The generated plan checker passes: 32 packets, 160 task records, ten source
  inputs and 31 defect records. All 259 referenced evidence hashes match.
- The preceding independent audit in `docs/JANKURAI_FINDINGS.md` distinguishes
  150 software-verified tasks, one audit-held task and nine physical tasks
  accepted with explicitly limited synthetic substitutes. These are historical
  audit categories, not a new physical qualification by this audit.
- Eighteen receipt references name two tested commits unavailable in a fresh
  public clone: `59acb0831f406d511f2b3c3b70e185c91e351e29` and
  `ee70637afdb03b6155ad207ca86b1a28162d0f24`. Both objects were recovered,
  pinned with custody refs and included in a privately retained all-ref bundle.
  Do not claim those original executions are independently reconstructible from
  the public repository alone.
- All five owner signature lines in `OWNER_APPROVAL_DOSSIERS.md` remain unsigned.
  Delegated software and budget decisions do not supply those signatures.
- The earlier `V5-00-03` hold includes a real native subject/time/field-bound
  trajectory manifest dependency. The subsequent I024 split series addressed
  module size debt; it did not produce the missing physical trajectory data.

## Local CI failure and bounded repair

The first complete local CI attempt failed in
`crates/crucible-runtime/tests/recovery.rs`:
`charged_preparation_does_not_consume_native_deadline_and_real_slow_child_still_times_out`.
The expected fast child was recorded as timed out. The second failure was a
poisoned shared test mutex after that panic. Formatting, Clippy and generated
status checks had passed; later suites were not reached in that attempt.

The retained scratch contains a valid score-25 output, with no accepted output
in the database. Lease expiry and scratch exhaustion do not explain the failure.
An unchanged isolated rerun passed. The original test allowed only 300 ms for
Python startup, an 80 ms sleep, the native fixture and durable output writes.
The available evidence does not identify which operation crossed the deadline.

The repair changes **only the test harness** to a two-second execution deadline,
three-second charged preparation delay and four-second slow child. Preparation
still exceeds the execution deadline, so incorrectly counting preparation would
fail the fast case. The slow child still independently exceeds that deadline,
must time out and must leave no accepted object. Wall/CPU charging assertions
remain. Production timeout behavior, resource limits, scientific acceptance
thresholds and evidence identities are unchanged.

Required acceptance before publishing this change: the targeted positive and
timeout cases, the complete local CI command `bash scripts/ci-local.sh`, the
publication scan and independent review on the resulting source. Each execution
must retain its command, exit status, source hashes and log hashes. A previous
hosted pass or isolated rerun does not replace that final run.

## Git and worktree custody

An all-ref bundle preserves 146 refs, including the two recovered tested
commits. Its SHA-256 is
`84ec34c4c7f1255e29cedabb148398bc60acb658502bc6871010b14e229c19e3`.
An independent mirror restore passed `git fsck --full --strict` and all 146
restored ref values matched. A second bundle copy was hash-verified separately.

Seven clean inactive worktrees and 54 remote branches were retired after that
verification. Patch-equivalent history was reconciled to main. Superseded
drafts were archived with explicit dispositions, rather than merged over later
corrections. In particular, an unpublished failed-attempt receipt remains in
private custody; it was not converted into accepted evidence.

Ignored evidence was moved atomically into custody, with file identity and
metadata checked before normal `git worktree remove`. This removes active
checkout sprawl; it does not claim that retained evidence disk space was freed.
The remaining primary checkout, active integration checkout and audit checkout
are not reported as cleaned while they remain occupied or dirty. Final runtime
inventory and any additional safe retirement belong in the closing receipt.
No private raw evidence, host inventory or archival bundle is public Git content.

## Gates that remain open

| Gate | Current evidence | Required completion evidence / responsible role |
| --- | --- | --- |
| Native trajectories (`V5-00-03` / D21) | Software accessions and synthetic examples | Native subject, time, field and output-bound trajectory manifest; numerical/evidence owner |
| Native restart and numerical qualification (`V5-14`) | Software restart harness | Real native multi-grid/model/restart runs, conservative balances and declared tolerances; numerical owner |
| Independent physical validation (`V5-15` and related deferred packets) | Synthetic accession/proxy checks | Source-bound measurements, uncertainty, held-out conditions and applicability envelope; validation owner |
| Industrial geometry and operation | Research reconstruction and contracts | Measured as-built interfaces, material/process/rig and service evidence; responsible engineering owners |
| Owner approvals | Five unsigned dossiers | Actual applicable owner signatures bound to the reviewed evidence; named dossier owners |
| Standard source conformance | Pinned Jankurai 1.7.5 returned a nonzero result | Rerun after cleanup; resolve findings or retain explicit reviewed limitations without calling the scanner green; source integrator |
| Python migration | Approved staged plan; Python remains active | Execute the remaining phases in `PY_TO_RUST_PLAN.md`; retain declared exceptions and parity evidence; migration owners |
| Final software acceptance | Initial full local CI failed | Complete source-bound CI and independent red-team verdict after repair; sole integrator and reviewer |

The original V4 deferrals, including TR16, threshold audit, EV09 and EV12, retain
their recorded evidence requirements. This table consolidates dependencies;
it does not create another task queue or rewrite completed receipt history.

## Handoff and stop conditions

Use `docs/MASTER_PLAN.md`, the existing V5 task registry and migration plan;
do not fork a competing master plan. Keep one integration PR active at a time.
The restored Captain reviews evidence on Codex Sol with x-high reasoning; the
integrator owns source edits, bounded local CI, merge and cleanup. Preserve
the existing coordination and resource-allocation receipts.

Stop the affected action on source drift, custody mismatch, an occupied/dirty
cleanup target, resource pressure, a failed test or an unresolved review finding.
Retain the failure. Never claim the scientific programme complete while any
required native, measurement or signature gate above is missing. A software-only
review verdict must state that scope explicitly.
