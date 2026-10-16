# Independent Wave-4 number trace — 16 October 2026

Reviewed integration snapshot: `cb31ba3` on `feat/joil-v4-truth-evidence`. This is an arithmetic and publication audit of committed sandbox evidence. No search, CFD, geometry evaluation or physical admission was run. EV-06 is **DONE-SANDBOX** under INT’s 07:02Z scope decision; physical admission remains deferred to workplan section 6.

**Result:** zero arithmetic mismatches within the precision of the serialized inputs. B3 has lower final best KPI than random search. Publication/evidence gaps W4-G1–G7 are handed to INT/DOC below; zero arithmetic mismatches does not close those gaps.

## Pinned inputs and method

Pointers below use `W`, `S`, `C`, `E` as these exact files; `*` selects all ten seed outcomes, and `a` means the full arm key in the mapping below. `g` is the zero-based generation index. Mean/CI values are dimensionless KPI unless a unit is given.

| Alias | File | SHA-256 |
|---|---|---|
| W | `docs/results/wave4-evolution-proof-v1.json` | `2e7192e2aff6ceceadc1f1efc03682ac602d784138cde05fc9dc265efdbd0885` |
| S | `docs/results/wave4-smoke-v1.json` | `385ae75ff20a8b07d81e574027ebc2504a97887c7c4a8d681d5c74c3c3d9da32` |
| C | `docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v2.json` | `4fa41f6b755453af22befd5f2294156afa655edbda8c69f1dbaccd7452f42eb9` |
| E | `docs/results/elite-3grid-gci-and-rsm-verification-v2.json` | `c5c04e32e375414ba93e91f761552c79774613e267af4a01d19b4a07ab901615` |
| README | `README.md` | `7151e3fa0cc55170589a31306ac513e83299f8f2902394e7c42f02d96b0a6d73` |
| WP | `docs/WHITEPAPER.md` | `9e14ec2d5dc9386c59c85a53fdf08074f37ac22b3976b68d35fbb0dcda2f77b1` |
| HERO | `docs/figures/wave4-readme-hero-kpi-and-elite.png` | `57ab31ca4b742e176149cb76b3082f336a7526cf4f5989671430fa1371782fc2` |
| PLATE | `docs/figures/wave4-evolution-ablation-and-lineage-proof.png` | `8ada9b7faeed9ce88d228816853d8f212383942bc0d407daa8309852dce9364e` |
| GEN | `tools/generate_wave3_proof_figures.py` | `a233cb84efab51db12959a21a96f9d5c8efdc0121a9bfa8ea7d6f4c06dd7272e` |

| Short name | Full JSON arm key |
|---|---|
| A1 | `A1_No_Mutation_Crossover_Only` |
| A2 | `A2_No_Selection_Random_Walk` |
| B1 | `B1_Random_Search` |
| B2 | `B2_Conventional_Axisym_NSGA2` |
| B3 | `B3_Wave3_Exotic3D_QD_CEI` |

The independent helper [`wave4_number_trace.py`](../../crates/joil-evidence/receipts/wave4_number_trace.py) imports no Joil module. It aggregates the seed traces, replays the declared 10,000-resample percentile bootstrap using independent NumPy code and SplitMix64 draws, and recomputes pairwise win counts, Cliff’s delta, normal-approximation one-sided p-values and Holm correction over all 16 tests. It also enumerates the untied 10-vs-10 rank null distribution as a sensitivity check. The published p-values are **normal approximations**, not exact permutation p-values; no continuity/tie correction is used by the receipt. The direction is B3 superiority, not a two-sided test.

All summary fields (mean, sample SD, median, IQR, both CI endpoints and n) were checked for all 1,000 generation/metric distributions (5 arms × 25 checkpoints × 8 metrics), plus final summaries. Serialization rounds scores/summaries to six decimals; comparison tolerance is 0.00000101 for serialized arithmetic, 0.000051 for four-decimal z scores, and 1e-9 for U, delta and eight-decimal p/Holm values. This allows one final decimal from already rounded inputs, rather than asserting more precision than the source. Tables show six decimals; the helper retains full recomputed floats. A last-digit difference in a printed recomputation is within that declared input-quantization tolerance, not a claim of bitwise equality.

```bash
OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 .private/ci-python/bin/python \
  crates/joil-evidence/receipts/wave4_number_trace.py \
  > .private/wave4-number-trace-recomputed.json
```

## Headline and budget trace

| Published/required quantity | File:JSON key or derivation | Independently recomputed | Match? |
|---|---|---|---|
| Seeds / master-seed interval | `W:/master_seeds`, `/n_seeds` | 10 unique seeds, 20261101…20261110 | yes |
| Generation checkpoints | `W:/arm_outcomes/a/*/generation_trace` | 25 each, g=0…24; `/generations=24` excludes gen-0 | yes |
| Population / eval increments | `W:/population_per_generation`; trace `cumulative_evaluations` differences | 18, increments 18; final 450 | yes |
| Search arms / displayed comparators | `W:/arm_outcomes` plus `/baseline_b0_ct35` | 5 search arms plus 1 frozen B0 reference = 6 displayed entries | yes, only 5 replicated arms |
| Search proposal budget | `W:/arm_outcomes/a/*/budget` | 50 × 450 = 22,500 charged search proposals | yes |
| Equal 450 budget for JOIL and random | `W:/arm_outcomes/{B3_Wave3_Exotic3D_QD_CEI,B1_Random_Search}/*/{budget,total_evaluations}` | 450 per seed in both arms, 10 seeds each | yes |
| Equal 450 budget and CI for CT-35 | `W:/baseline_b0_ct35`; no B0 key in `/arm_outcomes` | one frozen screen evaluation, KPI 23.931038; no independent seed CI | **no — W4-G3** |
| Additional reference cost | `W:/arm_outcomes/a/*/reference_evaluations` | 50 reference evaluations, outside 22,500 charged proposals; archetype/duty work also excluded | yes; restricted accounting scope |
| MC n in both PNG subtitles | `W:/arm_outcomes/a/*/best_candidate/mc_robustness/samples` | 72 in all 50 saved winners; PNG generator defaults to 28 when root field is absent | **no — W4-G4** |
| Bootstrap confidence / resamples | `W:/generation_band_curves/a/g/*/{ci95_low,ci95_high}`; declared producer policy | 95% percentile interval, 10,000 resamples replayed | yes; bootstrap settings not explicit top-level JSON fields |
| KPI constants | `W:/kpi_formula`; `requirements/joil-hydrocyclone-v1.toml:/thresholds/quality_limit_mg_l` | (30−C) + 25×grade + 10×(dp_ref−dp)/dp_ref − 500×violation | yes, 50 winners + baseline + promoted elite + 11 family candidates |
| Physical discharge ceiling / physical reliability | sandbox-only campaign admission and requirements | 30 mg/L is a research threshold; no admitted physical discharge limit or physical pass probability | **no if presented as physical** — W4-G5 |

| Arm | Charged budget | Invalid | Repaired | Unrepaired evaluations | Reference evaluations (outside budget) | Match partition? |
|---|---:|---:|---:|---:|---:|---|
| A1 | 4500 | 0 | 0 | 4500 | 10 | yes |
| A2 | 4500 | 0 | 2065 | 2435 | 10 | yes |
| B1 | 4500 | 0 | 58 | 4442 | 10 | yes |
| B2 | 4500 | 0 | 2329 | 2171 | 10 | yes |
| B3 | 4500 | 0 | 2263 | 2237 | 10 | yes |

## Final KPI mean, SD and confidence interval

Each source is `W:/arm_outcomes/a/*/generation_trace/24/{best_kpi_score,median_kpi_score}`, independently aggregated and compared with `W:/final_summary_statistics/a`. CT-35’s point value is recomputed from its saved candidate metrics; duplicating that point ten times would not create independent replication.

