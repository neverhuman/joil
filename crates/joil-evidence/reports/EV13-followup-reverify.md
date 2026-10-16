# EV13 follow-up figure verification at 503a299

**Requested follow-up PASS, with the residuals below.** Input commit:
`503a299a93169bd416a625ea47023a77810b9362`. This is independent source,
numeric, text and raster verification; it does not admit scientific or physical
claims. No prose, generator, result dossier or authoritative PNG was edited by EV.

The selectively exported snapshot includes the committed
`crates/joil-evidence/receipts/Plate10-field-arrays.json` at its normal relative
path, SHA-256 `0a428abbc580eca756340bbc61920f6ef165a31fae1028d8dab1714123f339d4`.
It has **no `runs/` directory**. All nine figures reproduced byte-for-byte and
RGBA-for-RGBA from that snapshot. All seven PNG/manifest digest pairs in TR's
13:21Z GIVE match. All 13 scientific result files retain their previous bytes.

| Check | Independent result |
|---|---|
| Plot-call/source mapping | 266 observed calls mapped; unique AST function/receiver/positional signatures, plus three reviewed literal-30 → requirement changes |
| Plate 10 field/profile values | 32/32 arguments across 14 calls match exactly, including JSON null masks and subject identity |
| Previous versus new plotting data | All 266 calls retain exactly the same positional data and numeric error bars as the 11033dd capture |
| Full source/value trace | 833 PASS, 0 FAIL, 0 UNTRACED; explicit `file#json-pointer` and transformations per row |
| Adversarial checks | 15/15 reject altered seed/CI/unit/data/subject/null mask/digest/call/wording/caveat |
| Historical compatibility | Original pin: 783 PASS / 16 UNTRACED; 11033dd: 791 PASS / 14 UNTRACED; both 0 FAIL. Old receipts remain historical |

The new line map was derived independently before comparing TR's preparation
map; all observed mappings agree. Seventeen ambiguous nodes in other, unrendered
figures/GIFs were left unmapped, rather than guessed. They are outside this
nine-figure audit. The normal Plate 10 `main()` consumes JSON; no historical raw
extractor callback supplies its data.

Visual inspection of the committed PNGs, tied to the captured text through exact
replay, confirms Wave-3's **450 Total Evaluations/Seed** and **22,500 Evaluations**,
while Wave-4 keeps **Charged Proposals**. Plate 3 now says **Normal-Stress
Positivity (`R_ii > 0`): 100.00%**. This diagonal-stress check is not complete
tensor realizability. Plate 10 visibly retains the legacy-coordinate and baseline
identity caveats. These wording and committed-source follow-ups can be closed.

Evidence files under `crates/joil-evidence/receipts/`:

- `EV13-followup-reverify.json`: source inventory, digests, exact field bindings,
  labels, negative checks, commands and gate evidence.
- `EV13-followup-figure-value-trace.json`: every explicit source/value comparison.
- `EV13-followup-plot-line-map.json`: actual → original source lines.
- `ev13_followup_reverify.py`: repeatable independent mapping, exact comparison
  and rejection checks. Run after `ev13_figure_trace.py`; exact invocation is in
  the receipt. Original generator files are exported from the recorded git pins.

## Residual list for DOC's PR #3 final review

1. **F03 remains unmet:** no three-arm equal-450-budget figure. Historical CT-35
   is a frozen single-evaluation reference. The new equal-budget CT-35 result in
   `Wave4-raw-capture-number-trace.json#/recomputed/final_summary_statistics/B0_CT35_Equal_Budget`
   belongs to a separate reproduction campaign; do not silently mix campaigns.
2. **F04 source binding is closed; spatial and identity defects remain.**
   `Plate10-field-arrays.json#/cases/{wave1,wave2_tpms,baseline}/coordinate_review`
   records owned-face first-vertex averages, excluding neighbour contributions,
   not finite-volume centroids. Respectively 12,670 / 12,533 / 12,503 zero-owner
   cells remain at the origin. The retained baseline has 313,260 cells versus
   334,768 in the archived dossier. Matching arrays is not corrected sampling or
   certification of the archived execution mesh.
3. **EV-09 stays BLOCKED-SCOPED in section 6.** The independent-label dataset
   `joil-independent-oil-observations-v1` is absent. Source-bound independent
   outlet-oil observations and reviewed mechanisms, with preregistered disjoint
   leave-family-out and leave-mechanism-out holdouts, must precede calibration
   and real OOD metrics. No labels or physical limits were invented.
4. **EV-12 is scoped accountability:** `EV12-Wave4-closure.json` accounts for 12
   cases as three exact active artifact chains, eight missing/pruned-mesh
   exclusions and one G1 setup-failure diagnostic. The nine exclusions are not
   identity PASS. G1 failed on missing `inlet1` / `MPI_ABORT` before solver
   timesteps; a retained sidecar was stale. Setup failure is not physical
   infeasibility.
5. Historical executed meshes and scientific identity, FEA/material/process/load
   transfer, and independent oil/physical validation remain deferred. Post-hoc
   raw-file inventories cannot recover absent execution identities. The original
   historical raw traces were never emitted; the 14,243-value reproduction
   comparison is a new campaign, including 4,500 extra CT-35 proposals.
6. Authored-family/prior LOFO remains a diagnostic, not independent held-out
   geometry. Incumbent milestones are not pedigree. Executed duty/feed protocol
   deviations and gene-derived F1 screening remain explicit limitations.
7. Archived/control convergence is oscillatory; standard GCI stays null. The
   solved eight-jet case is non-asymptotic and five winners remain unsolved.
   Missing G2 mesh capture limits the historical comparison; the same-pipeline
   control establishes no pressure advantage. Carrier results do not establish
   oil residence, physical validation or admission.

Full required `python3 tools/check.py`: **10/10 PASS**, exit 0. Gate receipt
`.private/checks/20261016T134931Z-9f9185d5/receipt.json`, SHA-256
`cab4394d491afdd826194ef996772e3b207c8826a3c5811a85043c06dec63d86`. Publication scan: 371 candidates PASS;
`git diff --check` passes. All executable sources and numeric artifacts retain their tested hashes. Two
evidence metadata files (this report and the reverify receipt) were finalized
after that run; its source inventory records their pre-annotation bytes. A
second gate verifies the finalized commit; its receipt hash is handed off on the
live board without rewriting the audited inputs.

TR merges the pushed EV SHA; DOC owns final-review prose. No PR or main merge,
new PR, rebase or force-push was performed.
