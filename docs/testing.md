# Local validation

Read MASTER_PLAN.md, SCIENCE.md, WORK_PACKETS.json and STATUS.json before work.
Install Rust 1.95.0 with rustfmt/clippy, Node 22.22.0, npm, Python 3.11+ and just.
Python is transitional: see ../PYTHON_EXCEPTIONS.md and ../PY_TO_RUST_PLAN.md.
For plotting tests create a project-local virtual environment and install
matplotlib==3.10.8; keep pip and Matplotlib caches under .private/.

`just fast` runs the Rust publication guard and deterministic plan/explorer checks.
`just required` runs all eight Foundation suites through scripts/ci-local.sh and
ops/ci/foundation.sh (currently the Python compatibility runner), with
exact commands, exit codes, source hashes and log hashes under .private/checks/.
`python3 tools/check.py --suite NAME` selects fmt, clippy, rust-tests, baseline,
python-tests, public-check, claims-audit or ui. UI includes npm ci and build.
`just portability` executes the architecture/interchange controls with rtk absent.
`just score` emits Jankurai JSON and Markdown in ignored .jankurai/.
A nonzero exit is failure; missing prerequisites are never passing evidence.
`scripts/ci-doctor.sh` checks local prerequisites. `just security` runs the Rust
publication scan, cargo-audit 0.22.1 and npm production-dependency audit; the CI
Security workflow provisions the scanner project-locally. Missing scanners fail.
The new `check-foundation` binary must run from the repository root; automated
Rust/Python receipt parity lives in crucible-cli/tests/foundation_parity.rs.
Optional local pre-push enforcement: `git config core.hooksPath ops/git-hooks`.

Reuse this checkout's target/ cache. Default two Cargo jobs, one Rayon/BLAS thread;
no simulation or GPU is part of these checks. Honor the live resource lease;
stop owned work on memory/disk pressure or a scope conflict. Bootstrap output
budget is 5 GiB, memory 12 GiB, at most four physical cores. Raising these limits
requires coordination. No paid operation or automatic resource expansion.

Receipts prove software checks only. Keep failed logs and report input revision,
changed paths, exact invocation, exit status and SHA-256 in the packet handoff.
Do not rewrite historical receipts to match a new implementation. Negative tests
must reject malformed, stale, missing and out-of-envelope evidence. Physical
validation still requires the independent gates in SCIENCE.md.

## Observability and repair evidence

The Foundation runner records each command, cwd, elapsed seconds, exit code,
stdout/stderr log bytes and SHA-256 in `.private/checks/`; its receipt binds the
input Git revision and selected source hashes. Preserve these logs on failure.
The packet handoff names the failure, owning component, reproduction command,
expected result, actual result and next dependency. Never turn absent evidence
into PASS or suppress a failure to improve an audit score.

For repair, reproduce the failure under the same bounded lease, add a negative
control at the affected boundary, implement the smallest scoped correction, then
run the affected tests and the required gate. Link the original failed log hash,
repair commit and successful receipt. For status import, test stale digest,
malformed evidence and attempted scientific promotion; for baseline inventory,
test missing records, tampered inventory and frozen-source identity mismatch.
A parser-specific diagnostic may differ while both implementations fail closed.

## Cost and spend-risk budget proof

Local CI is bounded software verification: no paid API, cloud provisioning,
simulation or GPU is authorized. The spending budget is zero paid operations.
Use `CARGO_BUILD_JOBS=2`, `RUST_TEST_THREADS=2`, at most four physical cores,
12 GiB RAM and 5 GiB outputs per allocated lane. Reuse `target/`; record its
size and retained evidence size before and after checks (`du -sh target`).
Record elapsed time and command exits in receipts; these are measured execution
cost evidence, not a monetary estimate or proof of kernel-enforced memory caps.

The worker monitors output growth and memory pressure; stop only owned work if
the lease expires or the budget is exceeded. Preserve receipts before cleanup.
An estimate exceeding the lease requires resource coordination before execution;
there is no automatic retry with a larger budget. Foundation suite execution has
a 600-second command timeout; a timeout is failed evidence requiring diagnosis.
Any paid service or expansion requires a new explicit user allocation. The
integrator reviews actual usage, failures and outstanding resource limits in the
packet receipt before approving further work.