| Arm | Best KPI mean ± sample SD | Best KPI 95% CI | Active population-median KPI mean [95% CI] | Final best C upper mean [95% CI], mg/L | Match? |
|---|---|---|---|---|
| B0 | 23.931038; SD unavailable | unavailable | unavailable | 27.627434; CI unavailable | yes point; **no replicated CI (G3)** |
| A1 | 33.956765 ± 0.405189 | [33.713883, 34.184385] | 33.640806 [33.353155, 33.923923] | 18.884314 [18.681624, 19.092148] | yes |
| A2 | 42.715589 ± 1.090280 | [42.064348, 43.349521] | 37.856710 [36.225913, 39.406820] | 11.362546 [10.888869, 11.834905] | yes |
| B1 | 45.936820 ± 0.545526 | [45.615566, 46.265452] | 37.983620 [37.173104, 38.828128] | 8.786195 [8.490736, 9.068007] | yes |
| B2 | 36.236410 ± 0.318799 | [36.047525, 36.416028] | 35.464588 [35.221807, 35.695880] | 17.137609 [16.968592, 17.318753] | yes |
| B3 | 45.131872 ± 0.581719 | [44.778022, 45.452437] | 43.628366 [43.230036, 44.038378] | 9.518449 [9.215962, 9.835706] | yes |

B3−random best-score mean difference = **−0.8049475** (published −0.804948), U=16, δ=−0.68, one-sided p=0.99491737, Holm p=1.0. Thus the stated superiority test does not establish a best-score win. B3’s population-median endpoint improves by +5.644746, U=100, δ=+1, Holm p=0.0012568; it is a different endpoint. B3−frozen CT-35 point difference is +21.200834; there is no B3-vs-B0 replicated rank test in this JSON.

## Every per-generation KPI and plotted band

For each row, the exact source pattern is `W:/arm_outcomes/a/*/generation_trace/g/<metric>` and the displayed counterpart is `W:/generation_band_curves/a/g/<metric>`. Every row uses all ten seeds. The last three columns additionally trace hero Panel B and the ablation diversity panel; every CI endpoint is independently resampled. No axis ticks, panel letters or drawing coordinates are empirical observations.

