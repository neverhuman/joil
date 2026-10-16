# TR deliverable numbers audit — 16 October 2026

Audited `origin/lane/paper@8eb439e8c33b799a235c97d40d5cffbf5807eed4` in a detached scratch worktree. README and WHITEPAPER were read fully, prioritizing sections1–5; AppendixA, captions, the artifact index and `docs/NUMBERS_TRACE.md` were also checked. DOC source files were not edited.

**Result:30 grouped NEEDs sent to DOC.**349 checks bound major table entries to their actual arm/statistic/grid/duty fields and all passed. Of72 printed SHA256 identifiers,71 match exact selected public bytes or an embedded reference; one baseline identifier has no source. Prose, scope and provenance issues below remain open. This is not physical validation.

The [machine receipt](2026-10-16-TR-deliverable-numbers-audit.json) records all input hashes,349 field checks,72 hash checks,30 findings, full per-line coverage (1924 legacy-parser numeral tokens on294 lines), a complete digit-bearing-line inventory including metadata/compound counts, spelled counts, the exact reproducer and command logs/digests. The legacy token count includes unit powers and splits some hyphenated thousands; it is not a count of measured quantities.

A coincident number is not provenance. The existing audit on this paper head returned0 (1364 numeric tokens,152 structured checks), while the independent checks found wrong or absent quantity bindings. The trace ledger covers the front matter, Abstract, section4.6, section5 and README; it omits sections1–4.5 and AppendixA.

## Findings handed to DOC

Line numbers below refer to the pinned paper commit. Where the observation is absent, the correct source value is **unknown** until a selected result is committed. Provisional protocol values are listed as declarations rather than measurement errors.

### NUM-01

docs/WHITEPAPER.md:64 across-all-grids pressure18.12–19.53kPa and energy21.54–23.12kJ/m3 omit G0. Correct ranges17.31–19.53kPa and21.40–23.12kJ/m3; elite-3grid-gci-and-rsm-verification-v1.json#/grids/0 has dp_u_pa17310.97248047435 and hydraulic_loss_j_per_treated_m321398.4083725502.

### NUM-02

README.md:48,70 and docs/WHITEPAPER.md:552 claim zero executed repaired/Wave4 designs, stale against wave4-gci-v1.json#/wave4_b3_top_winners/1 (subject s20261106-g023-i001, solver_status Solved3Grid,3 grid results). Correct1 Wave4 subject solved on3 grids; promoted s20261108 remains unsolved. Also scope historical two-inlet assertions in WHITEPAPER:37,61,355,450 to Wave1/2 shells; Wave4 solved subject enacts8 paths.

### NUM-03

docs/WHITEPAPER.md:214 says no same-Rust-pipeline CT35 exists. Correct1 same-pipeline control solved on3 grids; wave4-gci-v1.json#/same_pipeline_ct35_control/grids/2 dp_u_pa18633.8907284468. Replace stale zero-control statement with section4.6 control reference.

### NUM-04

docs/WHITEPAPER.md:116 underflow_reverse_flow_m3_s=1.56e-4 lacks matching committed baseline measurement. baseline-hydraulics-v1.json#/points/2/external_outlet_reverse_flow_m3_s/underflow =0 (overflow also0). Correct external underflow value0; original device-overflow plane separately has reverse_flow5.28735850997123e-5 and must not be conflated.

### NUM-05

docs/WHITEPAPER.md:269 2.85%–3.10% applies only to pressure vs G2; specific-energy differences are2.21%–2.29%. Derive from elite-3grid-gci-and-rsm-verification-v1.json#/rsm_turbulence_closure_crosscheck/{0,1}/hydraulic_loss_j_per_treated_m3 =21560.7533668708/21543.5183257735 vs /grids/2=22047.3249453656. Split pressure and energy ranges.

### NUM-06

docs/WHITEPAPER.md:286 formula0.725*(0.025/998)^(3/5)*94.3^(-2/5) gives204.088723768142um, not91.6um. Moreover no committed result field records the quoted epsilon_P99=94.3; correct derived hypothetical value204.1um, observed value unknown until a field-extraction receipt is committed. Remove unsupported breakup-prevention claim.

### NUM-07

