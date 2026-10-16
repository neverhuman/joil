# JOIL live coordination board (real-time)

Plan: `docs/WORKPLAN-2026-10-15.md` · Integration branch: `feat/joil-v4-truth-evidence` · Integrator: `agy_joil`
Rules: re-read this file before every edit batch. Append-only except your own lane row. Times UTC ISO.
Commit board changes on YOUR lane branch only for your row/locks; integrator reconciles. For instant visibility also append to the shared live copy `/home/ubuntu/joil-wt/JOIL-COORD.live.md` (not in git; everyone writes, `flock` it: `flock /home/ubuntu/joil-wt/.coord.lock -c 'echo "..." >> /home/ubuntu/joil-wt/JOIL-COORD.live.md'`).

## Lanes
| Lane | Screen | CLI | Worktree | Branch | Status | Last heartbeat |
|---|---|---|---|---|---|---|
| INT | agy_joil | agy Gemini 4 Argon high | /home/ubuntu/joil | feat/joil-v4-truth-evidence | active (INT-01/03/04/05 DONE, CI-MATPLOTLIB/IR-R7/TR-10/DOC-04 merged, INT-06 next) | 2026-10-16T06:50:00Z |
| TR | joil_truth | codex gpt-6.1-sol xhigh | /home/ubuntu/joil-wt/truth | lane/truth | active (TR-05/06 DONE, TR-01 WIP) | 2026-10-16T05:16:57Z |
| EV | `joil_evidence` | codex gpt-6.1-sol xhigh | /home/ubuntu/joil-wt/evidence | lane/evidence | CI-MATPLOTLIB verified; EV-06 physical admission BLOCKED; prior lane handoff retained | 2026-10-16T06:45:36Z |
| IR | joil_ir | agy Gemini | /home/ubuntu/joil-wt/ir | lane/ir | active (IR-01 WIP) | 2026-10-16T05:16:50Z |
| DOC | joil_paper | claude Opus xhigh | /home/ubuntu/joil-wt/paper | lane/paper | active (DOC-01 DONE, DOC-02 WIP) | 2026-10-16T05:13:41Z |

## Heartbeat protocol
Every ≤30 min (and on each item state change) append to the live file:
`HB <UTC> <lane> <item-id> <state: START|WIP|BLOCKED|DONE> <sha|-> <one-line note>`
No heartbeat for 60 min ⇒ the drive routine nudges the screen; 2 h ⇒ its locks expire.

## File locks (shared files only; lane-owned files need no lock — see workplan §1)
`LOCK <UTC> <lane> <path> <reason>` / `UNLOCK <UTC> <lane> <path>`
Shared: Cargo.toml, Cargo.lock, crates/joil-core/src/lib.rs, crates/joil-core/Cargo.toml, tools/check.py, .github/**, COMMS.md, COORD.md, runs/** metadata.

## Handoffs
`NEED <UTC> <from> -> <to> <item> <what>` · `GIVE <UTC> <from> -> <to> <item> <sha/path>`
Known handoffs: EV-03 types → TR-17, IR-07, EV-08 · EV-02 identity → TR-13, IR-10, EV-12 · EV-06 requirements → TR-16, INT-04 · TR-07 → INT-05 · TR-01..03 + IR-02 → INT-03/04 · IR-05/TR-10/TR-14 wording → DOC-03.

## Merge protocol
Lane: push lane branch, append `DONE <item> <sha>`. Integrator: `git merge --no-ff lane/<x>`, run `python3 tools/check.py`, push, append `MERGED <item> <sha>`. Lanes then `git merge feat/joil-v4-truth-evidence` into their lane (never rebase after pushing, never force-push). Order: workplan §3.

## Compute
OpenFOAM: INT only, ≤16 cores. Cargo `-j 8` per lane, pytest `-n 4`. GPU: IR only. Announce any job >30 min as `JOB <UTC> <lane> <cores> <eta> <cmd>`.

## Hard rules
Never merge the PR (Jepson merges). No force-push, no --admin, no squash. Push as jepsontaylor over HTTPS. Never touch xbabe0. Don't edit another lane's owned files — send NEED instead.

HB 2026-10-16T05:14:11Z EV EV-01 START - claim evidence kernel; EV-02/03/04 next; CPU <=8 cargo, no GPU
LOCK 2026-10-16T05:14:11Z EV Cargo.toml register joil-evidence workspace member
LOCK 2026-10-16T05:14:11Z EV Cargo.lock resolve joil-evidence workspace package

LOCK 2026-10-16T05:21:06Z EV Cargo.lock add TOML requirement parser; release with next commit
LOCK 2026-10-16T05:21:06Z EV tools/check.py add generated status/objective drift verification stage; no workflow edits needed (run via existing python-tests suite)

LOCK 2026-10-16T05:23:57Z EV tests/test_check.py update expected required stage count for generated-status check; add real stale-status/objective-mismatch negatives in EV-owned crate tests
UNLOCK 2026-10-16T05:46:21Z EV Cargo.toml
UNLOCK 2026-10-16T05:46:21Z EV Cargo.lock
UNLOCK 2026-10-16T05:46:21Z EV tools/check.py
UNLOCK 2026-10-16T05:46:21Z EV tests/test_check.py
UNLOCK 2026-10-16T05:53:40Z EV tools/gen_status.py
LOCK 2026-10-16T06:40:46Z EV .github/workflows/foundation.yml fix missing pinned Matplotlib in CI python-tests; project-local venv only
UNLOCK 2026-10-16T06:45:36Z EV .github/workflows/foundation.yml pinned matplotlib==3.10.8; isolated regression + PNG render + full gate 10/10 + actionlint PASS; receipt crates/joil-evidence/receipts/CI-matplotlib.json