| Arm | g / charged evals per seed | Best KPI mean [95% CI] | Population-median KPI mean [95% CI] | Best C upper mean [95% CI], mg/L | Diversity mean [95% CI], bits | Match? |
|---|---|---|---|---|---|---|
| A1 | 0 / 18 | 31.008835 [30.683310, 31.405387] | 26.691075 [26.283031, 27.106325] | 21.533777 [21.196295, 21.817095] | 0.767606 [0.619428, 0.909242] | yes |
| A1 | 1 / 36 | 31.541940 [31.180944, 31.928870] | 28.228004 [27.801146, 28.643781] | 21.044959 [20.725817, 21.346239] | 0.316763 [0.115328, 0.541531] | yes |
| A1 | 2 / 54 | 32.150065 [31.631915, 32.641514] | 28.913104 [28.149396, 29.589592] | 20.533119 [20.097601, 20.993243] | 0.264207 [0.065002, 0.493332] | yes |
| A1 | 3 / 72 | 32.637029 [32.250941, 33.039425] | 29.607766 [28.873860, 30.342277] | 20.046949 [19.705494, 20.371296] | 0.238112 [0.050326, 0.448618] | yes |
| A1 | 4 / 90 | 32.795103 [32.421060, 33.162502] | 30.060275 [29.207760, 30.883526] | 19.935371 [19.613712, 20.259699] | 0.180330 [0.000000, 0.360660] | yes |
| A1 | 5 / 108 | 33.057096 [32.748536, 33.369997] | 30.489972 [29.724528, 31.248184] | 19.701136 [19.427638, 19.975577] | 0.061909 [0.000000, 0.154772] | yes |
| A1 | 6 / 126 | 33.314972 [33.026321, 33.601849] | 31.016303 [30.242768, 31.762254] | 19.486797 [19.247150, 19.725910] | 0.081280 [0.000000, 0.201303] | yes |
| A1 | 7 / 144 | 33.429531 [33.133718, 33.717331] | 31.952354 [31.225980, 32.570675] | 19.369833 [19.121465, 19.617191] | 0.000000 [0.000000, 0.000000] | yes |
| A1 | 8 / 162 | 33.605947 [33.292930, 33.894344] | 31.962241 [30.960572, 32.791097] | 19.210822 [18.950946, 19.485399] | 0.000000 [0.000000, 0.000000] | yes |
| A1 | 9 / 180 | 33.607198 [33.303032, 33.897048] | 32.110524 [31.236934, 32.850665] | 19.208559 [18.949154, 19.474751] | 0.000000 [0.000000, 0.000000] | yes |
| A1 | 10 / 198 | 33.619552 [33.299379, 33.909518] | 32.525440 [31.901588, 33.044939] | 19.192981 [18.931412, 19.464571] | 0.000000 [0.000000, 0.000000] | yes |
| A1 | 11 / 216 | 33.651531 [33.360687, 33.924988] | 32.540531 [31.914591, 33.056480] | 19.163985 [18.926625, 19.406009] | 0.000000 [0.000000, 0.000000] | yes |
| A1 | 12 / 234 | 33.696086 [33.424665, 33.946116] | 32.667990 [32.006817, 33.202648] | 19.132552 [18.903662, 19.359869] | 0.000000 [0.000000, 0.000000] | yes |
| A1 | 13 / 252 | 33.703978 [33.435169, 33.954684] | 33.014505 [32.651184, 33.391225] | 19.125704 [18.909182, 19.349237] | 0.000000 [0.000000, 0.000000] | yes |
| A1 | 14 / 270 | 33.709308 [33.443629, 33.966506] | 33.128799 [32.754112, 33.511463] | 19.119264 [18.893599, 19.352921] | 0.000000 [0.000000, 0.000000] | yes |
| A1 | 15 / 288 | 33.766576 [33.509365, 34.007839] | 33.271904 [32.928634, 33.635708] | 19.068782 [18.853937, 19.289768] | 0.000000 [0.000000, 0.000000] | yes |
| A1 | 16 / 306 | 33.769898 [33.515725, 34.009714] | 33.224524 [32.756975, 33.656317] | 19.064661 [18.850202, 19.291983] | 0.000000 [0.000000, 0.000000] | yes |
| A1 | 17 / 324 | 33.791314 [33.551280, 34.018693] | 33.394070 [33.094278, 33.710337] | 19.042522 [18.838378, 19.254929] | 0.000000 [0.000000, 0.000000] | yes |
| A1 | 18 / 342 | 33.813235 [33.564577, 34.042304] | 33.523631 [33.245111, 33.798435] | 19.021120 [18.817945, 19.232080] | 0.000000 [0.000000, 0.000000] | yes |
| A1 | 19 / 360 | 33.818502 [33.566249, 34.059452] | 33.441697 [33.096609, 33.781207] | 19.018074 [18.804841, 19.233104] | 0.000000 [0.000000, 0.000000] | yes |
| A1 | 20 / 378 | 33.887469 [33.647267, 34.099998] | 33.467785 [33.129338, 33.802668] | 18.950629 [18.769295, 19.144155] | 0.000000 [0.000000, 0.000000] | yes |
| A1 | 21 / 396 | 33.920514 [33.661010, 34.157138] | 33.546417 [33.248293, 33.847167] | 18.915476 [18.709927, 19.130932] | 0.000000 [0.000000, 0.000000] | yes |
| A1 | 22 / 414 | 33.922365 [33.671491, 34.163242] | 33.614317 [33.314644, 33.893776] | 18.914728 [18.709517, 19.128978] | 0.000000 [0.000000, 0.000000] | yes |
| A1 | 23 / 432 | 33.941142 [33.708384, 34.169785] | 33.640579 [33.343291, 33.929660] | 18.898027 [18.705020, 19.098018] | 0.000000 [0.000000, 0.000000] | yes |
| A1 | 24 / 450 | 33.956765 [33.713883, 34.184385] | 33.640806 [33.353155, 33.923923] | 18.884314 [18.681624, 19.092148] | 0.000000 [0.000000, 0.000000] | yes |
| A2 | 0 / 18 | 31.008835 [30.683310, 31.405387] | 26.691075 [26.283031, 27.106325] | 21.533777 [21.196295, 21.817095] | 0.767606 [0.619428, 0.909242] | yes |
| A2 | 1 / 36 | 34.622239 [33.795079, 35.349669] | 28.110107 [27.644729, 28.585735] | 18.342176 [17.673465, 19.085144] | 1.075788 [0.775294, 1.381301] | yes |
| A2 | 2 / 54 | 36.871546 [35.925717, 37.782148] | 29.103256 [28.336083, 30.007032] | 16.364613 [15.537173, 17.199275] | 1.439181 [1.307833, 1.576659] | yes |
| A2 | 3 / 72 | 37.656128 [36.783922, 38.521252] | 30.490388 [29.304334, 31.818867] | 15.662165 [14.893891, 16.465253] | 1.498758 [1.291904, 1.697678] | yes |
| A2 | 4 / 90 | 38.326741 [37.386085, 39.239722] | 31.637099 [30.585701, 32.791720] | 15.187652 [14.405314, 16.040841] | 1.612210 [1.435056, 1.790500] | yes |
| A2 | 5 / 108 | 39.654072 [38.420696, 40.944064] | 32.661996 [31.270318, 33.970297] | 13.883310 [12.722729, 14.979276] | 1.634604 [1.398414, 1.852379] | yes |
| A2 | 6 / 126 | 38.819646 [37.484495, 40.126230] | 32.878196 [31.374075, 34.244966] | 14.703880 [13.546661, 15.892287] | 1.476589 [1.168476, 1.775184] | yes |
| A2 | 7 / 144 | 39.265921 [37.870161, 40.431825] | 33.694167 [32.444576, 35.004429] | 14.244061 [13.295678, 15.374979] | 1.554796 [1.265512, 1.798476] | yes |
| A2 | 8 / 162 | 39.870727 [38.882596, 40.739657] | 34.106095 [32.966993, 35.271756] | 13.722007 [12.975064, 14.585260] | 1.475451 [1.227135, 1.679034] | yes |
| A2 | 9 / 180 | 40.598393 [39.521076, 41.491001] | 34.984572 [33.930686, 36.029960] | 13.077654 [12.266990, 14.074562] | 1.186690 [0.842072, 1.480195] | yes |
| A2 | 10 / 198 | 40.827381 [39.743016, 41.800558] | 36.165267 [35.208486, 37.184398] | 13.010974 [12.076655, 14.077867] | 1.229788 [0.787489, 1.643137] | yes |
| A2 | 11 / 216 | 41.658155 [40.803851, 42.585214] | 36.733431 [35.805521, 37.861856] | 12.189963 [11.393462, 12.955275] | 1.150582 [0.754244, 1.542257] | yes |
| A2 | 12 / 234 | 41.686037 [40.820915, 42.555907] | 36.552718 [35.366604, 37.709961] | 12.235114 [11.458758, 12.940560] | 1.221889 [0.861436, 1.570912] | yes |
| A2 | 13 / 252 | 41.926207 [41.140477, 42.705382] | 36.136643 [34.709177, 37.565210] | 11.959270 [11.320793, 12.627874] | 1.206937 [0.892523, 1.535750] | yes |
| A2 | 14 / 270 | 41.847026 [41.077828, 42.604358] | 36.732955 [35.482597, 38.099772] | 12.043081 [11.429888, 12.697405] | 1.420069 [1.120151, 1.686992] | yes |
| A2 | 15 / 288 | 41.992764 [41.512326, 42.486158] | 37.536515 [36.138468, 38.891635] | 11.888384 [11.506229, 12.274047] | 1.481713 [1.130813, 1.781492] | yes |
| A2 | 16 / 306 | 42.258059 [41.282750, 43.106143] | 37.564071 [35.758304, 39.069028] | 11.701412 [10.949523, 12.584012] | 1.425144 [1.170047, 1.659705] | yes |
| A2 | 17 / 324 | 41.797649 [40.584565, 42.870218] | 37.408293 [36.025728, 38.744024] | 12.117814 [11.252271, 13.135310] | 1.361136 [0.931061, 1.718895] | yes |
| A2 | 18 / 342 | 42.278033 [41.470600, 42.950585] | 38.027248 [36.513278, 39.437030] | 11.710951 [11.105361, 12.433579] | 1.273595 [0.903834, 1.570934] | yes |
| A2 | 19 / 360 | 42.249409 [41.052640, 43.266151] | 37.589929 [36.233686, 38.971392] | 11.721163 [10.874255, 12.772169] | 1.307840 [0.953501, 1.603159] | yes |
| A2 | 20 / 378 | 42.398603 [41.458848, 43.200173] | 38.044607 [36.718260, 39.241656] | 11.652609 [10.981732, 12.450195] | 1.290213 [0.936135, 1.597538] | yes |
| A2 | 21 / 396 | 42.380599 [41.535606, 43.182996] | 37.817481 [36.944347, 38.570987] | 11.632028 [11.006061, 12.340646] | 1.198122 [0.926709, 1.460029] | yes |
| A2 | 22 / 414 | 42.329458 [41.615193, 43.016686] | 38.096518 [37.299665, 38.946488] | 11.694909 [11.149826, 12.303512] | 1.182305 [0.837626, 1.539816] | yes |
| A2 | 23 / 432 | 42.726490 [42.109306, 43.338698] | 38.236155 [37.265849, 39.232259] | 11.310488 [10.817677, 11.800630] | 1.171275 [0.773198, 1.571137] | yes |
| A2 | 24 / 450 | 42.715589 [42.064348, 43.349521] | 37.856710 [36.225913, 39.406820] | 11.362546 [10.888869, 11.834905] | 1.336376 [0.947320, 1.717530] | yes |
| B1 | 0 / 18 | 44.490288 [43.542422, 45.375083] | 37.291656 [36.407128, 38.108427] | 9.970681 [9.248499, 10.745226] | 2.494312 [2.399395, 2.597546] | yes |
| B1 | 1 / 36 | 44.875408 [44.304672, 45.497112] | 37.923862 [37.238752, 38.641234] | 9.676618 [9.148501, 10.192108] | 2.324046 [2.204403, 2.449046] | yes |
| B1 | 2 / 54 | 45.022501 [44.413199, 45.627286] | 38.371758 [37.423570, 39.381678] | 9.576819 [9.065813, 10.113328] | 2.290208 [2.109848, 2.444833] | yes |
| B1 | 3 / 72 | 45.184221 [44.634737, 45.738317] | 38.272721 [37.703600, 38.860066] | 9.483529 [8.994500, 9.968221] | 2.333508 [2.210409, 2.437934] | yes |
| B1 | 4 / 90 | 45.300582 [44.830417, 45.770577] | 37.587763 [36.705422, 38.433098] | 9.339548 [8.928568, 9.730551] | 2.233244 [2.010168, 2.449174] | yes |
| B1 | 5 / 108 | 45.382633 [44.889313, 45.857612] | 38.110087 [37.392836, 38.716705] | 9.279510 [8.885780, 9.688905] | 2.287934 [2.208760, 2.356003] | yes |
| B1 | 6 / 126 | 45.409005 [44.969388, 45.842797] | 38.150462 [37.366592, 38.979906] | 9.244162 [8.871020, 9.599592] | 2.355314 [2.218697, 2.485938] | yes |
| B1 | 7 / 144 | 45.489897 [45.168600, 45.880521] | 37.309300 [36.845782, 37.722445] | 9.153664 [8.838088, 9.390553] | 2.248293 [2.083939, 2.416159] | yes |
| B1 | 8 / 162 | 45.679279 [45.333109, 46.066798] | 38.581261 [38.031748, 39.139446] | 9.045133 [8.724073, 9.307938] | 2.344975 [2.232537, 2.455388] | yes |
| B1 | 9 / 180 | 45.679279 [45.339297, 46.055573] | 38.838794 [38.086816, 39.467621] | 9.045133 [8.725867, 9.314380] | 2.410622 [2.314936, 2.497256] | yes |
| B1 | 10 / 198 | 45.679279 [45.339727, 46.061615] | 38.231910 [37.259387, 39.278110] | 9.045133 [8.714325, 9.307544] | 2.468773 [2.392144, 2.548954] | yes |
| B1 | 11 / 216 | 45.679279 [45.336772, 46.064774] | 37.777524 [36.995892, 38.546602] | 9.045133 [8.717668, 9.313401] | 2.364475 [2.232724, 2.484978] | yes |
| B1 | 12 / 234 | 45.679279 [45.339684, 46.064060] | 37.546130 [37.168827, 37.920635] | 9.045133 [8.721021, 9.311540] | 2.329776 [2.235778, 2.420156] | yes |
| B1 | 13 / 252 | 45.679279 [45.337001, 46.063263] | 38.060911 [37.342480, 38.777611] | 9.045133 [8.732270, 9.309504] | 2.148219 [1.966816, 2.329726] | yes |
| B1 | 14 / 270 | 45.721109 [45.383239, 46.094214] | 37.279553 [36.438756, 38.193355] | 9.032365 [8.715226, 9.286487] | 2.147777 [1.970852, 2.311159] | yes |
| B1 | 15 / 288 | 45.746566 [45.413659, 46.110066] | 37.440306 [36.502654, 38.390957] | 9.013853 [8.714378, 9.260255] | 2.256642 [2.138691, 2.360514] | yes |
| B1 | 16 / 306 | 45.771054 [45.465391, 46.116171] | 37.330630 [36.600648, 38.107551] | 9.056628 [8.725035, 9.354727] | 2.287628 [2.208325, 2.383485] | yes |
| B1 | 17 / 324 | 45.771054 [45.461795, 46.127512] | 37.730872 [37.206210, 38.308415] | 9.056628 [8.720062, 9.356185] | 2.269471 [2.170530, 2.369343] | yes |
| B1 | 18 / 342 | 45.771054 [45.466142, 46.118874] | 38.106916 [37.318019, 38.718280] | 9.056628 [8.719489, 9.352381] | 2.348293 [2.220850, 2.455300] | yes |
| B1 | 19 / 360 | 45.771054 [45.470937, 46.121582] | 38.869845 [38.260733, 39.453529] | 9.056628 [8.729624, 9.350124] | 2.357806 [2.243106, 2.474816] | yes |
| B1 | 20 / 378 | 45.820188 [45.518792, 46.156142] | 37.974151 [37.174431, 38.803116] | 8.962835 [8.626976, 9.282056] | 2.259781 [2.152608, 2.368088] | yes |
| B1 | 21 / 396 | 45.874559 [45.593320, 46.184271] | 37.989857 [36.954070, 38.825617] | 8.847064 [8.565241, 9.098290] | 2.275407 [2.174005, 2.369514] | yes |
| B1 | 22 / 414 | 45.874559 [45.596102, 46.184555] | 38.277301 [37.546777, 38.915036] | 8.847064 [8.569960, 9.103719] | 2.327502 [2.217716, 2.425680] | yes |
| B1 | 23 / 432 | 45.874559 [45.593980, 46.186844] | 38.295139 [37.751720, 38.898201] | 8.847064 [8.569022, 9.096654] | 2.383652 [2.294995, 2.460573] | yes |
| B1 | 24 / 450 | 45.936820 [45.615566, 46.265452] | 37.983620 [37.173104, 38.828128] | 8.786195 [8.490736, 9.068007] | 2.312985 [2.182787, 2.435458] | yes |
| B2 | 0 / 18 | 27.471889 [27.145364, 27.792314] | 25.270739 [25.074480, 25.456058] | 24.593498 [24.320703, 24.878602] | 1.160718 [1.055909, 1.266614] | yes |
| B2 | 1 / 36 | 28.771383 [28.391858, 29.133931] | 26.905420 [26.588568, 27.228940] | 23.533460 [23.209760, 23.863113] | 0.760106 [0.664182, 0.843126] | yes |
| B2 | 2 / 54 | 30.263286 [29.588488, 30.885556] | 27.727192 [27.481558, 27.973850] | 22.279494 [21.732212, 22.898911] | 0.760839 [0.625315, 0.879184] | yes |
| B2 | 3 / 72 | 30.940592 [30.232717, 31.643160] | 28.729166 [28.407948, 29.012569] | 21.684481 [21.113083, 22.271757] | 0.730064 [0.652326, 0.810516] | yes |
| B2 | 4 / 90 | 31.383449 [30.787717, 32.006532] | 29.718629 [29.177685, 30.215513] | 21.385039 [20.847328, 21.896876] | 0.717894 [0.574055, 0.852119] | yes |
| B2 | 5 / 108 | 32.052447 [31.511248, 32.592595] | 30.511635 [30.014269, 31.019456] | 20.825766 [20.343782, 21.297125] | 0.645105 [0.538764, 0.747484] | yes |
| B2 | 6 / 126 | 32.586848 [32.201155, 32.986976] | 31.324253 [30.901831, 31.729389] | 20.401954 [20.007602, 20.763138] | 0.580105 [0.493868, 0.663084] | yes |
| B2 | 7 / 144 | 33.205348 [32.747849, 33.710226] | 31.733831 [31.449672, 31.995355] | 19.833548 [19.345891, 20.276897] | 0.531543 [0.401705, 0.672665] | yes |
| B2 | 8 / 162 | 33.730177 [33.453283, 34.077019] | 31.995875 [31.754116, 32.246207] | 19.393068 [19.089620, 19.654075] | 0.516867 [0.397010, 0.658856] | yes |
| B2 | 9 / 180 | 33.939540 [33.620948, 34.284095] | 32.267329 [31.946861, 32.609533] | 19.188332 [18.868800, 19.496202] | 0.585416 [0.485915, 0.697568] | yes |
| B2 | 10 / 198 | 34.155692 [33.777455, 34.588576] | 32.620155 [32.295523, 32.992832] | 19.051184 [18.633161, 19.400675] | 0.551368 [0.445143, 0.676964] | yes |
| B2 | 11 / 216 | 34.499674 [34.090282, 34.966709] | 32.839976 [32.491490, 33.215202] | 18.722324 [18.305298, 19.092164] | 0.378540 [0.243840, 0.498563] | yes |
| B2 | 12 / 234 | 34.760005 [34.314638, 35.252595] | 33.277494 [32.880591, 33.669747] | 18.505708 [18.054838, 18.928008] | 0.250564 [0.107375, 0.408264] | yes |
| B2 | 13 / 252 | 35.193127 [34.770264, 35.624652] | 33.603482 [33.295241, 33.917171] | 18.112723 [17.698759, 18.519889] | 0.169283 [0.030954, 0.336636] | yes |
| B2 | 14 / 270 | 35.234581 [34.845978, 35.644217] | 33.815710 [33.521684, 34.114174] | 18.068263 [17.688595, 18.432739] | 0.107375 [0.000000, 0.260216] | yes |
| B2 | 15 / 288 | 35.447283 [35.041638, 35.868433] | 34.188036 [33.921506, 34.439270] | 17.863048 [17.464560, 18.243728] | 0.095957 [0.000000, 0.253822] | yes |
| B2 | 16 / 306 | 35.629520 [35.202834, 36.046525] | 34.425206 [34.120544, 34.716777] | 17.688253 [17.301434, 18.085916] | 0.050326 [0.000000, 0.150977] | yes |
| B2 | 17 / 324 | 35.776785 [35.443964, 36.116584] | 34.630683 [34.282872, 34.967512] | 17.562581 [17.249294, 17.879319] | 0.030954 [0.000000, 0.092863] | yes |
| B2 | 18 / 342 | 35.861359 [35.533486, 36.181873] | 34.847516 [34.575316, 35.117889] | 17.483317 [17.188632, 17.792793] | 0.000000 [0.000000, 0.000000] | yes |
| B2 | 19 / 360 | 35.928275 [35.586273, 36.240279] | 34.968837 [34.689753, 35.250137] | 17.424377 [17.127619, 17.746447] | 0.000000 [0.000000, 0.000000] | yes |
| B2 | 20 / 378 | 36.137591 [35.911002, 36.360805] | 35.055817 [34.762770, 35.349256] | 17.223840 [17.015325, 17.445397] | 0.000000 [0.000000, 0.000000] | yes |
| B2 | 21 / 396 | 36.159068 [35.941237, 36.364874] | 35.168642 [34.878855, 35.445586] | 17.202141 [17.006896, 17.409556] | 0.000000 [0.000000, 0.000000] | yes |
| B2 | 22 / 414 | 36.171480 [35.952565, 36.379990] | 35.229607 [34.911522, 35.516049] | 17.192899 [16.994618, 17.404592] | 0.000000 [0.000000, 0.000000] | yes |
| B2 | 23 / 432 | 36.236410 [36.041097, 36.414860] | 35.339300 [35.057057, 35.600282] | 17.137609 [16.967884, 17.318582] | 0.000000 [0.000000, 0.000000] | yes |
| B2 | 24 / 450 | 36.236410 [36.047525, 36.416028] | 35.464588 [35.221807, 35.695880] | 17.137609 [16.968592, 17.318753] | 0.000000 [0.000000, 0.000000] | yes |
| B3 | 0 / 18 | 31.008835 [30.683310, 31.405387] | 26.691075 [26.283031, 27.106325] | 21.533777 [21.196295, 21.817095] | 0.767606 [0.619428, 0.909242] | yes |
| B3 | 1 / 36 | 35.130200 [34.528429, 35.863601] | 29.206478 [28.552362, 29.831920] | 17.888429 [17.213731, 18.425020] | 0.960604 [0.735889, 1.195744] | yes |
| B3 | 2 / 54 | 37.977192 [37.254983, 38.666326] | 32.759951 [32.219479, 33.278254] | 15.369474 [14.726330, 16.037335] | 1.516015 [1.307799, 1.706078] | yes |
| B3 | 3 / 72 | 39.122338 [38.315326, 40.054356] | 34.880360 [34.291191, 35.531292] | 14.422726 [13.566676, 15.185623] | 1.819778 [1.530702, 2.088795] | yes |
| B3 | 4 / 90 | 40.318878 [39.760048, 40.911507] | 36.522613 [35.852106, 37.176991] | 13.324551 [12.773871, 13.862564] | 1.865934 [1.576789, 2.119711] | yes |
| B3 | 5 / 108 | 40.974670 [40.581754, 41.397207] | 37.854135 [37.106683, 38.564886] | 12.810125 [12.423236, 13.170926] | 1.635302 [1.221184, 1.997180] | yes |
| B3 | 6 / 126 | 41.462588 [41.004542, 41.883979] | 38.944307 [38.462122, 39.411549] | 12.399287 [12.032493, 12.782940] | 1.530872 [1.248277, 1.777792] | yes |
| B3 | 7 / 144 | 41.917681 [41.455792, 42.363763] | 39.705067 [39.314790, 40.111700] | 12.009345 [11.651291, 12.391780] | 1.378732 [1.114565, 1.671042] | yes |
| B3 | 8 / 162 | 42.138635 [41.758865, 42.515395] | 40.327418 [39.800206, 40.843257] | 11.833252 [11.506205, 12.168875] | 1.238940 [0.901989, 1.610932] | yes |
| B3 | 9 / 180 | 42.613517 [42.191085, 43.058917] | 40.722519 [40.184487, 41.289604] | 11.511956 [11.088464, 11.922073] | 1.381624 [1.053314, 1.724583] | yes |
| B3 | 10 / 198 | 43.134227 [42.836658, 43.516257] | 41.000811 [40.397822, 41.528080] | 11.050621 [10.698566, 11.361762] | 1.354905 [1.012976, 1.716343] | yes |
| B3 | 11 / 216 | 43.285621 [43.006041, 43.635530] | 41.331882 [40.528918, 42.007882] | 10.898473 [10.603389, 11.156640] | 1.250487 [0.897718, 1.621366] | yes |
| B3 | 12 / 234 | 43.507135 [43.233107, 43.805287] | 41.376446 [40.653450, 42.080024] | 10.724266 [10.462512, 10.988899] | 1.078602 [0.699413, 1.463492] | yes |
| B3 | 13 / 252 | 43.689129 [43.367344, 44.008572] | 41.572414 [40.565528, 42.382091] | 10.554744 [10.298852, 10.834244] | 0.943894 [0.606684, 1.322074] | yes |
| B3 | 14 / 270 | 43.886476 [43.559845, 44.207635] | 41.860840 [40.933139, 42.560957] | 10.437165 [10.177590, 10.697602] | 0.979151 [0.593551, 1.393609] | yes |
| B3 | 15 / 288 | 44.044446 [43.731808, 44.317280] | 42.243341 [41.711740, 42.726324] | 10.355089 [10.126812, 10.583589] | 1.063972 [0.826600, 1.314909] | yes |
| B3 | 16 / 306 | 44.279517 [43.858227, 44.740128] | 42.405698 [41.949516, 42.807670] | 10.208260 [9.816633, 10.535285] | 1.151302 [0.894621, 1.386747] | yes |
| B3 | 17 / 324 | 44.523975 [44.060621, 44.976135] | 42.585277 [42.129185, 42.999809] | 10.016377 [9.667979, 10.356893] | 1.350475 [1.082004, 1.541725] | yes |
| B3 | 18 / 342 | 44.577185 [44.090716, 45.023031] | 42.672928 [42.252297, 43.050198] | 9.968538 [9.594615, 10.336663] | 1.330015 [1.133139, 1.490659] | yes |
| B3 | 19 / 360 | 44.727312 [44.357760, 45.096639] | 42.259997 [41.528821, 42.923753] | 9.830662 [9.536997, 10.107795] | 1.303717 [1.029492, 1.525756] | yes |
| B3 | 20 / 378 | 44.813932 [44.457256, 45.167587] | 42.924873 [42.387896, 43.466242] | 9.764539 [9.471923, 10.038906] | 1.293036 [1.009914, 1.554240] | yes |
| B3 | 21 / 396 | 44.934926 [44.567024, 45.284458] | 43.258800 [42.737409, 43.810082] | 9.654831 [9.366514, 9.939204] | 1.270226 [0.977427, 1.563628] | yes |
| B3 | 22 / 414 | 45.016076 [44.628208, 45.374508] | 43.727715 [43.382488, 44.055577] | 9.602307 [9.306921, 9.923639] | 1.317023 [1.065225, 1.569041] | yes |
| B3 | 23 / 432 | 45.090773 [44.710942, 45.439495] | 43.513111 [43.138977, 43.926785] | 9.520537 [9.224980, 9.813592] | 1.274786 [1.065413, 1.478290] | yes |
| B3 | 24 / 450 | 45.131872 [44.778022, 45.452437] | 43.628366 [43.230036, 44.038378] | 9.518449 [9.215962, 9.835706] | 1.332945 [1.101092, 1.546493] | yes |