docs/WHITEPAPER.md:402 says Wave2 bootstrap10,000 resamples; historical intervals used2,000 (C6 and section5.3 already say this). Correct2,000 for historical plate;10,000 is preregistration/current repaired code. git show 2f24c18^:crates/joil-core/src/evolve.rs has n_boot=2000usize; JSON stores CI endpoints but no resample-count field.

### NUM-08

docs/WHITEPAPER.md:444 raw p0.000157 is a two-sided NORMAL approximation, not minimum exact rank-sum p for n1=n2=10. Exact no-tie two-sided extreme-U p=2/C(20,10)=0.000010825088224469; committed wave2-multiseed-ablation-and-holdout-v1.json#/pairwise_tests/0 instead records ONE-sided normal p7.855e-5, Holm0.0003142 across4 tests. Keep receipt values but label one-sided normal approximation (also remove exact at WHITEPAPER:361).

### NUM-09

docs/WHITEPAPER.md:453 worst rejected Cout7.42mg/L and min_cout_safety_margin_mg_l=0.00 are not recorded in rejected_cohort_10pct_audit. Correct supported fields in wave2-multiseed-ablation-and-holdout-v1.json#/rejected_cohort_10pct_audit: audited64/pool640,false_omission_rank0_count0,worst_rank_reversal_cout_mg_l0.0; worst rejected Cout is unknown.7.421919 is B3 mean CI upper endpoint, not a rejected-candidate measurement.

### NUM-10

docs/WHITEPAPER.md:229 source conflict: baseline closure1.63e-5 matches elite-3grid-gci-and-rsm-verification-v1.json#/baseline_ct35_comparator/water_closure_fraction, but baseline-hydraulics-v1.json#/points/2/observations/water_closure_fraction for the same ct35-sst-extended-000/t600 is0.00015843274577089387 (1.58e-4). Resolve against retained native-integral hash363da337...; do not call1.63e-5 a verified common baseline observation.

### NUM-11

docs/WHITEPAPER.md:280,281,282,284,286,289 no committed results field records measured Utheta5.2/3.6/1.8–2.5m/s, radii30/8mm, axial ranges105–880/120–780mm, core pressure-16..-22kPa, entry wall delta10.8kPa, epsilonP99=94.3 or64 sampling stations. Cited elite-3grid-gci-and-rsm-verification-v1.json contains hydraulic scalars and Reynolds-stress stats, no extracted velocity/pressure profiles or Hinze field. Correct measured values UNKNOWN until commit selected field-extraction result; plot grid settings can be labelled authored instead.

### NUM-12

docs/WHITEPAPER.md:159 minimum overflow/underflow throat clearance >=3.5mm contradicts actual1.4mm screen floor. crates/joil-core/src/redteam.rs MIN_PASSAGE_DIAMETER_MM=1.4; rust-evolution-qualification-v1.json#/promoted_3d_elite/min_passage_diameter_mm=1.4000000000000001. Correct1.4mm analytic/provisional floor; do not invent a3.5mm admitted requirement.

### NUM-13

docs/WHITEPAPER.md:161 64 structural axial stations is unrecorded/wrong for linked structure.rs: historical preTR07 git1eeff10^ evaluates256/512/1024, current source adds128 as fourth grid. Correct historical three levels256/512/1024 or current four128/256/512/1024 with explicit provenance;64 is not a stored result field.

### NUM-14

docs/WHITEPAPER.md:408 lock-in exclusion ratio>=2.20 has no matching structural requirement/result. Actual provisional structure.rs MIN_MODAL_FREQUENCY_RATIO=3.0; Wave2 promoted gyroid /promoted_wave2_elites_by_family/cylindrical-gyroid-coalescer/structural_qualification/vane_modal_freq_ratio=31.026934102709003. Figure generator additionally draws its own1.50 ceiling; label illustrations distinctly and use3.0 when describing implemented screen.

### NUM-15

docs/WHITEPAPER.md:405 claims ten grade bins over5–100um; committed wave2-multiseed-ablation-and-holdout-v1.json#/baseline_b0_ct35/nominal_metrics/grade_efficiency_bins keys are3,5,8,10,15,20,25,40,60,100um. Correct source range3–100um. Figure script separately authors DSD bin centres6,10,14.5,19.5,25,31.5,39,48,60,76um; do not treat those as receipt binning.

### NUM-16

