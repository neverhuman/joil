# PR #3 final review (DOC-09)

Reviewer: DOC lane (`joil_paper`, Claude Opus). Final revision, 2026-10-16, against integration head `e68d617`
(PR #3 head; merged into `lane/paper`). Earlier drafts reviewed `0139ca0` and `cab01e8`. This review never merges
the PR; Jepson merges.

## 1. Scope and method

- **Read and edited by DOC:** README, WHITEPAPER, SCIENCE, GENOME (plus `docs/design/GENOME_V4.md`), EVIDENCE,
  the review synthesis and the numbers trace.
- **Built and owned by DOC:** the claims machinery in `tools/claims_audit.py`:
  - review-input checksums and finding traceability;
  - ledger-derived status and workplan checkboxes;
  - the wording rules;
  - the README status block;
  - the citation-bound numbers trace.
- **Code read against claims:**
  - Rust: `program.rs`, `phenotype.rs`, `cfd_case.rs`, `evolve.rs`, `structure.rs`, `joil-evidence`, and the
    parts of `joil-ir` the documents cite;
  - `physics/cuda/joil_cuda_eval.cu`;
  - the CAD and enactment tools, and the CFD/GCI scripts.
- **Re-run by DOC:** the IR-R1/IR-R2 slice and path checks, and `prepare-cfd-case` on a historical elite (DOC-07
  review, Resolution).
- **Independent number audits:**
  - EV recomputed the Wave-4 and Wave-4 GCI receipts and found no arithmetic mismatch at the serialized precision
    (`docs/reviews/2026-10-16-EV-wave4-number-trace.md`).
  - TR audited every number in README and WHITEPAPER against receipts and code; DOC resolved all `30` findings
    (`docs/reviews/2026-10-16-TR-deliverable-numbers-audit.md`).
  - IR checked the figures and captions (`docs/reviews/2026-10-16-IR-deliverable-check.md`).
- **Not reviewed line by line:** the whole diff. Large result JSONs and generated files were checked only through
  the receipts and hashes the documents cite.

## 2. Workplan §4 gates

| Gate | State | Evidence |
|---|---|---|
| `python3 tools/check.py` all stages exit 0 | pass, `10/10` stages, on `lane/paper` `745e74d` and on integration (INT receipt for `e68d617`) | ignored `.private/checks/` receipts |
| `cargo test --workspace`; `cargo clippy -D warnings` | pass (inside `check.py`) | same receipt |
| No `baseline_ct35()` fallback | **pass in substance**: the fallback pattern `unwrap_or_else(\|_\| … baseline_ct35` has 0 matches. The literal §4 grep still lists calls; each one is a reference evaluation, the explicit `insert_baseline_control` operator, seed construction, an edit-script base or `#[cfg(test)]` code. | INT-03 smoke audit: 0 disallowed fallbacks |
| claims_audit: 0 untraced numerals in README/WHITEPAPER abstract | **pass, exceeded**. The strict trace covers all of README and WHITEPAPER with 0 untraced. A numeral must match the receipt its sentence, caption or table column cites, or a checked declaration (literal, derived, count). | `docs/NUMBERS_TRACE.md` |
| GitHub Actions green on PR head | **pass**: `9/9` green on `e68d617` (`fmt`, `clippy`, `rust-tests`, `baseline`, `python-tests`, `public-check`, `claims-audit`, `ui`, `foundation`). | PR #3 checks |

## 3. What this PR achieves (supported by merged code and tests)

- **Truth repair.**
  - No silent baseline fallback.
  - Variation is transactional, with repair records and invalid/repaired/evaluated budget accounting.
  - Schema identity is preserved.
  - GCI uses ordinary division, validity states and no clamp.
  - Load transfer is integrated independently, and structural failure is a hard constraint.
  - Pareto hypervolume is a true union.
  - Monte Carlo tails report n and confidence intervals, and fail closed below the needed n.
  - Thresholds come from the requirement file, and the screen is named an analytic structural screen.
- **Evidence kernel.** Content identity, typed observations, a claim DAG with revocation, sandbox-only campaign
  admission, generated status, and surrogate abstention without independent labels.
- **DesignIR v1 and one CFD subject.**
  - The PR adds gene-to-IR coverage, a backend support matrix, voxel F0, split metrics, origin traces and a
    negative corpus.
  - Since IR-R1 the OpenFOAM writer emits the same `IrCompiledMesh` its enactment receipt describes: split ducts,
    ramp and skimmer, with ribbon and TPMS labelled wall-corrugation proxies.
- **Wave-4 study (INT-04).** The Wave-3 design was re-run on repaired operators with every proposal charged. On
  best screen score, uniform random search is not beaten (WHITEPAPER §5.7). The documents report this reversal
  as the main search result.
- **Same-pipeline CFD control (INT-06, G-09).** The archived shells match a `CT-35` case built by the same
  pipeline, so the published pressure-drop reductions were a pipeline offset. The documents say so (correction C2,
  §4.6 and its figure).
- **Documentation.**
  - Claims are re-stated at their evidence tier, with a lead figure showing the Wave-4 result.
  - Stale or unsupported numbers are corrected or withdrawn, including all `30` items of TR's audit.
  - Every numeral in README and WHITEPAPER is bound to its cited receipt or to a checked declaration.

## 4. Open items at sign-off

| # | Item | Owner | Blocking? |
|---|---|---|---|
| O-1 | ~~CI on the PR head~~. `9/9` green on `e68d617`. | - | closed |
| O-2 | ~~DOC-07 IR-R1 (CFD writer)~~. Resolved at `f49f67e`; DOC verified it by re-slicing the CLI-written STL. | - | closed |
| O-3 | ~~DOC-07 IR-R2..R5 and G-11~~. Resolved at `cab01e8`. Residuals: the exact-baseline branch asserts its Betti numbers, and two derived-taxon fields of the Wave-4 winner are inconsistent (reported to IR; not quoted). | IR | no |
| O-4 | ~~INT-04 Wave-4 study~~. Done (`cb31ba3`) and documented in WHITEPAPER §5.7 and the README. | - | closed |
| O-5 | ~~INT-06 / G-09~~. Done in scope (`docs/results/wave4-gci-v1.json`); documented in WHITEPAPER §4.6, correction C2, Abstract item 3 and README. One Wave-4 winner was solved, but it is not grid-converged. Of the other five, one mesh pinched off, one meshed but was never solved, and three were never meshed (logs read by EV and DOC). The rest of the sweep is §6. | - | closed |
| O-6 | TR-12 and EV-06 were closed on the workplan §5 decision 8–9 defaults (INT confirmation 07:02Z). Homologous crossover and physical admission go to §6. | Jepson may override | no |
| O-7 | Evidence-kernel notes R-1..R-3 (point-value pass/fail, upper-bound-only criteria, F2 sub-levels) | EV | no |
| O-8 | INT-generated figures. Plates 2, 8 and 9 keep baked-in stale text (G-07). Plates 3 and 10 still draw withdrawn v1 standard GCI values; their captions say so. The Wave-4 hero and Plate 11 have layout overlaps (IR). DOC forwarded all of these to INT. | INT | no |
| O-10 | `wave4-gci-v1.json` labels all five unsolved winners as G0 pinch-offs, and its G-09 text quotes an archived value hard-coded in `tools/run_wave4_gci_sweep.py` that no receipt holds. The documents use the run logs and the receipt values instead (NEED to INT). | INT | no |
| O-11 | Work still running at sign-off. INT is solving the promoted Wave-4 design `s20261108-g016-i000` (one `G0` solve with a different inlet refinement; `G1` failed again). TR finished a full 10-seed Wave-4 reproduction with raw capture. EV reports that it matches the committed Wave-4 receipt with `0` mismatches over `14,243` values (board, 10:11Z); the public receipts for this were not yet merged at `e68d617`. The INT solve is not in a committed receipt. Any new receipt needs a DOC update before its numbers are cited. | INT, TR, EV | no |
| O-9 | Ledger hygiene. INT recorded `MERGED` for EV-09, EV-10, EV-12 and TR-16 with no `DONE` from the owner, and EV stated that `MERGED EV-09` is not completion. The audit now counts `MERGED` only after `DONE`; existing rows were not downgraded. | owners | no |

## 5. Residual risks after this PR (for Jepson)

1. **Published Wave-1/2/3 search results predate the repairs.** They remain only as historical receipts. Wave-4
   shows that the Wave-3 headline (`B3` beats random search) did not survive the repairs.
2. **The search screen is still prior-laden.** The sandbox `F1` evaluator scores with gene-derived descriptors, so
   random draws over all labels score high at once. No physics-observed screen exists yet.
3. **GP-CEI no longer filters anything.** With every proposal charged, the acquisition only orders a batch that is
   fully evaluated. Any future claim about the surrogate needs a design in which it chooses.
4. **No CFD pressure-drop advantage has been shown.** A same-pipeline `CT-35` control matches the archived
   two-duct shells; the published reductions were a pipeline offset (correction C2). The one Wave-4 design simulated
   with split ducts is not grid-converged, and its fine-grid value is higher than the control's.
5. **No oil, transient or physical evidence.** `F2-Oil`, `F3` and `F4` have not run, physical validation is false,
   and campaigns are sandbox-only.
6. **No independent surrogate labels.** OOD metrics are "insufficient evidence" by design, and the per-label
   KPI error is `null`.
7. **The Wave-4 CFD runs have incomplete case identity.** EV's every-run audit listed `12` case directories at
   09:54Z (`3` active cases pass, `9` quarantined cases fail). Five solved meshes were pruned before capture, and one
   retained case no longer matched its recorded hash. By 10:15Z that failed case had been removed (`11` directories,
   `3` pass, `8` fail); EV records the removal as neither a pass nor a recovery. Several solved runs cannot be
   re-bound to their exact meshes (EV-12 blocked for those runs).
8. **Audit limits.**
   - The public Wave-4 receipt drops per-proposal repair records, so individual repairs and pedigrees cannot be
     audited from it.
   - The reported p-values use the normal approximation without continuity correction (EV).
9. **The genome is still the bounded macro language.** Open-ended invention (genome v4, QD over measured
   descriptors, mechanism cards) is a §6 follow-on.

## 6. Sign-off

**Signed off for documentation at integration head `e68d617`** (CI `9/9` green; `tools/check.py` `10/10`; claims audit
and numbers trace clean). The documents state what the merged code and committed receipts support, at its
evidence tier, and the residual risks above are disclosed in them.

This sign-off does not merge the PR; merging is Jepson's decision. It covers `e68d617` only. Any later commit must
keep `tools/check.py` and CI green, and a new receipt (O-11) must be documented by DOC before its numbers are
quoted.

## 7. Updates after sign-off (through integration `689eabc`)

The sign-off above covers `e68d617`. DOC documented these later changes; each was gated (`tools/check.py` `10/10`)
before push.

- **Residual risk 7 (Wave-4 case identity) is now resolved in scope and sharpened.** EV-12 closed (`eeee9f9`, merged
  `689eabc`; `crates/joil-evidence/receipts/EV12-Wave4-closure.json`). All `12` Wave-4 case directories are accounted.
  Only `3` keep an exact artifact chain: the control's `G1`, plus two `G0` cases that never solved. The meshes behind the
  control's `G0`/`G2` and all three grids of the solved design `s20261106-g023-i001` were not kept. The `G2` comparison
  behind correction C2 is therefore an incomplete historical diagnostic, and WHITEPAPER §4.6 now says so. No case is
  claim-bearing.
- **Independent number audit (TR NUM-TRACE, lane `truth` `6233ccd`).** TR checked `1,755` numeral occurrences at
  `82bc814` and grouped the exceptions into `14` findings. DOC re-checked each against receipts and code, then applied
  all of them. Examples:
  - executed held-out duties used other feed medians than the preregistration;
  - Monte Carlo ensembles are uniform, not LHS;
  - no 30-sample negative-control count exists;
  - the preregistration sets no false-omission threshold;
  - the `F4` table is a proposal, not a lock.
  The audit also removed a probe classification that is in no receipt. It also showed that several percentages had
  matched unrelated receipt fields by coincidence; those are now declared as derived.
- **README.** The README is now a short page: what/why, Figure 0, status, install, quick start and links. The tables
  and evidence tiers moved unchanged to `docs/RESULTS.md`, which the claims audit and numbers trace also cover. IR ran
  every README command from a fresh clone, and all exited `0` (receipt on lane `ir`, `be1365a`). DOC applied IR's fixes:
  - `rtk proxy` for the two `--nocapture` tests;
  - a plain timing note;
  - a warning that `--verify-existing` prints the withdrawn historical "certified" line.
- **Figures.** Plate 10 now reads $R$ from its receipts (INT `e62a8e9`). Plate 9 still prints a `0/30` negative-control
  label that no receipt supports; its caption flags this (NEED to INT).
- **Ledger.** TR-16 and EV-09 are moved to §6 (INT `MOVED-§6`). INT-01, DOC-09 and EV-12 are merged. The workplan
  stands at `56/59`; INT-02, INT-08 and DOC-10 are open.