Hero population-median C labels also use `W:/generation_band_curves/a/24/median_cout_sample_screen_upper_mg_l`: B3 11.008260 [10.580552, 11.423826] mg/L; B1 15.704192 [14.913580, 16.427428] mg/L. Match: yes. All intermediate population-median C, archive-size, hypervolume and exoticness distributions are included in the executable audit even where no numeral is printed on the figures.

## Significance trace — all 16 recorded comparisons

Exact pointers are `W:/pairwise_tests/i/<column>`; raw inputs are `W:/arm_outcomes/{B3,a}/*/<metric>`. U and δ count B3 wins, including reversal for C where lower is better. The Holm family contains all 16 tests at α=0.025. Each comparison below matches the receipt (including its normal-approximation policy). No B0 test exists.

| i | Comparator | Endpoint | Mean difference | U | z | One-sided p | Holm p | Cliff δ | Match? |
|---:|---|---|---:|---:|---:|---:|---:|---:|---|
| 0 | B1 | `final_best_kpi_score` | -0.804948 | 16 | -2.5702 | 0.99491737 | 1.00000000 | -0.6800 | yes |
| 1 | B1 | `final_median_kpi_score` | 5.644746 | 100 | 3.7796 | 0.00007855 | 0.00125680 | +1.0000 | yes |
| 2 | B1 | `final_best_cout_sample_screen_upper_mg_l` | 0.732254 | 17 | -2.4946 | 0.99369441 | 1.00000000 | -0.6600 | yes |
| 3 | B1 | `final_pareto_hypervolume` | -0.007335 | 17 | -2.4946 | 0.99369441 | 1.00000000 | -0.6600 | yes |
| 4 | B2 | `final_best_kpi_score` | 8.895462 | 100 | 3.7796 | 0.00007855 | 0.00125680 | +1.0000 | yes |
| 5 | B2 | `final_median_kpi_score` | 8.163777 | 100 | 3.7796 | 0.00007855 | 0.00125680 | +1.0000 | yes |
| 6 | B2 | `final_best_cout_sample_screen_upper_mg_l` | -7.619160 | 100 | 3.7796 | 0.00007855 | 0.00125680 | +1.0000 | yes |
| 7 | B2 | `final_pareto_hypervolume` | 0.075613 | 100 | 3.7796 | 0.00007855 | 0.00125680 | +1.0000 | yes |
| 8 | A1 | `final_best_kpi_score` | 11.175107 | 100 | 3.7796 | 0.00007855 | 0.00125680 | +1.0000 | yes |
| 9 | A1 | `final_median_kpi_score` | 9.987560 | 100 | 3.7796 | 0.00007855 | 0.00125680 | +1.0000 | yes |
| 10 | A1 | `final_best_cout_sample_screen_upper_mg_l` | -9.365865 | 100 | 3.7796 | 0.00007855 | 0.00125680 | +1.0000 | yes |
| 11 | A1 | `final_pareto_hypervolume` | 0.095407 | 100 | 3.7796 | 0.00007855 | 0.00125680 | +1.0000 | yes |
| 12 | A2 | `final_best_kpi_score` | 2.416283 | 98 | 3.6285 | 0.00014259 | 0.00125680 | +0.9600 | yes |
| 13 | A2 | `final_median_kpi_score` | 5.771655 | 100 | 3.7796 | 0.00007855 | 0.00125680 | +1.0000 | yes |
| 14 | A2 | `final_best_cout_sample_screen_upper_mg_l` | -1.844097 | 98 | 3.6285 | 0.00014259 | 0.00125680 | +0.9600 | yes |
| 15 | A2 | `final_pareto_hypervolume` | 0.032098 | 100 | 3.7796 | 0.00007855 | 0.00125680 | +1.0000 | yes |

