# EV-13 proof-figure trace for DOC-09

Audit target: integration `1c9c3b484543ea459f5283fbc2dd7da4a0303207`.
This is an independent figure/generator audit. TR's NUMTRACE prose audit is not repeated.

The [receipt](../receipts/EV13-proof-figure-trace.json) records nine PNGs, their
manifests, source hashes, exact fixes and the cleanup receipt. The
[value trace](../receipts/EV13-figure-value-trace.json) contains 783 successful
explicit checks, zero numeric differences in those checks, and 16 UNTRACED
rows. Every compared array includes expected/captured values, transformations
and individual source JSON pointers. Five altered-value/coverage/identity
negative tests reject. These counts include metadata and coverage checks;
they are not a claim that every historical pixel has a verified numeric source.

| Figure group | Binding to committed PNG | Result |
|---|---|---|
| Wave-4 README hero, page-1 lead, Plate 11 | Exact RGBA reproduction, all three | Plotted receipt values match; F01/F02/F03/F05/F08 remain |
| Wave-4 same-pipeline GCI, Plate 3, Plate 10 | Exact RGBA reproduction, all three | Receipt-bound series match; Plate 10's 14 field/profile/wall calls lack committed numeric arrays (F04) |
| Historical Wave-3 hero and Plate 11 | Current generator differs from both PNGs | Source projections match, but current captions import false Wave-4 narratives; historical error-bar scalar lacks a result JSON pointer (F06) |
| Wave-2 duty/ablation plate | Current replay differs in layout/style | Source projections match; embedded protocol/threshold/HV qualifications need correction (F07/F08) |

Selected primary trace entries (the machine trace enumerates every compared point):

| Number in figure | Committed source and JSON pointer | Observed |
|---|---|---|
| B3 final best mean and CI | `docs/results/wave4-evolution-proof-v1.json#/generation_band_curves/B3_Wave3_Exotic3D_QD_CEI/24/best_kpi_score` | 45.131872 [44.778022, 45.452437] |
| B1 final best mean and CI | Same file, `#/generation_band_curves/B1_Random_Search/24/best_kpi_score` | 45.936820 [45.615566, 46.265452] |
| Best-score B3 > B1 test | Same file, `#/pairwise_tests/0` | U=16, Holm p=1, Cliff delta=-0.68 |
| All seed KPI/Cout trajectories, all arm bands | Same file, `#/arm_outcomes` and `#/generation_band_curves` | Explicit per-seed/per-generation pointers in value trace |
| Top-seed milestone count and callouts | Same file, `#/arm_outcomes/B3_Wave3_Exotic3D_QD_CEI/7/lineage_milestones` | Seed 20261108, 14 incumbent improvements; not pedigree |
| Eleven authored-family bars and counts | Same file, `#/held_out_geometry_generalization` | Source values match; independent held-out geometry result absent, KPI RMSE null |
| Executed duty bars | `docs/results/wave2-multiseed-ablation-and-holdout-v1.json#/baseline_b0_held_out_duties` and `#/promoted_wave2_top_elite_held_out_duties` | Values match; feed median protocol deviations remain |
| GCI pressure versus cell spacing | `docs/results/wave4-gci-v1.json#/same_pipeline_ct35_control/grids`, `#/wave4_b3_top_winners/1/grids`, plus archived v2 dossier grids | All points match after Pa/1000 conversion; standard GCI remains null |

DOC acknowledged and independently confirmed F01–F08 on the live board at
11:56:34Z. Captions flag the gaps; INT owns generator/figure fixes. That is a
handoff, not evidence that the figures have already been corrected. F09, posted
subsequently, marks Plate 3's correctly converted mass-closure numerical gate
as code-bound policy without a result/manifest JSON pointer. It requires a
policy field in the manifest or an explicit DOC-09 coverage disposition; no
physical limit is invented or changed.

Residual risks to retain in DOC-09 / PR #3:

- **EV-09 deferred:** `joil-independent-oil-observations-v1` is absent. Independent Cout labels, reviewed observed mechanisms, and disjoint family/mechanism holdouts are still required. Real OOD/calibration metrics stay null.
- **EV-12 scoped closure:** twelve cases are accounted for: three verified active artifact chains and nine explicit non-claim exclusions. Eight executed meshes are missing; the failed G1 diagnostic has a stale sidecar. Exclusion is not identity PASS or physical infeasibility. The G2 comparison remains an incomplete historical diagnostic.
- **Historical/FEA remainders:** original executed meshes cannot be recovered from input-only copies. FEA, material/process evidence, load transfer and complete scientific/requirements identity remain deferred to section 6.
- **Raw reproduction scope:** the new raw-capture campaign reproduces 14,243 fields with zero differences. Original historical raw logs were never emitted. The new equal-budget CT-35 arm is a separate campaign; no existing figure supplies the requested three-arm equal-budget comparison.
- **Generalization/physics scope:** authored-family diagnostics are not independent geometry holdouts; milestones are not pedigree. Duty screens deviate from preregistration. Carrier CFD and normal-stress positivity do not qualify oil transport, full residence inventory, complete tensor realizability, physical validation or manufacturing.
- **GCI scope:** archived/control sequences are oscillatory; the solved eight-jet sequence is not asymptotic. Standard GCI is null, five other winners are unsolved, and the same-pipeline control removes the earlier apparent pressure advantage.
- **Figure closure:** preserve F01–F09 until INT supplies corrected figures/manifests or DOC explicitly dispositions missing coverage. In particular, missing committed field arrays and historical reproduction differences prevent an overall all-values-traced PASS.

Four authorized `/tmp/joil_ev12_wr_test_*` synthetic fixture directories had no
source/log or active-process references and were removed after private backup
and git-bundle verification. No worktree, build output or simulation was deleted.
