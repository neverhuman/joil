# PR #3: JOIL v4 truth repair, evidence kernel, DesignIR foundations and corrected documents

Draft text for the PR #3 description (workplan INT-08). Jepson marks the PR ready and merges it. The claims audit
checks every number on this page against the result file named next to it.

## What this PR changes

- **Truth repair.** A failed variation no longer silently becomes `CT-35`. Every proposal is charged to the budget,
  with its repair recorded. GCI no longer clamps the order, Pareto hypervolume is a true union, and Monte Carlo tails
  report their sample size. Thresholds come from the requirement file.
- **Evidence kernel.** A new `joil-evidence` crate records content identity, typed observations and claims that can
  be revoked. `docs/STATUS.json` and the README status table are generated from it, and campaigns stay sandbox-only.
- **DesignIR.** Genes map to one design subject. The CFD case writer emits the geometry its enactment receipt
  describes, including split inlet ducts.
- **Re-run study (Wave-4).** The Wave-3 search study was repeated on the repaired code, and a `CT-35` control was
  built and solved by the same CFD pipeline as the candidate shells.
- **Documents.** README, WHITEPAPER and `docs/RESULTS.md` state each claim at its evidence tier. Earlier claims that
  did not survive the repairs are corrected or withdrawn, and the claims audit fails on any untraced number.

## Headline results

| Result | Numbers | Result file |
| :--- | :--- | :--- |
| Evolution does not beat uniform random search on best screen score (`10` seeds, `450` charged proposals each) | `B1` `45.936820` (`[45.615566, 46.265452]`) vs `B3` `45.131872` (`[44.778022, 45.452437]`); `B3` > `B1` has $p_{\text{Holm}} = 1.0$, Cliff's $\delta = -0.68$ | [`docs/results/wave4-evolution-proof-v1.json`](results/wave4-evolution-proof-v1.json) |
| Evolution leads only on population median | `B3` `43.628366` vs `B1` `37.983620`, $p_{\text{Holm}} = 0.0012568$ | [`docs/results/wave4-evolution-proof-v1.json`](results/wave4-evolution-proof-v1.json) |
| `CT-35` reference on the same screen | single frozen evaluation `23.931038` | [`docs/results/wave4-evolution-proof-v1.json`](results/wave4-evolution-proof-v1.json) |
| Independent reproduction with raw logs | `14,243` fields compared, `0` differ; separate equal-budget `CT-35` arm `23.775490` (`[23.547869, 24.024860]`) | [`crates/joil-evidence/receipts/Wave4-raw-capture-consumption.json`](../crates/joil-evidence/receipts/Wave4-raw-capture-consumption.json) |
| No CFD pressure-drop advantage over `CT-35` built by the same pipeline | control `G2` `18,633.89 Pa`; the archived shells lie within `-4.92` to `+90.05 Pa` of it; the control is `-36.77%` below the recorded `CT-35` case, so the earlier "% below `CT-35`" figures were a pipeline offset | [`docs/results/wave4-gci-v1.json`](results/wave4-gci-v1.json) |
| The one Wave-4 design solved with split ducts is not grid-converged | `G2` pressure drop `2.49×` the control's; conservative GCI `4,720.58%` | [`docs/results/wave4-gci-v1.json`](results/wave4-gci-v1.json) |
| Wave-4 CFD case identity | `3` of `12` cases keep an exact artifact chain; none is claim-bearing | [`crates/joil-evidence/receipts/EV12-Wave4-closure.json`](../crates/joil-evidence/receipts/EV12-Wave4-closure.json) |

All search numbers are values of a screen built from gene-derived descriptors (sandbox tier), not physical
performance. Nothing has been tested physically.

## Page-1 figure

[`docs/figures/wave4-page1-why-it-matters.png`](figures/wave4-page1-why-it-matters.png) (generator
`tools/generate_page1_figure.py`, `SHA-256: 7f87173ec83361279d92047cdc25da5102ad9f6c8bd88ff52937a6c41b9232e7`) leads
the WHITEPAPER. It plots best screen `KPI_score` against generation for evolution and uniform random search, with
the `CT-35` reference ([`docs/results/wave4-evolution-proof-v1.json`](results/wave4-evolution-proof-v1.json)).
It shows that random search is not beaten on best score. The figure has two gaps: `CT-35` appears only as a single
frozen reference, not as an equal-budget arm, and there is no held-out geometry test.

## Checks

- `tools/check.py` passes all `10` stages on each lane head and on the integration branch, and GitHub Actions is
  green on the PR head.
- The claims audit traces every numeral in README, WHITEPAPER, `docs/RESULTS.md` and this page.
- Independent audits: TR checked every number in README and WHITEPAPER, and DOC applied all its findings. EV traced
  the plotted figure values. IR ran every README command from a fresh clone
  ([`docs/receipts/README-freshclone-2026-10-16.md`](receipts/README-freshclone-2026-10-16.md)).
- Final review and residual risks: [`docs/reviews/2026-10-16-PR3-FINAL-REVIEW.md`](reviews/2026-10-16-PR3-FINAL-REVIEW.md).

## Open in this PR

- Workplan items INT-02 (lane merges), INT-08 (ready for review) and DOC-10 (keep the workplan current).
- EV-13 figure fixes: the hero, Plate 6 and Plate 11 need regenerating, and Plate 10's field panels need committed
  arrays. The captions flag each problem until the fixed figures land.

## Deferred to workplan §6

- Physical limits and statistical policy criteria, pending admitted requirement sources (TR-16).
- Independent oil-concentration labels and real out-of-distribution holdouts for the surrogate (EV-09).
- Historical and FEA case identity, and full scientific identity beyond the Wave-4 artifact chain (EV-12).
- Conformal throat meshing and the full CFD sweep of the Wave-4 winners; five of six were not solved.
- Homologous crossover (TR-12) and physical campaign admission (EV-06).
- Oil transport (`F2-Oil`), transient (`F3`) and physical (`F4`) validation.
- Open-ended invention: genome v4 and quality-diversity over measured descriptors.

🤖 Generated with [Claude Code](https://claude.com/claude-code)
