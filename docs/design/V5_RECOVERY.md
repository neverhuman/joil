# V5 fixture recovery contract and narrow scope correction

V5-07 depends on accepted runtime `423f80b` and accepted receipts `a8975ae`. The original scope omitted the module exports, database snapshot/pointer API, crate registration and design contract needed by its named entry points. The integrator adds exactly runtime lib/db/Cargo, workspace Cargo/lock and this document to V5-07. No V5-01 source path is transferred.

This is a bounded **synthetic integer software fixture**. Two-dimensional compiled integer seeds and addressed mutations produce sum-of-squares observations in real owned native fixture children. They exercise orchestration, not physical simulation, general genome search, validation or statistical qualification. Operator covariance, innovations/species, RNG counters, all archives, planned/in-flight and pending work, acquisition allocation, stop state and complete canonical ledger/backup are mandatory checkpoint state. State schemas have no defaults for omitted state. The covariance drives mutations and is updated by accepted epochs.

Private Store publication precedes the transactional SQLite history pointer. At least two roots are pinned and kept; verification follows complete manifest/dependency closure and a byte-bound SQLite backup. Pointer generations order recovery; mtimes do not. A corrupt latest root produces an explicit fallback diagnostic. Publication test hooks support actual owned SIGKILL before/after object rename and pointer commit; device power loss is not experimentally claimed.

SQLite remains the single authoritative resource/family ledger across decision forks. Resume never replaces it with an older backup or creates a missing ledger. Its event prefix, charges and family attempts must extend the checkpoint. Immutable solver/material/interface/requirement/config/recipe/executable identities must match. Fork changes preferences and decision lineage, shares observations and the same ledger, and recomputes decisions. Seed import compiles only bounded integer values and discards inherited verdicts/identity. Statistical family spending is never inferred from toy scores.

Real process costs and throughput arrival order are separate from logical epoch state. Due epoch and ordinal determine application; campaign/island/epoch/proposal/operator/draw determine RNG. Different worker counts may change actual costs without changing logical state. Physical requests remain unsupported.

Recovery reconciles existing committed charged attempts before dispatch. A crash that loses an active attempt's wait4 accounting cannot be silently charged zero or labelled exact: the recorded owned fixture may be cancelled through its PID/start/boot identity, but its reservation is retained and redispatch fails closed with `crash_accounting_unresolved`. Automatic resume is supported for checkpoints with planned work and for accepted charged results after the checkpoint. Unknown crash cost requires explicit operator accounting reconciliation, outside this lease; it does not prevent offline inspection/verification. Bounded SIGINT normally lets the current <=30 second fixture finish, records actual costs, checkpoints and stops; a forced kill during unaccounted native work has the stated safe halt.

CLI commands are admit/run/status/stop/checkpoint verify/fork/seed-import. Invalid input is rejected before run-root creation or jobs. Fresh roots are private and unique; no auto-install, fleet operations or GPU. The wrapper uses the existing toolchain only. The fixture run is constrained to two physical cores selected from its inherited operator affinity, <=512 MiB aggregate child address space and <=64 MiB aggregate active child scratch. The outer user lease remains authoritative. Raw state/process identifiers stay private.

Acceptance requires a restored nonempty mid-epoch state, missing-field controls, independent score arithmetic, actual checkpoint-boundary kills/fallback, identity drift refusal, fork spending retention, seed verdict removal, shuffled/worker-count logical equality with distinct actual-cost digests, CLI lifecycle/stop/kill recovery and fail-closed lost-accounting behavior. TR-16 and EV-09/EV-12 remain deferred. Unassigned V5-04 means this does not complete all M1 statistics or the broader physical release.

Integrator operational correction: the existing required runner forced four Cargo/Rayon threads despite the two-core coding allocation. V5-07 supporting scope adds tools/check.py, tests/test_check.py and foundation.yml solely for Cargo jobs2/numerical1, a control observing the actual child environment, and complete locked-graph prefetch before unchanged offline architecture controls. This does not relax a gate or change model defaults.


Run using the existing toolchain from the checkout (all outputs are ignored private data):

```sh
rtk cargo run --locked -p crucible-cli -- run campaigns/fixture-v5.toml --until 1
# Use the printed ROOT; the same config and executable are required.
rtk cargo run --locked -p crucible-cli -- run campaigns/fixture-v5.toml --resume ROOT
rtk cargo run --locked -p crucible-cli -- checkpoint verify ROOT
rtk cargo run --locked -p crucible-cli -- status ROOT
rtk cargo run --locked -p crucible-cli -- stop ROOT
# Explicit continuation consumes a prior stop request without deleting its history.
rtk cargo run --locked -p crucible-cli -- run campaigns/fixture-v5.toml --resume ROOT --continue
rtk cargo run --locked -p crucible-cli -- fork ROOT --weights -1,1
rtk cargo run --locked -p crucible-cli -- seed-import SEED_JSON
```

The executable must match its frozen bytes; rebuilding changed code requires a new experiment. Worker count and operational fault injection are outside logical observation identity. Decision weights fork a lineage. The fixture's operator is a bounded correlated integer transform, not a Gaussian sampler or an implementation of physical genome evolution. Native attempt accounting uses the V5-06 declared wait4/thread-CPU measurement scope; checkpoint/admission overhead and shared coordinator memory are not included in that attempt-cost digest. The outer process lease still bounds all software work. Offline verification checks closure and a real SQLite backup; it does not recreate missing current consumed-budget history.

CI follow-up scope correction: V5-07 also owns runtime/task.rs solely to separate native execution timeout from bounded, charged executable-verification/preparation time. The prior timer began before preparation and could cancel a healthy child immediately on a slower build/host; the absolute resource lease and all measured CPU/wall charges remain. A deterministic trusted single-process fixture control delays preparation, then verifies success within the native deadline and rejection of an actual slow child. This is a software deadline definition, with no physical threshold change. Required-runner profile defaults now follow AGENTS (incremental0, line-tables-only unless explicitly set). CI with fewer than two physical cores must prove refusal of unavailable worker2 and exercise only its permitted count; leased local controls still exercise1/2.

Fresh-clone acceptance uses `tools/v5_fixture_smoke.py` against an isolated clone with its own Git objects and a clean detached working tree. It records the unchanged shipped two-worker command, a one-epoch checkpoint/resume to epoch2, offline verification, exact charge counts, logical replay equality, and invalid physical-domain refusal before root creation. `worker-capability.json` is retained privately. The shipped `workers=2` configuration requires two granted physical cores and is refused on a smaller allocation; this witness never substitutes a one-worker configuration. CI lifecycle controls may exercise a one-core refusal path, so only the leased local two-core witness establishes actual worker1/worker2 coverage. The per-worker toolchain/build cache is reused and declared; no clean-cache or dependency-install claim is made.
