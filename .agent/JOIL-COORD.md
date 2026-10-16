# JOIL live coordination board (real-time)

Plan: `docs/WORKPLAN-2026-10-15.md` · Integration branch: `feat/joil-v4-truth-evidence` · Integrator: `agy_joil`
Rules: re-read this file before every edit batch. Append-only except your own lane row. Times UTC ISO.
Commit board changes on YOUR lane branch only for your row/locks; integrator reconciles. For instant visibility also append to the shared live copy `/home/ubuntu/joil-wt/JOIL-COORD.live.md` (not in git; everyone writes, `flock` it: `flock /home/ubuntu/joil-wt/.coord.lock -c 'echo "..." >> /home/ubuntu/joil-wt/JOIL-COORD.live.md'`).

## Lanes
| Lane | Screen | CLI | Worktree | Branch | Status | Last heartbeat |
|---|---|---|---|---|---|---|
| INT | agy_joil | agy Gemini 4 Argon high | /home/ubuntu/joil | feat/joil-v4-truth-evidence | active (INT-01/03/04/05 DONE, CI-MATPLOTLIB/IR-R7/TR-10/DOC-04 merged, INT-06 next) | 2026-10-16T06:50:00Z |
| TR | joil_truth | codex gpt-6.1-sol xhigh | /home/ubuntu/joil-wt/truth | lane/truth | active (TR-05/06 DONE, TR-01 WIP) | 2026-10-16T05:16:57Z |
| EV | `joil_evidence` | codex gpt-6.1-sol xhigh | /home/ubuntu/joil-wt/evidence | lane/evidence | EV12 Wave4 scoped DONE:all12 accounted/3 verified chains/9 exclusions; gate10/10; EV09 data-blocked | 2026-10-16T11:22:04Z |
| IR | joil_ir | agy Gemini | /home/ubuntu/joil-wt/ir | lane/ir | active (IR-01 WIP) | 2026-10-16T05:16:50Z |
| DOC | joil_paper | claude Opus xhigh | /home/ubuntu/joil-wt/paper | lane/paper | active (DOC-01..08 MERGED incl. DOC-04/07; DOC-09 final review draft, waits on final-head CI; DOC-10 ongoing: INT-06/G-09 documented, section 6 deferrals inserted) | 2026-10-16T08:40:00Z |

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
HB 2026-10-16T07:13:38Z EV EV-06 DONE-SANDBOX cb31ba3 INT 07:02Z scope confirmation; physical admission deferred to section6.
LOCK 2026-10-16T07:13:38Z EV docs/reviews/2026-10-16-EV-wave4-number-trace.md user-assigned independent Wave-4 arithmetic/publication review; seven gaps handed off to INT/DOC.
UNLOCK 2026-10-16T07:17:23Z EV docs/reviews/2026-10-16-EV-wave4-number-trace.md trace complete; independent arithmetic PASS; seven publication/evidence gap groups handed to INT/DOC; full tools/check.py10/10 PASS.
BLOCKED EV-09 independent source-bound Cout labels and reviewed observed mechanism inventory absent; real OOD/calibration deferred to section6; foundation merged.
DONE EV-10 dcaf087d9c1f028e8f65699a50b28c0461b89bb1
BLOCKED EV-12 current prepared-case subject bundle has volume_mesh=None and no full CaseManifest/ScientificIdentity; post-mesh, historical and FEA writer integration await IR/INT.
HB 2026-10-16T07:41:52Z EV LEDGER-FIGURE-REVERIFY WIP c5eb7f1 both requested PNG hashes and all manifest sources verified from remote blobs; latest DOC cf9df59 Wave4 table and section5.7 key numbers match audited source; no new numeric gap; full gate running.
HB 2026-10-16T08:16:12Z EV EV-12-WAVE4 WIP - scoped owner08:06 request; analysis.case_identity finalizer + every-run validator/tests implemented; actual meshes/writer integration still required; no DONE-SCOPED before nonempty actual-run audit.
GIVE 2026-10-16T08:16:12Z EV -> DOC EV09/EV12-SECTION6 crates/joil-evidence/reports/section6-ev09-ev12-deferrals.json; dataset joil-independent-oil-observations-v1 required/not collected; disjoint geometry-family and observed-mechanism holdouts; historical/FEA artifact identity recovery deferred.
HB 2026-10-16T08:34:39Z EV EV-12-WAVE4 BLOCKED - actual INT runs audit11 cases/0 full identities/11 errors;5 solved meshes pruned without captures. Corrected helper accepts distinct canonical JSON hash encodings; retained input construction passes on CT35G1 and two8jet G0 cases, no original metadata written.
GIVE 2026-10-16T08:34:39Z EV -> INT/DOC WAVE4-GCI-NUMBER-TRACE crates/joil-evidence/reports/wave4-gci-number-trace-v1.json;54618 numeric fields traced,54615 match,3 unsupported archived-source leaves; full machine trace retained ignored with SHA256 and executable reproducer. Status/observer semantics gaps remain explicitly handed off.
HB 2026-10-16T08:49:04Z EV EV12-CAPTURE-HARDENING WIP - origin integration cab01e8 already merged; malformed manifests/references and symlinks reject per case; every-wave4-directory audit includes RSM/prepared-only. Real retained3 input snapshots finalize/verify;8 missing-mesh cases reject; actual original manifest audit11 errors remains BLOCKED-SCOPED. One-page INT checklist: crates/joil-evidence/reports/EV12-capture-checklist.md.
HB 2026-10-16T08:49:04Z EV EV-LEDGER DONE - EV01/02/03/04/05/07/08/11 DONE; EV06 DONE-SANDBOX per INT07:02 scope; EV10 DONE runtime hook dcaf087d. EV09 independent labels/OOD and EV12 actual captures/historical/FEA remain BLOCKED-SCOPED; section6 copy delivered aea27ae and DOC08:14 insertion ACK. No workplan checkbox edits.
HB 2026-10-16T08:50:58Z EV EV12-CAPTURE-HARDENING DONE - full tools/check.py exit0 (10/10, actual retained-input regression enabled); gate receipt sha256=60e7ab7dfa385cb84e310206523c96243561888555f0471c67696b0f28c69f2a. Capture checklist and full receipt at crates/joil-evidence/reports/EV12-capture-checklist.md and receipts/EV12-capture-hardening.json. Original every-case audit deliberately remains failed11/11; no original metadata writes or PR merges.
HB 2026-10-16T09:21:10Z EV WAVE4-RAW-PROOF-RECOMPUTE BLOCKED 78a181c INT09:18 confirms no original raw candidate/population JSONL was emitted; actual CLI exits2 with recomputed/comparison null. New independent raw consumer +7 synthetic oracles/negatives tested; missing raw is not zero mismatches. No summary-derived values relabelled as raw.
GIVE 2026-10-16T09:21:10Z EV -> INT/IR EV12-RETAINED-CAPTURE verified3 current retained manifests against actual bytes; hashes agree with INT09:18 capture. Eight unfinalized/pruned/prepared-only cases are preserved in quarantine and remain explicit unresolved scope; no all-original-case completion claimed.
HB 2026-10-16T09:26:01Z EV WAVE4-RAW-CONSUMER DONE - script+7 synthetic oracle/negative tests and missing-source receipt complete; full tools/check.py exit0 all10 stages, gate receipt sha256=1a0a2fc87575d3e6d448a10e2a965ccce38a50858d91355f88c0d3304bef02e8. Requested original raw proof recomputation remains BLOCKED (INT09:18 confirms raw logs never emitted); values/mismatch count null.
HB 2026-10-16T09:49:11Z EV EV12-WRITER-REVIEW DONE - five executable NEEDs posted to IR (canonical bypass, wrong-case relative finalize, incorrect prune receipt, script imports/indentation); synthetic probes and read-only retained cases recorded in receipts/EV12-writer-review.json. No writer PASS while defects remain.
HB 2026-10-16T09:49:11Z EV EV12-EVERY-RUN-AUDIT WIP - canonical filename INT09:46 contract accepted;12 current active/quarantine case rows listed: active3 PASS, quarantine9 FAIL including newly failed G1 cellProc mismatch. Receipt includes exact hashes and missing-file/stale reasons; awaiting final INT DONE refresh.
GIVE 2026-10-16T09:49:11Z EV -> TR WAVE4-RAW-CAPTURE fold API1426d8d exact wrapper/report parity and raw residual/coverage test passed; consumer handles separate B0 comparison, family-keyed folds and explicit insufficient/null rows (9 tests PASS). Full lane gate now running before push.
HB 2026-10-16T09:54:10Z EV EV12-REVIEW-VERIFICATION DONE - full gate10/10 exit0; gate receipt SHA256=cd505c9411d400c62858e6dbcf2675651eeaac992418c58b5a054901160beefe; writer5 NEEDs and every-case12-row snapshot retained explicitly; API1426d8d + consumercf728a9 ready for TR. No overall EV12 closure or recovered-original-raw claim.
HB 2026-10-16T10:12:20Z EV WAVE4-RAW-CAPTURE-CONSUME WIP - NEW raw campaign independently manifest/JSONL bound; all14243 compared fields match both summaries after exact Rust arithmetic regression fix5f27fc7. Public number trace/receipt retained; authored-family scope gap and missing historical original raw stay explicit. Final gate pending.
HB 2026-10-16T10:16:52Z EV WAVE4-RAW-CAPTURE-CONSUME DONE - numerical reproduction14243 fields0diff against new +original summaries; all154 raw artifacts/SHA/JSONL bindings independently checked; exact source/executable identity verified. Gate10/10 SHAec08dff1d15aee172f55dd5486864620d39b601b7daa21a92be656be993bbae3. Authored-family-only/absent historical raw limitations retained; current11-case audit3 activePASS/8 quarantineFAIL plus prior G1 absent remains BLOCKED-SCOPED.
HB 2026-10-16T10:40:14Z EV EV12-WR-01..05 DONE - independent exact ae339b1 fixture passes11 checks +3 retained cases; old-source negative reproduces5 findings; corrupted parity with0 numbered findings now fails closed. Receipt crates/joil-evidence/receipts/EV12-writer-reverify-ae339b1.json. No branch merge or tracked IR edit.
HB 2026-10-16T10:40:14Z EV EV09 BLOCKED - declared independent dataset absent at EV/INT; eight existing synthetic admission/holdout/surrogate tests pass, no real fit/OOD metrics. Updated source-bound readiness receipt and existing section6 handoff retained for DOC; no workplan checkbox edits.
HB 2026-10-16T10:44:44Z EV EV12-REVERIFY-EV09-READINESS DONE - exact IR ae339b1 source independently clears5 findings, positive11checks/3cases and old-source/false-parity negatives; EV09 remains BLOCKED with null real OOD/calibration metrics and explicit section6 dataset/holdout handoff. Full gate10/10 exit0 SHA84ba0c554b842b495b21991fc154bf3a5048379f284856b62d208fa8b8706813. No branch/PR merges or new PR.
HB 2026-10-16T11:16:16Z EV EV12-DISPOSITION-AUDIT WIP - source-bound12-case audit0 unresolved;3 active scoped chains exact vs preserved input copies;8 missing meshes explicitly excluded with no reconstructed hashes;G1 failed non-claim diagnostic retained with stale sidecar disclosed. Four positive/negative tests including real roots PASS. Full gate pending; historical/FEA and broader claim admission remain section6.
HB 2026-10-16T11:22:04Z EV EV-12 DONE - scoped OWNER11:07 identity-or-disposition acceptance:11 preserved inputs +retained G1 diagnostic,12 accounted/0 unresolved,3 verified artifact chains/9 explicit non-claim exclusions. Strict manifest checks unchanged; no pruned hash backfill, G1 stale sidecar/chronology caveat preserved. Full gate10/10 SHAadf04cd75b7a5d9ffd00fd0a37c8326f848afbd20aa91b374bca007d4ba8a904 with actual roots and6 relevant tests unskipped; historical/FEA/broader claim admission remain section6.
