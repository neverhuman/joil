# PR #3 final review (DOC-09): DRAFT, not yet signed off

Reviewer: DOC lane (`joil_paper`, Claude Opus). This revision was written 2026-10-16 against integration head
`cab01e8`, merged into `lane/paper`. The first draft was written against `0139ca0`. This review never merges the
PR; Jepson merges. It is finalised when the open items in §4 close or are moved to workplan §6 with a reason
(INT-08).

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
- **Not reviewed line by line:** the whole diff. Large result JSONs and generated files were checked only through
  the receipts and hashes the documents cite. EV recomputed the Wave-4 receipt independently
  (`docs/reviews/2026-10-16-EV-wave4-number-trace.md`) and found no arithmetic mismatch at the serialized precision.

## 2. Workplan §4 gates

| Gate | State | Evidence |
|---|---|---|
| `python3 tools/check.py` all stages exit 0 | pass on `lane/paper` (see the DOC-04 handoff receipt on the board) | ignored `.private/checks/` receipt |
| `cargo test --workspace`; `cargo clippy -D warnings` | pass (inside `check.py`) | same receipt |
| No `baseline_ct35()` fallback | **pass in substance**: the fallback pattern `unwrap_or_else(\|_\| … baseline_ct35` has 0 matches. The literal §4 grep still lists calls; each one is a reference evaluation, the explicit `insert_baseline_control` operator, seed construction, an edit-script base or `#[cfg(test)]` code. | INT-03 smoke audit: 0 disallowed fallbacks |
| claims_audit: 0 untraced numerals in README/WHITEPAPER abstract | **pass**. The trace covers README and WHITEPAPER "Read this first", Abstract and §5, with 0 untraced. Since this revision a numeral in a sentence or subsection that cites receipts must match one of those receipts. | `docs/NUMBERS_TRACE.md` |
| GitHub Actions green on PR head | `cb31ba3`: 9/9 green (INT). `db23f28`: `python-tests` and six other checks green, `rust-tests` still running when this was written. Must be re-checked on the final head. | PR #3 checks |

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
- **Documentation.**
  - Claims are re-stated at their evidence tier.
  - Stale or unsupported numbers are corrected or withdrawn.
  - Every numeral in the traced sections is bound to its cited receipt or to a checked declaration.

## 4. Open before sign-off

| # | Item | Owner | Blocking? |
|---|---|---|---|
| O-1 | CI on the final PR head. The matplotlib fix (`84d0c2e`) is merged and `python-tests` passes. | INT | yes, until green on the final head |
| O-2 | ~~DOC-07 IR-R1 (CFD writer)~~. Resolved at `f49f67e`; DOC verified it by re-slicing the CLI-written STL. | - | closed |
| O-3 | ~~DOC-07 IR-R2..R5 and G-11~~. Resolved at `cab01e8`. Residuals: the exact-baseline branch asserts its Betti numbers, and two derived-taxon fields of the Wave-4 winner are inconsistent (reported to IR; not quoted). | IR | no |
| O-4 | ~~INT-04 Wave-4 study~~. Done (`cb31ba3`) and documented in WHITEPAPER §5.7 and the README. | - | closed |
| O-5 | ~~INT-06 / G-09~~. Done in scope (`docs/results/wave4-gci-v1.json`); documented in WHITEPAPER §4.6, correction C2, Abstract item 3 and README. One Wave-4 winner was solved, but it is not grid-converged. Of the other five, one mesh pinched off, one meshed but was never solved, and three were never meshed (logs read by EV and DOC). The rest of the sweep is §6. | - | closed |
| O-6 | TR-12 and EV-06 were closed on the workplan §5 decision 8–9 defaults (INT confirmation 07:02Z). Homologous crossover and physical admission go to §6. | Jepson may override | no |
| O-7 | Evidence-kernel notes R-1..R-3 (point-value pass/fail, upper-bound-only criteria, F2 sub-levels) | EV | no |
| O-8 | Figure plates with baked-in stale text (G-07). The Wave-4 figures were regenerated and checked by DOC; Plates 2, 8 and 9 were not. | INT | no |
| O-10 | `wave4-gci-v1.json` labels all five unsolved winners as G0 pinch-offs, and its G-09 text quotes an archived value hard-coded in `tools/run_wave4_gci_sweep.py` that no receipt holds. The documents use the run logs and the receipt values instead (NEED to INT). | INT | no |
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
7. **Audit limits.**
   - The public Wave-4 receipt drops per-proposal repair records, so individual repairs and pedigrees cannot be
     audited from it.
   - The reported p-values use the normal approximation without continuity correction (EV).
8. **The genome is still the bounded macro language.** Open-ended invention (genome v4, QD over measured
   descriptors, mechanism cards) is a §6 follow-on.

## 6. Sign-off

Not signed off. The remaining blocker is O-1 (CI green on the final head). Once it closes, this review becomes the
PR #3 comment for Jepson.
