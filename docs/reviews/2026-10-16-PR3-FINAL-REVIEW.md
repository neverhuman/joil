# PR #3 final review (DOC-09): DRAFT, not yet signed off

Reviewer: DOC lane (`joil_paper`, Claude Opus). Draft written 2026-10-16 against integration head `0139ca0` and
`lane/paper` at `33fb686`. This review never merges the PR; Jepson merges. It is finalised once the open items
in §4 close or are moved to workplan §6 with a reason (INT-08).

## 1. Scope and method

- **Read and edited by DOC:** README, WHITEPAPER, SCIENCE, GENOME (plus `docs/design/GENOME_V4.md`), EVIDENCE,
  the review synthesis and the numbers trace.
- **Built and owned by DOC:** the claims machinery in `tools/claims_audit.py`:
  - review-input checksums and finding traceability;
  - ledger-derived status and workplan checkboxes;
  - the wording rules;
  - the README status block;
  - the strict numbers trace.
- **Code read against claims:** `program.rs`, `phenotype.rs`, `cfd_case.rs`, `evolve.rs`, `structure.rs`,
  `physics/cuda/joil_cuda_eval.cu`, `joil-evidence`, `joil-ir` (the parts the documents cite), the CAD and
  enactment tools, and the CFD/GCI scripts.
- **Not reviewed line by line:** the whole +93,916/−1,033 diff. Large result JSONs and generated files were checked
  only through the receipts and hashes the documents cite.

## 2. Workplan §4 gates

| Gate | State | Evidence |
|---|---|---|
| `python3 tools/check.py` all stages exit 0 | **pass on `lane/paper`** (`d08a4ed`; 10 stages) | ignored receipt `.private/checks/20261016T063314Z-3f8b8378/receipt.json` |
| `cargo test --workspace`; `cargo clippy -D warnings` | pass (inside `check.py`) | same receipt |
| No `baseline_ct35()` fallback | **pass in substance**: the fallback pattern `unwrap_or_else(\|_\| … baseline_ct35` has 0 matches. The literal §4 grep still lists 22 calls; each is a reference evaluation, the explicit `insert_baseline_control` operator, seed construction, an edit-script base or `#[cfg(test)]` code. | `evolve.rs`, `program.rs`, `surrogate.rs` call sites inspected; INT-03 smoke audit finds 0 disallowed fallbacks |
| claims_audit: 0 untraced numerals in README/WHITEPAPER abstract | **pass**: the strict trace covers README plus WHITEPAPER "Read this first", Abstract and §5, with 0 untraced | `docs/NUMBERS_TRACE.md` |
| GitHub Actions green on PR head | **fail** on `0139ca0`: `python-tests` errors in `test_evidence.test_tr07_int05_python_gci_order_and_wave2_v2_reports` (`ModuleNotFoundError: matplotlib` on the runner). All DOC tests pass in that run. | run `38860402402`; NEED posted to INT/TR |

## 3. What this PR achieves (supported by merged code and tests)

- **Truth repair.**
  - No silent baseline fallback.
  - Variation is transactional, with repair records and invalid/repaired/evaluated budget accounting.
  - Schema identity is preserved.
  - GCI uses ordinary division, validity states and no clamp.
  - Load transfer is integrated independently, and structural failure is a hard constraint.
  - Pareto hypervolume is a true union.
  - Monte Carlo tails report n and confidence intervals.
  - Thresholds come from the requirement file, and the screen is named an analytic structural screen.
- **Evidence kernel.** Content identity, typed observations, a claim DAG with revocation, sandbox-only campaign
  admission, generated status, and surrogate abstention without independent labels.
- **DesignIR v1.** Gene-to-IR coverage, backend support matrix, voxel F0, split metrics, origin traces and a
  negative corpus.
- **Documentation.** Re-stated at evidence tier; Wave-3 re-framed as search efficiency on a prior-laden screen;
  stale or unsupported numbers corrected or withdrawn; every numeral traced.

## 4. Open before sign-off

| # | Item | Owner | Blocking? |
|---|---|---|---|
| O-1 | CI red on PR head (matplotlib import in a test) | INT/TR | yes |
| O-2 | DOC-07 IR-R1: the OpenFOAM writer still emits unsplit inlet ducts while carrying `IrCompiledMesh`'s enactment and identity | IR (+ INT hold on INT-06) | yes, for any new CFD claim |
| O-3 | DOC-07 IR-R2..IR-R5: STL path tool, CUDA rename claim, corrugation labelled `ConservativeApproximation`, Betti parity vs constant | IR | no (documentation accuracy) |
| O-4 | INT-04 Wave-4 study: `wave3-proof` stops at the surrogate fit (no independent labels) | INT decision | yes for DOC-04's Wave-4 section |
| O-5 | INT-06 CFD sweep on Wave-4 winners | INT | depends on O-2 and O-4 |
| O-6 | TR-12 (homologous crossover) and EV-06 (physical admission) BLOCKED | Jepson: workplan §5 decisions 8–9 | needs a decision or a move to §6 |
| O-7 | Evidence-kernel review notes R-1..R-3 (point-value pass/fail, upper-bound-only criteria, F2 sub-levels) | EV | no |
| O-8 | Figure plates with baked-in stale text (G-07); explorer UI labels (G-08) | INT | no |

## 5. Residual risks after this PR (for Jepson)

1. **Published results predate the repairs.** Every Wave-1/2/3 search number came from code with silent
   fallback, single-box "HV" and gene-derived screens. They remain only as historical receipts, labelled in
   WHITEPAPER "Read this first" (workplan §5 decision 11).
2. **The search screen is still prior-laden.** The sandbox `F1` evaluator still scores with gene-derived
   descriptors. IR-07 keeps priors out of *qualified* tiers, but no physics-observed screen exists yet.
3. **CFD comparisons are not like-for-like.** The CT-35 control used a different pipeline and outlet extension
   (G-09), and the candidate shells do not contain the named mechanisms (O-2).
4. **No oil, transient or physical evidence.** `F2-Oil`, `F3` and `F4` have not run; physical validation is
   false; campaigns are sandbox-only.
5. **No independent surrogate labels.** OOD metrics are "insufficient evidence" by design.
6. **The genome is still the bounded macro language.** Open-ended invention (genome v4, QD over measured
   descriptors, mechanism cards) is a §6 follow-on.

## 6. Sign-off

Not signed off. Re-review after O-1, O-2 and O-4 resolve; then this draft becomes the PR comment for Jepson.