docs/WHITEPAPER.md:411 labels rendered promoted gyroid as2->6 jets, radial0.70R and z22..62mm, I_exotic1.73 with range0.48..1.73. Actual wave2-multiseed-ablation-and-holdout-v1.json#/promoted_wave2_elites_by_family/cylindrical-gyroid-coalescer/program has branches_per_port4 (8 labelled jets), tpms radial_inner_fraction0.7529, axial_start29.0971/end58.4138mm. I1.73/range0.48..1.73 are hardcoded in figure generator, not result measurements. Correct genes or explicitly label those particular numbers authored schematic constants;100% unobstructed core is not measured.

### NUM-17

docs/WHITEPAPER.md:405 Mean +/-95% CI ribbons/error bars are not stored or resampled grade/DSD confidence bands: tools/generate_wave2_figures_and_animations.py authors Rosin–Rammler exponents and +/-0.35um width. Correct label illustrative envelope; actual cohort-level95% interval UNKNOWN. Source JSON has per-candidate grade bins and seed summaries, not those plotted bands.

### NUM-18

docs/WHITEPAPER.md:172,173,174,176 per-node seed20261015/20262026/20263037,16CPU threads,16islands,45generations,24GiB and total24124.19MiB/headroom3.42GiB have no run-configuration fields in cited rust-evolution-qualification-v1.json. That receipt supports3 named nodes/devices, peak used20627.12MiB and85.5% only. Commit selected per-node run-config/resource receipt or label these unverified historical configuration; exact total/headroom source UNKNOWN.

### NUM-19

