# I018 additive CLI migration handoff

The authoritative migration inventory remains [PY_TO_RUST_PLAN.md](../../PY_TO_RUST_PLAN.md).
This receipt-linked handoff records ports 15–16; it does not replace that plan.

`bench-cfd STAGE --case PATH [--metrics-bin PATH] [--time FLOAT]` ports
`tools/bench_cfd.py` and its restricted runtime invocation. All seven stages,
image/lease checks, output budget, native metrics identity and saved-time rename,
measurement-plane identity and tetrahedral wall-patch conversion remain intact.
Receipts bind the compatibility driver SHA-256 to `runtime.py`; the Rust source
identity is recorded separately in the packet receipt. Docker execution is native.

`baseline-report --point CASE SNAPSHOT_DIR [--point CASE SNAPSHOT_DIR ...]
--out PATH` ports `analysis/baseline_report.py`. It reconciles saved time, flux,
density/gravity, positive mechanical loss and all source/execution hashes before
writing a new report exclusively. Unknown oil remains unknown, qualification
remains false and campaign admission remains closed. The extractor digest binds
the Python compatibility source; this is not the native implementation digest.

Both binaries additionally accept `--root ROOT` for isolated replay and default
to the source checkout, matching the source-relative Python roots. Production
ports invoke no Python. Python sources and existing callers remain unchanged;
there is no Python LOC reduction or consumer cutover in this packet.

`bench_cfd_parity` uses synthetic Bash Docker output and project-local temporary
workspaces. It checks all stage commands, exact logs and metadata, wall rewrite,
resource controls and rejection paths. Runtime wall time is volatile. For later
stages only, measured shared output bytes differ by the exact serialized byte
length of preceding timing receipts; the test asserts that delta before comparing
receipts. It never treats an arbitrary output-size mismatch as timing noise.
`baseline_report_parity` compares stdout bytes exactly and report JSON values with
only `created_utc` removed; JSON object ordering is not an output-byte promise.
It covers repeated points, backflow/PDR reasons, extension and immutable output,
plus tampered/missing provenance and invalid duty/flux/energy controls.

Native usage, malformed-JSON and OS diagnostic wording can differ. The runtime
monitor retains the Python ten-second disk-check interval and 3600-second stage
budget (300 seconds for the post-tet mesh check). Long timeout/output-growth
termination and real OpenFOAM execution are not exercised by these bounded
synthetic tests. Software parity is not physical validation. Full Foundation CI,
review of the pushed commit and later consumer migration remain dependencies.
Exact commands, failures, hashes and results are in
[the I018 receipt](../receipts/i018-rust-ci-20261019.json).