For B3-vs-B1 best KPI, `math.erfc` gives normal p=0.994917399054 (published approximation 0.99491737). Enumerating all C(20,10)=184,756 untied rank allocations gives exact one-sided p=0.996579272121. For complete separation U=100, exact p=1/184,756=0.000005412544 instead of approximate 0.00007855. These sensitivity values are **not** silently substituted into the published Holm family. A claim of “exact Mann–Whitney p” for this receipt would be unsupported.

## Authored-family summaries and missing geometry holdout

Pointers are `W:/held_out_geometry_generalization/i`, recomputed using `W:/promoted_wave3_elites_by_family/<family>/{mc_robustness,nominal_metrics,constraint_violation}` and the KPI formula. Rediscovery counts are unique B3 seed IDs appearing in milestone child labels or the final winner’s label. These are screen label counts, not blinded geometry generalization. The source contains 11 summary rows, but no withheld-geometry search arm, independent labels or raw held-out prediction/residual ledger. Family RMSE-C values can be located but cannot be independently reconstructed from the aggregate-only receipt. That missing residual ledger is included in W4-G2, even if a later figure prints the aggregate RMSE correctly.

| i / family | File:JSON key | Recomputed KPI | Unique B3 rediscovery seeds / 10 | Recorded fold n / C RMSE (mg/L) | KPI RMSE | Match? |
|---|---|---:|---:|---|---|---|
| 0 / baseline-ct35 | `W:/held_out_geometry_generalization/0` | 23.931038 | 0/10 | 1 / 5.657403 (recorded only) | null | yes KPI/count; **no KPI error bar (G2)** |
| 1 / bifurcated-quad-manifold | `W:/held_out_geometry_generalization/1` | 32.740092 | 0/10 | 2 / 5.334092 (recorded only) | null | yes KPI/count; **no KPI error bar (G2)** |
| 2 / contoured-s-cone-skimmer | `W:/held_out_geometry_generalization/2` | 34.194461 | 0/10 | 3 / 2.124581 (recorded only) | null | yes KPI/count; **no KPI error bar (G2)** |
| 3 / counter-fluted-recovery-vortex | `W:/held_out_geometry_generalization/3` | 45.263780 | 2/10 | 2 / 0.317714 (recorded only) | null | yes KPI/count; **no KPI error bar (G2)** |
| 4 / cylindrical-gyroid-coalescer | `W:/held_out_geometry_generalization/4` | 45.787472 | 4/10 | 4 / 1.730490 (recorded only) | null | yes KPI/count; **no KPI error bar (G2)** |
| 5 / helical-pitched-ribbon | `W:/held_out_geometry_generalization/5` | 31.605194 | 0/10 | 3 / 2.378800 (recorded only) | null | yes KPI/count; **no KPI error bar (G2)** |
| 6 / hybrid-omni-exotic | `W:/held_out_geometry_generalization/6` | 41.842372 | 10/10 | 4 / 3.231820 (recorded only) | null | yes KPI/count; **no KPI error bar (G2)** |
| 7 / octa-branch-helical-manifold | `W:/held_out_geometry_generalization/7` | 46.593857 | 8/10 | 2 / 1.987614 (recorded only) | null | yes KPI/count; **no KPI error bar (G2)** |
| 8 / schwarz-p-rectifier-head | `W:/held_out_geometry_generalization/8` | 45.936340 | 10/10 | 10 / 2.369418 (recorded only) | null | yes KPI/count; **no KPI error bar (G2)** |
| 9 / spiral-rifled-vortex-head | `W:/held_out_geometry_generalization/9` | 33.466058 | 0/10 | 3 / 1.088688 (recorded only) | null | yes KPI/count; **no KPI error bar (G2)** |
| 10 / trifurcated-6jet-vortex-head | `W:/held_out_geometry_generalization/10` | 44.717292 | 8/10 | 3 / 1.452800 (recorded only) | null | yes KPI/count; **no KPI error bar (G2)** |

