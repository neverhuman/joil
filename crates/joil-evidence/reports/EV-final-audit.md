# EV final lead/hero and fresh-clone audit

Input: `857517a9e150d0b38ba0815f5f5f5c26deb1a62d`. **Requested figure/caption and install/first-three-command spot-check PASS**. This is source and software verification, not physical admission.

Both committed PNGs exist, match their manifests and have the same full digests in the live documents:

- `docs/figures/wave4-readme-hero-kpi-and-elite.png`: `fab2f5f59ab5dc75cb90416dc24aaa2beafab4dee1a97888c6b783bdeb9602b0`.
- `docs/figures/wave4-page1-why-it-matters.png`: `7f87173ec83361279d92047cdc25da5102ad9f6c8bd88ff52937a6c41b9232e7`.

All 48 EV13 figure inputs and all 69 TR numtrace source files match the already independently checked bytes. The explicit plot-call trace remains 833 PASS / 0 FAIL / 0 UNTRACED, with 32 exact field-array bindings across 14 calls. Each plotted array and quoted plot number has file/field references in `EV13-followup-figure-value-trace.json`; this report adds the prose-caption trace.

| Caption and quantity | Printed | Committed result file + JSON field |
|---|---:|---|
| README hero: seeds per arm | `10` | `docs/results/wave4-evolution-proof-v1.json#/n_seeds` |
| WHITEPAPER lead: seeds per arm | `10` | `docs/results/wave4-evolution-proof-v1.json#/n_seeds` |
| README hero: Monte Carlo samples per candidate | `72` | `docs/results/wave4-evolution-proof-v1.json#/promoted_wave3_top_elite/mc_robustness/samples` |
| README hero: CT35 screen Hinze diameter, micrometres | `64.0` | `docs/results/wave4-evolution-proof-v1.json#/baseline_b0_ct35/nominal_metrics/hinze_dmax_um` |
| README hero: initial generation index | `0` | `docs/results/wave4-evolution-proof-v1.json#/generation_band_curves/B3_Wave3_Exotic3D_QD_CEI/0/generation` |
| WHITEPAPER lead: charged proposals per seed and arm | `450` | `docs/results/wave4-evolution-proof-v1.json#/evaluations_per_seed_per_arm` |
| WHITEPAPER lead: B1 final best KPI, seed mean | `45.936820` | `docs/results/wave4-evolution-proof-v1.json#/final_summary_statistics/B1_Random_Search/best_kpi_score/mean` |
| WHITEPAPER lead: B3 final best KPI, seed mean | `45.131872` | `docs/results/wave4-evolution-proof-v1.json#/final_summary_statistics/B3_Wave3_Exotic3D_QD_CEI/best_kpi_score/mean` |
| WHITEPAPER lead: best KPI B3>B1, Holm p | `1.0` | `docs/results/wave4-evolution-proof-v1.json#/pairwise_tests/0/p_value_holm_bonferroni` |
| WHITEPAPER lead: best KPI B3>B1, Cliff delta | `-0.68` | `docs/results/wave4-evolution-proof-v1.json#/pairwise_tests/0/cliffs_delta` |
| WHITEPAPER lead: population median B3>B1, Holm p | `0.0012568` | `docs/results/wave4-evolution-proof-v1.json#/pairwise_tests/1/p_value_holm_bonferroni` |
| WHITEPAPER lead: B3 final population median KPI, seed mean | `43.628366` | `docs/results/wave4-evolution-proof-v1.json#/final_summary_statistics/B3_Wave3_Exotic3D_QD_CEI/median_kpi_score/mean` |
| WHITEPAPER lead: B1 final population median KPI, seed mean | `37.983620` | `docs/results/wave4-evolution-proof-v1.json#/final_summary_statistics/B1_Random_Search/median_kpi_score/mean` |
| WHITEPAPER lead: CT35 frozen reference KPI | `23.931038` | `docs/results/wave4-evolution-proof-v1.json#/baseline_b0_ct35_kpi_score` |

Repeated seed/budget/p-value mentions in the same captions use the same named fields. The 95% interval declaration refers to `docs/results/wave4-evolution-proof-v1.json#/generation_band_curves/{B3_Wave3_Exotic3D_QD_CEI,B1_Random_Search}/24/best_kpi_score/{ci95_low,ci95_high}`: confidence is encoded in the field names, rather than a separate scalar confidence leaf. The single frozen reference is `/arm_outcomes/B3_Wave3_Exotic3D_QD_CEI/0/reference_evaluations = 1`. No unrelated physical-tail confidence value was substituted.

Figure indices, Wave numbers, F1, CT-35, arm labels, panel labels, section/correction locations and SHA-256 identify documents, protocols or subjects. They are not measured quantities. The full opaque hashes are checked, not tokenized into scientific numbers.

The unchanged-source TR number recheck passed 736 direct facts, 89 derived/inherited facts, 1,769 classified occurrences and 409 table checks. The current claims audit passed 152 structured claims and 1,462 numeric tokens with zero errors. Three caption-check negatives reject a wrong named arm mean, wrong test metric and changed PNG bytes.

## Independent fresh clone

The OWNER-authorized `/tmp/joil-ev-fresh` was a separate HTTPS clone at the input SHA, with new project-local Python/Cargo caches and target. No TR clone or build cache was reused. Cargo jobs and Rust test threads were capped at four; incremental builds were disabled and debug information used line tables.

| Exact README command | Exit | Wall seconds |
|---|---:|---:|
| `python3 -m venv .private/quickstart-python` | 0 | 2.988 |
| `. .private/quickstart-python/bin/activate` | 0 | 0.003 |
| `python3 -m pip install numpy matplotlib` | 0 | 11.279 |
| `rtk cargo test --workspace --locked` | 0 | 168.150 |
| `rtk proxy cargo test -p joil-core --test regression closed_catalog_validity_11family_sampling -- --nocapture --exact` | 0 | 35.079 |
| `rtk proxy cargo test -p joil-core --test regression operator_chain_validity_20step_11family -- --nocapture --exact` | 0 | 36.579 |

At4Cargo jobs/4Rust test threads: workspace168.150144s is a few minutes; filtered checks35.078969s and36.579373s are under a minute. This limited spot-check does not cover the81s unittest observed independently byTR; general README timing revision remains DOC-owned.

The workspace command took about 2.8 minutes; the two filtered checks each took under a minute. This limited spot-check agrees with that part of the README timing text. It does not overrule TR’s separately observed 81-second Python unittest: DOC owns the requested general runtime wording update.

Before deletion: clean Git status, no stashes or local-only commits, and every local tag present remotely. All 168 non-build-cache target/test artifacts were archived outside the clone and their bytes reverified; command logs, versions and hashes were also retained. The clone was then deleted as requested. The private archive is test evidence, not physical validation or public raw data.

## Final documentation dependency and retained scope

The exact final DOC SHA after its 503a299 merge is a separate conditional verification. EV will run current numtrace and fresh figure verification against that SHA and post EVPASS or exact failures for INT-08. A verification at 857517a does not certify an unseen final branch.

EV-09 remains BLOCKED-SCOPED for absent independent labels/disjoint OOD holdouts. EV-12 closes accountability as three exact chains plus nine nonclaim exclusions. Keep F03’s missing equal-budget three-arm figure and F04’s disclosed legacy spatial/baseline-identity defects. Missing historical executed identities/meshes, FEA/material/process/load transfer and independent oil/physical validation remain deferred. No labels or physical limits were invented; no new PR, main operation or PR merge was performed.
