# Local validation

Read MASTER_PLAN.md, SCIENCE.md, WORK_PACKETS.json and STATUS.json before work.
Install Rust 1.95.0 with rustfmt/clippy, Node 22.22.0, npm, Python 3.11+ and just.
Python is transitional: see ../PYTHON_EXCEPTIONS.md and ../PY_TO_RUST_PLAN.md.
For plotting tests create a project-local virtual environment and install
matplotlib==3.10.8; keep pip and Matplotlib caches under .private/.

`just fast` runs the Rust publication guard and deterministic plan/explorer checks.
`just required` runs all eight Foundation suites through tools/check.py, with
exact commands, exit codes, source hashes and log hashes under .private/checks/.
`python3 tools/check.py --suite NAME` selects fmt, clippy, rust-tests, baseline,
python-tests, public-check, claims-audit or ui. UI includes npm ci and build.
`just portability` executes the architecture/interchange controls with rtk absent.
`just score` emits Jankurai JSON and Markdown in ignored .jankurai/.
A nonzero exit is failure; missing prerequisites are never passing evidence.
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