The four saved off-design **operating duties** are separate from geometry holdout: `W:/promoted_wave3_top_elite_held_out_duties` versus `/baseline_b0_held_out_duties`. They remain outputs of the same screen. Their recorded screen concentration values (mg/L) are:

| i / duty | Baseline C | Promoted B3 C | Derived reduction | Source | Match? |
|---|---:|---:|---:|---|---|
| 0 / D1_turndown | 22.713638 | 9.114021 | 59.8742% | `W:/{baseline_b0,promoted_wave3_top_elite}_held_out_duties/0/cout_mg_l` | yes extraction/arithmetic; no independent duty evaluation |
| 1 / D2_turnup | 29.466510 | 7.284262 | 75.2795% | `W:/{baseline_b0,promoted_wave3_top_elite}_held_out_duties/1/cout_mg_l` | yes extraction/arithmetic; no independent duty evaluation |
| 2 / D3_sheared_feed | 44.646483 | 13.544257 | 69.6633% | `W:/{baseline_b0,promoted_wave3_top_elite}_held_out_duties/2/cout_mg_l` | yes extraction/arithmetic; no independent duty evaluation |
| 3 / D4_cold_viscous_slug | 44.607167 | 18.881296 | 57.6721% | `W:/{baseline_b0,promoted_wave3_top_elite}_held_out_duties/3/cout_mg_l` | yes extraction/arithmetic; no independent duty evaluation |

## Lineage counts and figure callouts

Pointers: `W:/arm_outcomes/a/i/lineage_milestones`, `/total_cross_family_lineage_jumps` and `/generation_trace/24/cumulative_lineage_jumps`. Milestone counts and milestone cross-label flags are counted directly. All-proposal jump totals only reconcile against the cumulative trace: the full proposal ledger was removed (`proposal_receipts=[]` in all 50 runs), so independent recount of every proposal event is unavailable (W4-G7). Milestones record incumbent improvements from multiple candidates; the ordered list is not proof that each successive candidate is a direct descendant of the previous one.

