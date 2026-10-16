# PR #3: JOIL v4 truth repair, evidence kernel, DesignIR foundations and corrected documents

Description of PR #3 (workplan INT-08). The acting integrator marks the PR ready for review once the lane merge,
EV and DOC checks and CI are green; Jepson alone merges. The claims audit checks every number on this page
against the result file named next to it.

## What this PR changes

- **Truth repair.** A failed variation no longer silently becomes `CT-35`, and every proposal is charged to the
  budget. GCI no longer clamps the order, Pareto hypervolume is a true union, and thresholds come from the
  requirement file.
- **Evidence kernel.** A new `joil-evidence` crate records content identity, typed observations and revocable
  claims. The README status table is generated from it, and campaigns stay sandbox-only.
- **DesignIR.** Genes map to one design subject, and the CFD case writer emits the split inlet ducts it describes.
- **Wave-4 re-run.** The Wave-3 search study was repeated on the repaired code, and a `CT-35` control was built and
  solved by the same CFD pipeline as the candidate shells.
- **Documents.** README, WHITEPAPER and `docs/RESULTS.md` state each claim at its evidence tier, and the claims audit
  fails on any untraced number.

## Figures

- **Figure 0 (README hero):** [`docs/figures/wave4-readme-hero-kpi-and-elite.png`](https://github.com/neverhuman/joil/blob/4819bdf28eb84d5b93254c01f061d4b0c506c54b/docs/figures/wave4-readme-hero-kpi-and-elite.png)
  (`SHA-256: fab2f5f59ab5dc75cb90416dc24aaa2beafab4dee1a97888c6b783bdeb9602b0`). Screen score by generation for every
  search arm, with a gene-label schematic and archived CFD of separate subjects.
- **Page-1 figure (why it matters):** [`docs/figures/wave4-page1-why-it-matters.png`](https://github.com/neverhuman/joil/blob/4819bdf28eb84d5b93254c01f061d4b0c506c54b/docs/figures/wave4-page1-why-it-matters.png)
  (`SHA-256: 7f87173ec83361279d92047cdc25da5102ad9f6c8bd88ff52937a6c41b9232e7`). Best screen score by generation for
  evolution against uniform random search, with the `CT-35` reference: random search is not beaten on best score.
  `CT-35` appears only as a single frozen reference, and there is no held-out geometry test.

## Headline results

| Result | Numbers | Result file |
| :--- | :--- | :--- |
| Evolution does not beat uniform random search on best screen score (`10` seeds, `450` charged proposals each) | `B1` `45.936820` (`[45.615566, 46.265452]`) vs `B3` `45.131872` (`[44.778022, 45.452437]`); `B3` > `B1` has $p_{\text{Holm}} = 1.0$, Cliff's $\delta = -0.68$ | [`docs/results/wave4-evolution-proof-v1.json`](https://github.com/neverhuman/joil/blob/4819bdf28eb84d5b93254c01f061d4b0c506c54b/docs/results/wave4-evolution-proof-v1.json) |
| Evolution leads only on population median | `B3` `43.628366` vs `B1` `37.983620`, $p_{\text{Holm}} = 0.0012568$ | [`docs/results/wave4-evolution-proof-v1.json`](https://github.com/neverhuman/joil/blob/4819bdf28eb84d5b93254c01f061d4b0c506c54b/docs/results/wave4-evolution-proof-v1.json) |
| `CT-35` reference on the same screen | single frozen evaluation `23.931038` | [`docs/results/wave4-evolution-proof-v1.json`](https://github.com/neverhuman/joil/blob/4819bdf28eb84d5b93254c01f061d4b0c506c54b/docs/results/wave4-evolution-proof-v1.json) |
| Independent reproduction with raw logs | `14,243` fields compared, `0` differ; separate equal-budget `CT-35` arm `23.775490` (`[23.547869, 24.024860]`) | [`crates/joil-evidence/receipts/Wave4-raw-capture-consumption.json`](https://github.com/neverhuman/joil/blob/4819bdf28eb84d5b93254c01f061d4b0c506c54b/crates/joil-evidence/receipts/Wave4-raw-capture-consumption.json) |
| No CFD pressure-drop advantage over `CT-35` built by the same pipeline | control `G2` `18,633.89 Pa`; the archived shells lie within `-4.92` to `+90.05 Pa` of it; the control is `-36.77%` below the recorded `CT-35` case, so the earlier "% below `CT-35`" figures were a pipeline offset | [`docs/results/wave4-gci-v1.json`](https://github.com/neverhuman/joil/blob/4819bdf28eb84d5b93254c01f061d4b0c506c54b/docs/results/wave4-gci-v1.json) |
| The one Wave-4 design solved with split ducts is not grid-converged | `G2` pressure drop `2.49×` the control's; conservative GCI `4,720.58%` | [`docs/results/wave4-gci-v1.json`](https://github.com/neverhuman/joil/blob/4819bdf28eb84d5b93254c01f061d4b0c506c54b/docs/results/wave4-gci-v1.json) |
| Wave-4 CFD case identity | `3` of `12` cases keep an exact artifact chain; none is claim-bearing | [`crates/joil-evidence/receipts/EV12-Wave4-closure.json`](https://github.com/neverhuman/joil/blob/4819bdf28eb84d5b93254c01f061d4b0c506c54b/crates/joil-evidence/receipts/EV12-Wave4-closure.json) |

All search numbers are values of a screen built from gene-derived descriptors (sandbox tier), not physical
performance. Nothing has been tested physically.

## Checks

- `tools/check.py` passes all stages on the lane heads and the integration branch.
- The claims audit traces every numeral in README, WHITEPAPER, `docs/RESULTS.md` and this page. TR's independent
  number audit and its rerun after the figure regeneration left no open findings
  ([`crates/joil-core/tests/truth_numtrace_postregen_audit.json`](https://github.com/neverhuman/joil/blob/4819bdf28eb84d5b93254c01f061d4b0c506c54b/crates/joil-core/tests/truth_numtrace_postregen_audit.json)).
- EV re-checked the regenerated figures independently (a reproduction check, not scientific validation): all
  reproduce byte-for-byte from committed sources
  ([`crates/joil-evidence/reports/EV13-followup-reverify.md`](https://github.com/neverhuman/joil/blob/4819bdf28eb84d5b93254c01f061d4b0c506c54b/crates/joil-evidence/reports/EV13-followup-reverify.md)).
- IR, DOC and TR ran every README install and quick-start command from fresh checkouts; all passed
  ([`docs/receipts/README-freshclone-2026-10-16.md`](https://github.com/neverhuman/joil/blob/4819bdf28eb84d5b93254c01f061d4b0c506c54b/docs/receipts/README-freshclone-2026-10-16.md),
  [`docs/receipts/README-freshclone-503a299-2026-10-16.json`](https://github.com/neverhuman/joil/blob/4819bdf28eb84d5b93254c01f061d4b0c506c54b/docs/receipts/README-freshclone-503a299-2026-10-16.json)).
- Final review and residual risks: [`docs/reviews/2026-10-16-PR3-FINAL-REVIEW.md`](https://github.com/neverhuman/joil/blob/4819bdf28eb84d5b93254c01f061d4b0c506c54b/docs/reviews/2026-10-16-PR3-FINAL-REVIEW.md).

## Open residuals

- No figure compares evolution, random search and `CT-35` as three equal-budget arms; the equal-budget `CT-35` arm
  exists only in the separate raw-log reproduction.
- Plate 10's field maps use legacy cell positions (not finite-volume centroids), and its `CT-35` field mesh differs
  from the archived one. The caption says so.
- `main` already holds the integration branch through `3619bd7`, merged through PR #5. PR #4 is a draft and is not
  the integration PR.

## Deferred to workplan §6

- Physical limits and statistical policy criteria, pending admitted requirement sources (TR-16).
- Independent oil-concentration labels and real out-of-distribution holdouts (EV-09).
- Historical and FEA case identity beyond the Wave-4 artifact chain (EV-12).
- Conformal throat meshing and the full CFD sweep of the Wave-4 winners; five of six were not solved.
- Homologous crossover (TR-12) and physical campaign admission (EV-06).
- Oil transport (`F2-Oil`), transient (`F3`) and physical (`F4`) validation.
- Open-ended invention: genome v4 and quality-diversity over measured descriptors.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
