# V5 benchmark protocol (software contract; measurements pending)

This protocol defines the record needed before selecting worker packing or claiming useful throughput. No results are supplied by this document. `tools/bench_cfd.py` and the `scene.rs` placeholder evaluator are not qualified benchmark producers. Synthetic workers can test accounting and rejection behavior only.

## Fixed cohort and seven runs

Freeze and hash one corpus manifest before a comparison. Each record binds the exact corpus digest, software/solver identity, resource class, cache state, and complete attempt identity. Keep every timeout, numerical failure, infrastructure failure, and rejected decision in the cohort. `BENCH-01..07` are reserved for (01) baseline cold independent jobs, (02) same jobs warm, (03) measured artifact/cache reuse, (04) binary I/O end-to-end, (05) warm-start with invariant comparison, (06) permitted rank/thread packing after independent jobs, and (07) coordinator recovery/replay. Do not infer measurements from historical host inventory or peak bandwidth.

For every attempt record wall time, peak RSS, disk bytes, CPU/GPU and I/O charges, cache state, resource class, solver/model identity, numerical acceptance, and evidence digest. Report p50 and nearest-rank p95 for wall/RSS/disk. Report failed-attempt count and unique decisions that pass numerical quality and have bound physical evidence. A repeated/retried decision counts once; failures remain in denominator and ledger. Warm and cold cohorts remain separate.

Qualified throughput is unique qualified decisions divided by summed wall seconds for the full attempts in the cohort. It is absent when no decision qualifies. VRAM occupancy, raw cell updates, iteration rate, the fake `scene.rs` evaluator, and synthetic fixtures are diagnostics only and never qualify. A result with missing physical evidence, failed numerical checks, or synthetic evaluator has zero qualified decisions.

## Decision order and stop conditions

Measure independent jobs first. Compare only explicitly leased rank/thread packs and resource vectors. Consider MPI strong scaling only after single-job cost/yield is measured. A proposed cache, binary I/O path, or warm start must preserve exact decision identity and all numerical outputs within predeclared tolerances, including failed attempts and required outputs. Pruned output, changed warm-start ranking, or omitted expensive failures invalidates the comparison.

Use complete workflow cost per qualified unique decision, with resource class and uncertainty, to choose packing. No candidate configuration is selected until repeated representative measurements and numerical-quality evidence exist. No CFD campaign is authorized by these software interfaces.