| Arm | Seed | Counted milestones | Cross-label milestone flags | Recorded all-proposal jumps / final cumulative trace | Match? |
|---|---:|---:|---:|---|---|
| A1 | 20261101 | 14 | 1 | 9 / 9 | yes counts/consistency; full recount unavailable |
| A1 | 20261102 | 11 | 1 | 13 / 13 | yes counts/consistency; full recount unavailable |
| A1 | 20261103 | 15 | 1 | 16 / 16 | yes counts/consistency; full recount unavailable |
| A1 | 20261104 | 15 | 1 | 9 / 9 | yes counts/consistency; full recount unavailable |
| A1 | 20261105 | 11 | 1 | 2 / 2 | yes counts/consistency; full recount unavailable |
| A1 | 20261106 | 8 | 2 | 17 / 17 | yes counts/consistency; full recount unavailable |
| A1 | 20261107 | 11 | 1 | 1 / 1 | yes counts/consistency; full recount unavailable |
| A1 | 20261108 | 10 | 1 | 9 / 9 | yes counts/consistency; full recount unavailable |
| A1 | 20261109 | 15 | 1 | 9 / 9 | yes counts/consistency; full recount unavailable |
| A1 | 20261110 | 11 | 1 | 4 / 4 | yes counts/consistency; full recount unavailable |
| A2 | 20261101 | 21 | 5 | 102 / 102 | yes counts/consistency; full recount unavailable |
| A2 | 20261102 | 26 | 7 | 111 / 111 | yes counts/consistency; full recount unavailable |
| A2 | 20261103 | 19 | 8 | 120 / 120 | yes counts/consistency; full recount unavailable |
| A2 | 20261104 | 21 | 10 | 150 / 150 | yes counts/consistency; full recount unavailable |
| A2 | 20261105 | 19 | 7 | 117 / 117 | yes counts/consistency; full recount unavailable |
| A2 | 20261106 | 20 | 10 | 146 / 146 | yes counts/consistency; full recount unavailable |
| A2 | 20261107 | 19 | 7 | 150 / 150 | yes counts/consistency; full recount unavailable |
| A2 | 20261108 | 21 | 8 | 104 / 104 | yes counts/consistency; full recount unavailable |
| A2 | 20261109 | 22 | 7 | 137 / 137 | yes counts/consistency; full recount unavailable |
| A2 | 20261110 | 21 | 9 | 110 / 110 | yes counts/consistency; full recount unavailable |
| B1 | 20261101 | 7 | 7 | 432 / 432 | yes counts/consistency; full recount unavailable |
| B1 | 20261102 | 8 | 8 | 432 / 432 | yes counts/consistency; full recount unavailable |
| B1 | 20261103 | 3 | 3 | 432 / 432 | yes counts/consistency; full recount unavailable |
| B1 | 20261104 | 5 | 5 | 432 / 432 | yes counts/consistency; full recount unavailable |
| B1 | 20261105 | 1 | 1 | 432 / 432 | yes counts/consistency; full recount unavailable |
| B1 | 20261106 | 1 | 1 | 432 / 432 | yes counts/consistency; full recount unavailable |
| B1 | 20261107 | 4 | 4 | 432 / 432 | yes counts/consistency; full recount unavailable |
| B1 | 20261108 | 5 | 5 | 432 / 432 | yes counts/consistency; full recount unavailable |
| B1 | 20261109 | 4 | 4 | 432 / 432 | yes counts/consistency; full recount unavailable |
| B1 | 20261110 | 1 | 1 | 432 / 432 | yes counts/consistency; full recount unavailable |
| B2 | 20261101 | 19 | 1 | 34 / 34 | yes counts/consistency; full recount unavailable |
| B2 | 20261102 | 21 | 4 | 66 / 66 | yes counts/consistency; full recount unavailable |
| B2 | 20261103 | 21 | 3 | 62 / 62 | yes counts/consistency; full recount unavailable |
| B2 | 20261104 | 24 | 2 | 38 / 38 | yes counts/consistency; full recount unavailable |
| B2 | 20261105 | 21 | 3 | 48 / 48 | yes counts/consistency; full recount unavailable |
| B2 | 20261106 | 24 | 1 | 47 / 47 | yes counts/consistency; full recount unavailable |
| B2 | 20261107 | 20 | 1 | 41 / 41 | yes counts/consistency; full recount unavailable |
| B2 | 20261108 | 20 | 1 | 60 / 60 | yes counts/consistency; full recount unavailable |
| B2 | 20261109 | 14 | 2 | 49 / 49 | yes counts/consistency; full recount unavailable |
| B2 | 20261110 | 21 | 3 | 63 / 63 | yes counts/consistency; full recount unavailable |
| B3 | 20261101 | 29 | 8 | 134 / 134 | yes counts/consistency; full recount unavailable |
| B3 | 20261102 | 25 | 10 | 164 / 164 | yes counts/consistency; full recount unavailable |
| B3 | 20261103 | 19 | 7 | 154 / 154 | yes counts/consistency; full recount unavailable |
| B3 | 20261104 | 21 | 7 | 158 / 158 | yes counts/consistency; full recount unavailable |
| B3 | 20261105 | 22 | 9 | 173 / 173 | yes counts/consistency; full recount unavailable |
| B3 | 20261106 | 19 | 9 | 161 / 161 | yes counts/consistency; full recount unavailable |
| B3 | 20261107 | 21 | 10 | 148 / 148 | yes counts/consistency; full recount unavailable |
| B3 | 20261108 | 14 | 9 | 165 / 165 | yes counts/consistency; full recount unavailable |
| B3 | 20261109 | 20 | 8 | 187 / 187 | yes counts/consistency; full recount unavailable |
| B3 | 20261110 | 22 | 8 | 158 / 158 | yes counts/consistency; full recount unavailable |

| Arm | Total milestones | Cross-label milestone flags | Recorded all-proposal jumps |
|---|---:|---:|---:|
| A1 | 121 | 11 | 89 |
| A2 | 209 | 78 | 1247 |
| B1 | 39 | 39 | 4320 |
| B2 | 205 | 21 | 508 |
| B3 | 212 | 85 | 1602 |

The ablation figure’s highlighted seed is **20261108**, with **14 milestones**, selected by maximum final B3 KPI. Every numerical callout is:

| Milestone index | File:JSON key | g | Jets | KPI (plotted to 2 decimals) | TPMS label | Match? |
|---:|---|---:|---:|---|---|---|
| 0 | `W:/arm_outcomes/B3_Wave3_Exotic3D_QD_CEI/7/lineage_milestones/0` | 0 | 4 | 30.683537 (30.68) | None | yes |
| 4 | `W:/arm_outcomes/B3_Wave3_Exotic3D_QD_CEI/7/lineage_milestones/4` | 3 | 6 | 38.019838 (38.02) | Diamond | yes |
| 9 | `W:/arm_outcomes/B3_Wave3_Exotic3D_QD_CEI/7/lineage_milestones/9` | 7 | 8 | 42.504686 (42.50) | None | yes |
| 13 | `W:/arm_outcomes/B3_Wave3_Exotic3D_QD_CEI/7/lineage_milestones/13` | 16 | 8 | 45.936340 (45.94) | SchwarzP | yes |

## Hero schematic and archived CFD numbers

The header, mean/CI bands and legend numbers are traced above. The following are the additional data-bearing numerals embedded in hero Panels C/D. Diagram layout coordinates and tick marks are drawing instructions, not measurements. Archived CFD is traced to C/E, not falsely attributed to W.