docs/WHITEPAPER.md:129,152 ring-mesh triangle range2976–6928 has no matching value in any committed result/receipt. Correct range UNKNOWN until a per-subject ring-mesh census is committed; do not substitute CFD triangle counts (6416 in exotic-vs-baseline-cfd-benchmark-v1.json#/promoted_exotic_3d_cfd_case/total_triangles is a different surface).

### NUM-20

README.md:161 and docs/WHITEPAPER.md:390 arithmetic minimum wall clearance>=1.24mm and minimum section Jacobian1.000 are absent from cited closed-catalog execution receipt docs/receipts/wave2-genome-and-10seed-ablation-20261015.json. That receipt records220 samples/pass rate1 and85 label transitions but no extremal clearance/Jacobian. Correct historical extrema UNKNOWN pending selected audit result; ledger matching1.24 to unrelated vane thickness is not evidence.

### NUM-21

docs/WHITEPAPER.md:578 every executed sequence is Oscillatory is stale once section4.6 Wave4 is included. wave4-gci-v1.json#/wave4_b3_top_winners/1/roache_gci_v2_analysis/dp_u_pa/gci_validity_state=Monotone, observed_order_p3.076521759775982; its reject_fraction state=Divergent. Scope all-Oscillatory statement to archived Wave1/2 dossiers only; standard GCI remains null.

### NUM-22

docs/NUMBERS_TRACE.md:1,3 coverage excludes WHITEPAPER sections1–4.5 and Appendix A; it is not an every-number audit of the paper. It also binds bootstrap10000 to continuation chunk_steps,220 validity samples to unrelated Wave4 evaluations,95 confidence to mesh face indices and README1.24 clearance to vane thickness. Correct source bindings are named statistics/protocol/closed-catalog fields or explicit missing-result status; extend coverage without treating numerical coincidence as provenance.

### NUM-23

no-result declarations inventory: WHITEPAPER:326-327 physical envelope ranges (rho820–890,sigma15–35,oil0.10%,flow2.60–5.40,split1.50–3%,roughness35/12um,gas1%,microemulsion8um/5mN/m,solid150um),:334 G3>=1.2e6 cells,:335/:346 builds3/CT25um/throat+/-50um/area+/-1.5%,:347 flow-loop500+/-25mg/L,d32=22+/-2um,r2+/-0.05%,power90%,alpha0.025,:348 capacity+10%/LCB95% have NO empirical result. Keep them explicitly proposed/provisional; SCIENCE/prereg/NUMBERS_TRACE declarations are policy sources, not admitted or measured limits. Correct observed/admitted values remain UNKNOWN; no numeric limits invented by TR.

### NUM-24

docs/WHITEPAPER.md:454 -36.46%..-39.05% is not range across all15 G0/G1/G2/LRR/SSG results. Correct all15 range-41.7528542414%..-33.5685477332% (-41.75%..-33.57%), from wave2-tpms-elite-gci-rsm-and-f2-oil-v1.json#/wave2_octa_branch_8jet_elite_5run_dossier/grids/0/dp_u_pa=17164.39654881485 and /wave2_tpms_gyroid_elite_5run_dossier/grids/1/dp_u_pa=19576.165924881498 vs baseline29468.219129471345. Alternatively explicitly scope current range to G2/RSM only.

### NUM-25

docs/WHITEPAPER.md:297 injected_per_bin128,dt5e-5 vs2.5e-5s,short-window0.01–0.10s and97.2% retained are not recorded in the cited rust-evolution-qualification-v1.json#/flux_weighted_inlet_cohort_probe (that block records only pass/discrepancy/eligibility flags).10 cohorts/0 discrepancies are backed elsewhere. A separate Wave2 stage0/t150 records dt5e-5,end0.01,remaining0.971875; do not silently use that as t600/dt-halving evidence. Commit selected per-run cohort inputs/remaining inventories or mark unrecorded quantities UNKNOWN.

### NUM-26

docs/WHITEPAPER.md:229 baseline effective h1.545mm has no recorded baseline volume or h field in elite-3grid-gci-and-rsm-verification-v1.json#/baseline_ct35_comparator; it is not derivable from cells334768 alone. Correct value UNKNOWN pending actual volume/effective-size source; do not infer from a different extended/unextended geometry.

### NUM-27

docs/WHITEPAPER.md:116 SHA05c5c75a6164be45b3e24ffeb4b4ef0688081e46894bf865be1f5d3282aa3402 matches no public result/artifact or embedded result/receipt digest; claims_audit merely hardcodes it as allowed. If citing committed baseline-hydraulics-v1.json, correct file SHA256 is2c53abe39be29a9ae9a160836773d35be761005a191ea3eaa093e58f5c5e4461. Otherwise name/publish the original referenced artifact; do not substitute a new hash for an unnamed old subject.

### NUM-28

docs/WHITEPAPER.md:414 n180 Lagrangian trajectories with sizes10–65um is not a recorded transport result. tools/generate_wave2_figures_and_animations.py draws3 authored spiral curves per frame (j_branch in range(3)),90 points each, with no integrated droplet sizes. Correct rendered count3 illustrative curves; actual simulated-trajectory count0 in this panel, size interval not measured. The adjacent1800 B3 evaluations is separately derivable and supported.

### NUM-29

docs/WHITEPAPER.md:482 says Wave3 B3 mean best crosses B1 by generation14. Correct first crossing is generation16: wave3-evolution-proof-v1.json#/generation_band_curves/B3_Wave3_Exotic3D_QD_CEI/14/best_kpi_score/mean=45.868584 vs B1/14=46.013244; g15=45.999479 vs46.026659; g16=46.173056 vs46.068869. Replace14 with16.

### NUM-30

docs/WHITEPAPER.md:372 claims dp_u<=29.5kPa feasibility budget; implemented historical and repaired sandbox cap is1.15*baseline, i.e.33888.453Pa=33.888453kPa for baseline29468.22. Sources requirements/joil-hydrocyclone-v1.toml#/thresholds/sandbox_pressure_ratio_max=1.15 and evolve.rs max_dp_u multiplication (historical preTR16 also1.15). Correct prose to historical/provisional sandbox cap33.89kPa; physical admitted pressure limit remains UNKNOWN. Do not interpret candidate mean28.17 as a29.5kPa gate.

## Verification and handoff

The exact quantity-bound verifier exited0;349 field checks matched at printed precision. Its planted+1 KPI negative control was rejected. All numeric-bearing lines and spelled count lines were inspected; historical values remain historical. Full outputs and timings are preserved under ignored `.private/truth-deliverable-numbers/` and their digests are in the machine receipt.

Scratch cleanup uses `git worktree remove` after clean status, stash/local-only-ref/tag inspection and proof that the detached commit is remote-reachable. Cleanup passed: scratch removed, absence verified, no scratch-exclusive data/commits, and shared stash/integration refs preserved; the machine receipt records the cleanup digest. The lane gate and final push/remote-head verification are recorded in the final board DONE/GIVE for this review commit.

DOC owns all wording/trace changes. INT/EV must resolve conflicting baseline observations and missing original artifacts. No OpenFOAM, original-case writes, PR merges or force pushes were performed.