| Figure number/claim | File:JSON key | Recomputed or source value | Match? |
|---|---|---|---|
| B0 KPI / C label | `W:/baseline_b0_ct35_kpi_score`, `/baseline_b0_ct35_cout_sample_screen_upper_mg_l` | 23.93 / 27.63 mg/L | yes |
| Elite KPI / C / reduction | `W:/promoted_wave3_top_elite_kpi_score`, `/promoted_wave3_top_elite/mc_robustness/cout_sample_screen_upper_mg_l` | 45.94 / 8.83 mg/L / -68.0% | yes screen arithmetic |
| 6 drawn elite jets | `W:/promoted_wave3_top_elite/program/inlet_split/branches_per_port` | 2 fixed ports × 4 branches = 8, not 6 | **no — G5** |
| dmax 69.2 / 91.6 μm | `W:/{baseline_b0_ct35,promoted_wave3_top_elite}/nominal_metrics/hinze_dmax_um` | 64.000000 / 98.994002 μm | **no — G5** |
| TPMS inner radius 0.70R / 100% open core / Do=1.40 mm / exact scale | `W:/promoted_wave3_top_elite/program/tpms_insert/radial_inner_fraction` | 0.7417R; remaining assertions absent as measured geometry evidence | **no — G5** |
| B0 2 ports | `W:/baseline_b0_ct35/program/inlet_split` | disabled split, fixed two-port baseline | yes gene contract, not schematic metrology |
| 15 archived runs / 3 elites / 3 grids / 2 RSM closures | `C:/wave1_vs_wave2_surrogate_to_cfd_ranking_audit` and 5-run dossiers in C/E | 3 subjects × (G0,G1,G2,LRR,SSG) = 15; CT-35 reference separate | yes archived count; **not Wave-4 validation** |
| CT-35 three repeated bars, zero error | `C:/wave2_tpms_gyroid_elite_5run_dossier/baseline_ct35_comparator/dp_u_pa` | 29.468219 kPa single SST reference; no baseline LRR/SSG or baseline GCI in inputs | **no for duplicated RSM/zero uncertainty — G6** |
| cylindrical-gyroid-coalescer: G2/LRR/SSG kPa / change | `C:/wave1_vs_wave2_surrogate_to_cfd_ranking_audit/0/openfoam_{g2,lrr,ssg}_dp_u_pa` | 18.723939 / 18.245098 / 18.154237; -36.5% vs archived baseline | yes conversion |
| trifurcated-6jet-vortex-head: G2/LRR/SSG kPa / change | `C:/wave1_vs_wave2_surrogate_to_cfd_ranking_audit/1/openfoam_{g2,lrr,ssg}_dp_u_pa` | 18.699404 / 18.119200 / 18.165706; -36.5% vs archived baseline | yes conversion |
| octa-branch-helical-manifold: G2/LRR/SSG kPa / change | `C:/wave1_vs_wave2_surrogate_to_cfd_ranking_audit/2/openfoam_{g2,lrr,ssg}_dp_u_pa` | 18.628967 / 18.326401 / 17.960570; -36.8% vs archived baseline | yes conversion |
| trifurcated ± “GCI21” | `E:/roache_gci_analysis/dp_u_pa` | (max(G0,G1,G2)−min(G0,G1,G2))/2/1000 = 1.107781 kPa; standard GCI=null, Oscillatory | **no label — G6** |
| gyroid ± “GCI21” | `C:/wave2_tpms_gyroid_elite_5run_dossier/roache_gci_analysis/dp_u_pa` | (max(G0,G1,G2)−min(G0,G1,G2))/2/1000 = 1.078521 kPa; standard GCI=null, Oscillatory | **no label — G6** |
| octa ± “GCI21” | `C:/wave2_octa_branch_8jet_elite_5run_dossier/roache_gci_analysis/dp_u_pa` | (max(G0,G1,G2)−min(G0,G1,G2))/2/1000 = 1.141436 kPa; standard GCI=null, Oscillatory | **no label — G6** |
| 552k/553k G2 / 305k RSM / 100% realizable labels | archived case mesh/solver records, not W | Rounded mesh descriptions require per-case meshes; no 100%-realizable numerical ledger in W | **not independently established here — G6** |

## README and WHITEPAPER §5.6 publication trace

At cb31ba3 the README hero/caption/table still explicitly use **Wave-3** and source `wave3-evolution-proof-v1.json`; the Wave-4 PNG exists but is not embedded. Its historical numbers must not be relabelled as Wave-4. WHITEPAPER §5.6 is a smoke report, not the new multi-seed study. Thus no new Wave-4 multi-seed number in these prose locations can be signed off as already published (W4-G1). The complete replacement Wave-4 values are in the tables above. The following traces every empirical numeral in the existing §5.6 smoke prose against its now-refreshed source S. Item IDs, section labels and field names are identifiers, not measurements.

| §5.6 printed quantity | File:JSON key / derivation | Recomputed | Match? |
|---|---|---|---|
| 3 seeds; 2 islands ×9; 6 generations; MC12 | `S:/config` | 3; 2×9; 6; 12 | yes |
| 0 disallowed fallback sites | `S:/zero_silent_baseline_fallbacks_audit/disallowed_fallback_matches` | 0 | yes |
| 381 proposals | `S:/aggregate_proposal_budget/budget` | 3×127=381 | yes |
| 0 invalid /0.0% | `S:/aggregate_proposal_budget/proposals_invalid` | 0 /0.0% | yes |
| 237 repaired /62.2% | `S:/aggregate_proposal_budget/proposals_repaired` | 228 /381=59.8425% | **no — G1** |
| 144 unrepaired /37.8% | `S:/aggregate_proposal_budget/evaluations_unrepaired` | 153 /381=40.1575% | **no — G1** |
| mutation121 attempts /78.5% repaired | `S:/operator_statistics/mutate` | 133 attempts;104/133=78.1955% | **no — G1** |
| macro crossover203 attempts /70.0% | `S:/operator_statistics/non_homologous_macro_crossover` | 191 attempts;124/191=64.9215% | **no — G1** |
| inlet_ramp 124 touches | `S:/repair_fields_histogram/~1inlet_ramp` | 101 | **no — G1** |
| cone_profile 85 touches | `S:/repair_fields_histogram/~1cone_profile` | 55 | **no — G1** |
| skimmer 82 touches | `S:/repair_fields_histogram/~1skimmer` | 127 | **no — G1** |
| inlet_split 58 touches | `S:/repair_fields_histogram/~1inlet_split` | 43 | **no — G1** |
| tpms_insert 5 touches | `S:/repair_fields_histogram/~1tpms_insert` | 4 | **no — G1** |
| ribbon_cascade 2 touches | `S:/repair_fields_histogram/~1ribbon_cascade` | key absent (not the printed 2) | **no — G1** |

## Handoff register

| Gap | Exact missing or mismatched evidence | Owner / disposition |
|---|---|---|
| W4-G1 | README still historical Wave-3; §5.6 smoke numbers stale and INT-04 described as blocked. New best-score result favors random. | GIVE EV → INT/DOC 07:07Z; DOC owns prose/NUMBERS_TRACE. |
| W4-G2 | No unseen-geometry search arm or raw out-of-fold predictions; KPI RMSE null for all 11 families; C-RMSE residual ledger absent. Figure’s missing-key fallback invents zero uncertainty. | GIVE EV → INT/DOC 07:07Z. INT ACK 07:09Z reports plotting fix, not yet reviewed as a commit in this cb31ba3 audit. Raw validation remains absent. |
| W4-G3 | CT-35 is a single frozen reference, no 10-seed equal-450 arm, CI or pairwise test. Reference/family/duty work outside 22,500 search proposals. | GIVE EV → INT/DOC 07:07Z; describe the accounting scope, do not invent replications. |
| W4-G4 | PNGs default MC n=28 while all saved candidate records have72 and manifests say72. | GIVE EV → INT/DOC 07:09Z; infer from recorded samples or persist actual config and regenerate. |
| W4-G5 | Schematic hardcoded jets/dmax/radius do not describe the saved elite; exact-scale/core/physical ceiling labels unsupported. | GIVE EV → INT/DOC 07:09Z; use actual values and label schematic/screen scope. |
| W4-G6 | Oscillatory half-span labelled GCI; archived subjects conflated with new winner; duplicate CT-35 RSM bars and zero-error baseline unsupported. | GIVE EV → INT/DOC 07:09Z plus DOC 07:07Z figure NEED; preserve missing uncertainty and actual subject identity. |
| W4-G7 | Aggregate all-proposal lineage counts cannot be fully recounted because proposal receipts and rejected/other ancestry events were compacted away. Milestones are incumbent events, not a reconstructed pedigree. | GIVE EV → INT/DOC with final trace; count available milestones and label aggregate consistency only. |

## Verification receipts

Positive audit: command above, exit0; all generation/final summary and rank-test comparisons pass, all 50 partitions and generation lengths pass, 11 family KPI/rediscovery counts pass. Negative probes: a planted +1 in B3 mean produces the exact summary-pointer mismatch; changing a single B1 budget to449 raises an assertion; a planted 1e-7 change in the recorded p-value is rejected at its exact JSON pointer. Full foundation gate and public-content check are recorded in the [accompanying lane receipt](../../crates/joil-evidence/receipts/Wave4-number-trace.json) and final live-board DONE/GIVE.

| Artifact (ignored local evidence) | SHA-256 |
|---|---|
| `.private/wave4-number-trace-recomputed.json` | `6dd35fb98456a7791acbd0e2d5838f57560927bdf6be9dc67e87a4eb3a9358a8` |
| Independent helper | `a6794534e2a086edccc758a1bf4d8e85b777990e50d999edcf916d3c4dfb3f69` |
