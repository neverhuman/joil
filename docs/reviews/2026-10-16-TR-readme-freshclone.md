# TR README fresh-clone verification — 16 October 2026

All **15 literal quick-start commands**, including the optional five-seed Wave-2 study and all three optional CAD commands, exited **0**. The two fresh-clone gate runs passed **10/10** and **11/11** stages respectively. No README source was edited.

The README needs three corrections: provide Python installation commands, use `--mc 72` for a study with complete objective vectors, and remove the obsolete statement that `wave3-proof` cannot run. The exact proposed patch below is handed to DOC. No mismatch was found in the historical KPI scores or operator transition count printed by the exercised commands.

## Checkout and environment

- Tested remote: `https://github.com/neverhuman/joil`, detached `origin/feat/joil-v4-truth-evidence` at `cb31ba338dc6284fe4c481ac0c550ff756e8ef87`.
- Tested README SHA-256: `7151e3fa0cc55170589a31306ac513e83299f8f2902394e7c42f02d96b0a6d73`. This is a pinned snapshot, not a claim about later README revisions.
- User-requested disposable clone: `/tmp/joil-freshclone-tr`. The explicit task overrides the standing no-clones-in-`/tmp` rule for this verification. The clone was removed after evidence preservation and the Git audit below.
- System prerequisites were already installed: Rust/Cargo **1.95.0**, Python **3.12.3**, Node **26.1.0**, npm **11.13.0**, RTK **0.40.0**. This verifies a fresh source/dependency environment on an existing Linux toolchain, not operating-system installation on an empty machine.
- The clone started without a target directory, virtual environments or node_modules. Cargo registry/target, pip/npm caches, Matplotlib config, temporary files and XDG caches/data were project-local and fresh. The installed Rust/Node/Python executables were reused; no global package installation or toolchain configuration change was made.
- Base Python environment: `.private/readme-python`, with user site-packages disabled. A negative import before installation failed with `ModuleNotFoundError: No module named 'numpy'`. Installing the README's required `numpy matplotlib` succeeded, resolving **NumPy 2.5.4** and **Matplotlib 3.11.2**. The runner put that environment's bin directory first in PATH, equivalent to activating it. Optional CAD used the separate `.venv` created by its literal README command and the committed requirements lock.
- Resource environment: `CARGO_BUILD_JOBS=4`, `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=line-tables-only`, `RAYON_NUM_THREADS=4`, `OMP_NUM_THREADS=1`, `OPENBLAS_NUM_THREADS=1`. No CFD solver or GPU computation was launched.
- README commands were copied verbatim and executed sequentially in a noninteractive Bash shell. No `--mc`, filter, output path or failure workaround was silently added to the literal runs. Setup and supplemental patch checks are separately labelled below.

## Command transcript

Commands #1, #38 and #39 ran at the TR lane root. All other commands ran at `/tmp/joil-freshclone-tr`. Each measurement is elapsed wall time for the displayed shell command, including its child processes. The README's compound npm command is measured as the single displayed command. Exit 1 on the deliberate prerequisite negative probe is expected; it is not a quick-start failure.

| # | Phase | Exact command | Exit | Wall seconds |
|---|---|---|---:|---:|
| 1 | checkout | `git clone https://github.com/neverhuman/joil /tmp/joil-freshclone-tr` | 0 | 4.404 |
| 2 | checkout | `git checkout origin/feat/joil-v4-truth-evidence` | 0 | 0.091 |
| 3 | environment | `git rev-parse HEAD` | 0 | 0.004 |
| 4 | environment | `rustc --version` | 0 | 0.021 |
| 5 | environment | `cargo --version` | 0 | 0.014 |
| 6 | environment | `python3 --version` | 0 | 0.003 |
| 7 | environment | `node --version` | 0 | 0.007 |
| 8 | environment | `npm --version` | 0 | 0.103 |
| 9 | environment | `rtk --version` | 0 | 0.005 |
| 10 | environment | `python3 -m venv .private/readme-python` | 0 | 2.557 |
| 11 | prerequisite-negative | `python3 -c "import numpy, matplotlib"` | 1 | 0.012 |
| 12 | missing-install-step | `python3 -m pip install numpy matplotlib` | 0 | 9.424 |
| 13 | environment | `python3 -m pip freeze` | 0 | 0.285 |
| 14 | quick-start-1 | `rtk cargo test --workspace --locked` | 0 | 82.693 |
| 15 | quick-start-1 | `rtk cargo test -p joil-core --test regression closed_catalog_validity_11family_sampling -- --nocapture --exact` | 0 | 27.642 |
| 16 | quick-start-1 | `rtk cargo test -p joil-core --test regression operator_chain_validity_20step_11family -- --nocapture --exact` | 0 | 27.648 |
| 17 | quick-start-1 | `rtk cargo run --locked -p joil-cli -- baseline` | 0 | 0.640 |
| 18 | quick-start-1 | `rtk proxy python3 analysis/kpi_fitness.py --wave3-results docs/results/wave3-evolution-proof-v1.json` | 0 | 0.104 |
| 19 | quick-start-1 | `rtk proxy python3 tools/run_elite_3grid_gci_and_rsm.py --verify-existing` | 0 | 0.091 |
| 20 | quick-start-1 | `rtk proxy python3 -m unittest discover -s tests -v` | 0 | 40.492 |
| 21 | quick-start-1 | `rtk proxy python3 tools/check_public.py` | 0 | 0.128 |
| 22 | quick-start-1 | `rtk proxy python3 tools/claims_audit.py` | 0 | 12.070 |
| 23 | quick-start-1 | `rtk proxy sh -c 'cd apps/explorer && npm ci && npm run build'` | 0 | 4.752 |
| 24 | quick-start-1 | `rtk proxy python3 tools/check.py` | 0 | 145.748 |
| 25 | quick-start-optional | `RAYON_NUM_THREADS=4 rtk cargo run --release --locked -p joil-cli -- wave2-benchmark --seeds 20261101,20261102,20261103,20261104,20261105 --out artifacts/wave2-rerun.json` | 0 | 72.207 |
| 26 | quick-start-optional | `rtk proxy python3 -m venv .venv` | 0 | 4.139 |
| 27 | quick-start-optional | `rtk proxy .venv/bin/pip install -r requirements-cad.txt` | 0 | 62.768 |
| 28 | quick-start-optional | `rtk proxy python3 tools/check.py --cad` | 0 | 144.719 |
| 29 | unfiltered-audit | `rtk proxy cargo test -p joil-core --test regression operator_chain_validity_20step_11family -- --nocapture --exact` | 0 | 28.145 |
| 30 | readme-patch-validation | `RAYON_NUM_THREADS=4 rtk cargo run --release --locked -p joil-cli -- wave2-benchmark --seeds 20261101,20261102,20261103,20261104,20261105 --mc 72 --out artifacts/wave2-rerun-mc72.json` | 0 | 41.068 |
| 31 | stale-limitation-check | `RAYON_NUM_THREADS=4 rtk cargo run --release --locked -p joil-cli -- wave3-proof --seeds 20261101,20261102,20261103,20261104,20261105 --generations 0 --pop 2 --mc 72 --out artifacts/wave3-readme-smoke.json` | 0 | 1.289 |
| 32 | cleanup-audit | `git status --porcelain=v1 --untracked-files=all` | 0 | 0.014 |
| 33 | cleanup-audit | `git status --short --ignored` | 0 | 0.104 |
| 34 | cleanup-audit | `git stash list` | 0 | 0.004 |
| 35 | cleanup-audit | `git log --all --not --remotes --oneline` | 0 | 0.006 |
| 36 | cleanup-audit | `git for-each-ref --format='%(objectname) %(refname)' refs/tags` | 0 | 0.004 |
| 37 | cleanup-audit | `git ls-remote --tags origin` | 0 | 0.322 |
| 38 | cleanup | `python3 -c "from pathlib import Path; import shutil; p=Path('/tmp/joil-freshclone-tr'); shutil.rmtree(p); assert not p.exists()"` | 0 | 1.927 |
| 39 | patch-syntax-check | `git apply --check .private/readme-freshclone/README.proposed.patch` | 0 | 0.005 |

## Observed results and corrections

1. **Installation is implicit.** README lists the Python packages as requirements but supplies no base environment/install command. The clean negative import and successful project-local install are recorded above. The proposed bootstrap uses `.private/quickstart-python`; the exercised equivalent used `.private/readme-python` so it remained separate from optional CAD's `.venv`.
2. **Literal quick starts pass.** Workspace Rust tests reported 97 passes across 26 suites. Both named regression filters actually ran one test, rather than silently matching zero. The NumPy/Matplotlib requirements were sufficient for the Python suite, public check, claims audit, explorer build and full gate. The CAD lock installed and baseline CAD gate passed on Python 3.12.
3. **Historical printed values agree.** KPI verification reported `passed=true`, 10 seeds, 50 verified seed runs, baseline KPI `26.486473`, promoted KPI `47.745321`, and the exact README Wave-3 source digest `9a21229bd1c6762e21970a22c1bf2c9dd75e4a5540353f5b5bd20f6c4bb35168`. Qualified vector/capacity values stayed null. RTK compresses the walk test's captured output, so a supplemental `rtk proxy cargo test` exposed its original summary:

```text
OPERATOR_RANDOM_WALK_SUMMARY: total_steps=220 total_passed=220 pass_fraction=1 transitions=85 global_families=11 min_clearance_mm=1.0000
```

The 220 passing steps and 85 transitions match the README. The walk's printed clearance is not the sampling audit's historical 1.24 mm claim. The sampling test asserts 220 passes using seed 20261115; the prose's historical sampling call uses 20261116, so the filtered test is not an independent reproduction of every historical sampling metric.

4. **Default Wave-2 objectives are insufficient.** The exact optional command exits 0 and writes its ignored artifact, but its 15 saved best candidates (three arms × five seeds) and baseline all have `mc_robustness.samples=12` and `objectives.status="insufficient"` for `cout_p95`, `grade_20um_p05` and `d50_p95`. Each saved best candidate is unranked (`usize::MAX`), and all 15 reported Pareto hypervolumes are zero. This follows the repaired quantile policy; it is not numerical proof of physical infeasibility. A separate run of the proposed `--mc 72` command supplies a complete sandbox objective vector, as recorded below. The two reruns use distinct ignored outputs and preserve the committed historical receipts.
5. **The Wave-3 failure bullet is stale.** A separate five-seed, generation-zero, population-two `wave3-proof --mc 72` smoke exits 0 on this same source. It uses the repaired sandbox prior surrogate; this establishes command compatibility, not scientific performance or independent validation. The obsolete physical-record fitting error is no longer a reason to block a sandbox rerun.

Artifact `wave2-rerun.json`: SHA-256 `419209d960569b94bc29b8a410201719e05a5fc92cc8bc7411ea96cf1c431f9a`, 18,085,279 bytes.

- `arm_b1_uniform_random_outcomes`: samples [12], objective status ['insufficient'], reported HV values [0.0, 0.0, 0.0, 0.0, 0.0].
- `arm_b2_conventional_axisym_outcomes`: samples [12], objective status ['insufficient'], reported HV values [0.0, 0.0, 0.0, 0.0, 0.0].
- `arm_b3_exotic3d_qd_cei_outcomes`: samples [12], objective status ['insufficient'], reported HV values [0.0, 0.0, 0.0, 0.0, 0.0].

Artifact `wave2-rerun-mc72.json`: SHA-256 `da892f710c7ca24258222696212381dde6077275916e19884e9aebd167668e6c`, 18,729,036 bytes.

- `arm_b1_uniform_random_outcomes`: samples [72], objective status ['sandbox_estimate'], reported HV values [0.4100232982445309, 0.4031328838099006, 0.404685600862885, 0.4077164295780898, 0.4087249854849397].
- `arm_b2_conventional_axisym_outcomes`: samples [72], objective status ['sandbox_estimate'], reported HV values [0.3101951972489948, 0.31599137047907394, 0.3130847730288266, 0.30702243142161295, 0.3037233450434722].
- `arm_b3_exotic3d_qd_cei_outcomes`: samples [72], objective status ['sandbox_estimate'], reported HV values [0.42489265027900724, 0.43046492822610677, 0.4267654131800741, 0.42725620251561947, 0.4214886204285908].

Artifact `wave3-readme-smoke.json`: SHA-256 `cf98876588db7ea2b26f09d6e3f65a7068101af78d45c07dea15b185df44a257`, 450,109 bytes.

## Exact README patch for DOC

Apply against the tested README snapshot. README.md remains DOC-owned. After applying, DOC must update `docs/NUMBERS_TRACE.md`/any scoped declarations and rerun the claims audit; this report does not edit either file. The supplemental study uses a separate output filename to keep the original run available for comparison.

````diff
--- a/README.md
+++ b/README.md
@@ -105,6 +105,14 @@
 
 Requirements: Rust (stable, `cargo`), Python 3 with `numpy` and `matplotlib`, and Node.js `>=22.12.0` for the explorer (`apps/explorer/package.json`). Omit `rtk` / `rtk proxy` if `rtk` is not installed.
 
+From the repository root, install the Python requirements in an isolated environment (use Python 3.12 if you also want exact CAD):
+
+```bash
+python3 -m venv .private/quickstart-python
+. .private/quickstart-python/bin/activate
+python3 -m pip install numpy matplotlib
+```
+
 **1. Check the repository.** Every command below is fast and writes only to ignored paths. Workplan item DOC-05 runs this list from a clean checkout; the log is kept in ignored `.private/checks/`.
 
 ```bash
@@ -121,15 +129,15 @@
 rtk proxy python3 tools/check.py
 ```
 
-**2. Re-run a screen study (optional, slow).** Always pass `--out` into the ignored `artifacts/` directory. The CLI defaults overwrite the committed, hash-bound receipts in `docs/results/`, and on repaired code the regenerated numbers differ from those historical receipts.
+**2. Re-run a screen study (optional, slow).** Always pass `--out` into the ignored `artifacts/` directory. The CLI defaults overwrite the committed, hash-bound receipts in `docs/results/`, and on repaired code the regenerated numbers differ from those historical receipts. The explicit `--mc 72` meets the current equal-tailed interval policy for the p05/p95 objectives. The default of 12 samples leaves these objectives insufficient and supplies no Pareto-rank signal.
 
 ```bash
-RAYON_NUM_THREADS=4 rtk cargo run --release --locked -p joil-cli -- wave2-benchmark --seeds 20261101,20261102,20261103,20261104,20261105 --out artifacts/wave2-rerun.json
+RAYON_NUM_THREADS=4 rtk cargo run --release --locked -p joil-cli -- wave2-benchmark --seeds 20261101,20261102,20261103,20261104,20261105 --mc 72 --out artifacts/wave2-rerun.json
 ```
 
 Known limitations of re-runs on the current code:
 
-- `wave3-proof` stops with "at least 4 valid physical records required to fit GP surrogate". The surrogate now refuses screen-derived labels (EV-09), and how to re-run Wave-3 is an open decision in workplan item INT-04.
+- The repaired `wave3-proof` runs as a sandbox study using a sandbox prior surrogate; its results remain screen diagnostics. Always give it a separate `--out artifacts/...` path. A small fresh-clone run is recorded in [the README verification transcript](docs/reviews/2026-10-16-TR-readme-freshclone.md).
 - `export-3d-preview` reads ignored campaign files under `runs/`, so it cannot run from a fresh checkout.
 - `tools/generate_wave3_proof_figures.py` rewrites the committed, hash-bound figures, so run it only when regenerating figures together with a new receipt.
 
````

## Full-gate subcommands

These are the child command measurements from the fresh clone's own check receipts, retained before cleanup. They supplement the outer command wall times above.

Receipt `20261016T071136Z-88509267`: all requested pass `True`, source `cb31ba338dc6284fe4c481ac0c550ff756e8ef87`, SHA-256 `6f869ae1cfc966753021d9650b13a5dde919270ecb676e2bdc60310b36c1081b`.

| Stage | Exact argv | Exit | Wall seconds |
|---|---|---:|---:|
| generated-status | `["python3", "tools/gen_status.py", "--check"]` | 0 | 0.044 |
| fmt | `["cargo", "fmt", "--all", "--", "--check"]` | 0 | 0.310 |
| clippy | `["cargo", "clippy", "--workspace", "--locked", "--all-targets", "--", "-D", "warnings"]` | 0 | 9.800 |
| rust-tests | `["cargo", "test", "--workspace", "--locked"]` | 0 | 70.768 |
| baseline | `["cargo", "run", "--locked", "-p", "joil-cli", "--", "baseline"]` | 0 | 0.047 |
| python-tests | `["python3", "-m", "unittest", "discover", "-s", "tests", "-v"]` | 0 | 44.000 |
| public-check | `["python3", "tools/check_public.py"]` | 0 | 0.190 |
| claims-audit | `["python3", "tools/claims_audit.py"]` | 0 | 15.594 |
| npm-ci | `["npm", "ci", "--no-audit", "--no-fund"]` | 0 | 1.457 |
| ui-build | `["npm", "run", "build"]` | 0 | 3.263 |

Receipt `20261016T071626Z-f926aabd`: all requested pass `True`, source `cb31ba338dc6284fe4c481ac0c550ff756e8ef87`, SHA-256 `6556d12276ab6f5a63671c25c6b9db17310a42c26496b42eea6e3a08ab209ce4`.

| Stage | Exact argv | Exit | Wall seconds |
|---|---|---:|---:|
| generated-status | `["python3", "tools/gen_status.py", "--check"]` | 0 | 0.054 |
| fmt | `["cargo", "fmt", "--all", "--", "--check"]` | 0 | 0.330 |
| clippy | `["cargo", "clippy", "--workspace", "--locked", "--all-targets", "--", "-D", "warnings"]` | 0 | 0.583 |
| rust-tests | `["cargo", "test", "--workspace", "--locked"]` | 0 | 74.123 |
| baseline | `["cargo", "run", "--locked", "-p", "joil-cli", "--", "baseline"]` | 0 | 0.091 |
| python-tests | `["python3", "-m", "unittest", "discover", "-s", "tests", "-v"]` | 0 | 42.909 |
| public-check | `["python3", "tools/check_public.py"]` | 0 | 0.072 |
| claims-audit | `["python3", "tools/claims_audit.py"]` | 0 | 12.534 |
| npm-ci | `["npm", "ci", "--no-audit", "--no-fund"]` | 0 | 1.103 |
| ui-build | `["npm", "run", "build"]` | 0 | 2.342 |
| cad | `["/tmp/joil-freshclone-tr/.venv/bin/python", "cad/ct35/build.py", "--out", "/tmp/joil-freshclone-tr/artifacts/20261016T071626Z-f926aabd/ct35"]` | 0 | 10.349 |

## Evidence retention and cleanup

Raw command logs and a machine-readable transcript are retained in the TR worktree's ignored `.private/readme-freshclone/`. Logs were stored outside the disposable clone from the start. Gate receipts/logs, generated study/CAD artifacts and the non-build-cache target evidence were copied out before deletion; `saved-artifacts.json` lists their original paths, sizes and SHA-256 digests. These private artifacts are not staged as public Git content.

Before deletion, the recorded Git audit checked status (including ignored paths), stashes, local commits not reachable from remotes, and local/remote tags. There were no source edits, untracked nonignored files, stashes, local-only commits or local-only tags to save. Generated artifacts were preserved. The recorded cleanup command removed only `/tmp/joil-freshclone-tr` and asserted its absence. No other worktree, shared cache or project was removed.

## Command log integrity

Each digest covers the full combined stdout/stderr log. Clone/checkout messages and environment diagnostics are included. Timings and exit codes are the measured values above, not estimates.

| # | Log | SHA-256 |
|---|---|---|
| 1 | `01.log` | `36bed8b3571a827cf6a1753a62637165e7f7ebfd9d162000d64b36827a6d4e51` |
| 2 | `02.log` | `cd29110906bc21595c93876b61f6315a159e58de15955a250085272a26f93ba4` |
| 3 | `03.log` | `8cc28fa3454f3d7cd917f3cc3c8d86bf82eda628a46d713e5d1720524d05147b` |
| 4 | `04.log` | `f4d034f5660bb49f22bfcba3f3445be67e25b704c193ff421b53863b4faee148` |
| 5 | `05.log` | `c10ec31b8c6e6e2693cf65fc1971b41edbc3da1ae7db6f9f3f36c4823f8dcab5` |
| 6 | `06.log` | `5b3e43dc38ca01e3f5a7854ba50d4330864b0c1e0f646650b2c78cf072acc366` |
| 7 | `07.log` | `2ca995cc259b7733f154600fc48973f1dcd71a53749f37176548c8516beeda94` |
| 8 | `08.log` | `3dbc807e13de125e909bea5877613d5d67989a43fb8e982fd79ba0efa46ac952` |
| 9 | `09.log` | `2e209881302a5cff43cd752ee0e1bebf188bd9b6012f9b46a9b0d935f9cb2d14` |
| 10 | `10.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 11 | `11.log` | `9bd3e35258d432edbe7bb225c04425d12d4a11c30f7afe771405287fdfb61b82` |
| 12 | `12.log` | `a649754a0c708ad264819877154672d90e7463f605c187a14540b8ed098056a9` |
| 13 | `13.log` | `0e68a707d57fdecb6bd7f80a497279d84d71001a3ec7d1bcc82f9cce4fd68095` |
| 14 | `14.log` | `6c944d6e2ba9ea821fa4d3e76ef4866e1df2b0d110986778ad5138c60f739917` |
| 15 | `15.log` | `38a643266dc7a6fab30a7ec4a62e440bc8219fb950d536e3a7b18350e2ba9e98` |
| 16 | `16.log` | `e10195015f29bbd56f4b1c950e9b8d5b8edb13fa91f97ef035b99a503a3c9bab` |
| 17 | `17.log` | `4c16e2b763204e84d0c9540ded3cdb8d7873c62d52d31b5a56bf942d09ebaecd` |
| 18 | `18.log` | `a499e1759dbfbdac0933b51aeed9af845fc1f58d8b56487ba3e92ce948bc99a6` |
| 19 | `19.log` | `6219e364acbaeb600e76303d1931730021f04c41b6a221569224532f9accd23b` |
| 20 | `20.log` | `2bbc10cbe014d89625e41d16e015becf971651697cda9d14c9c8e99ef11b9f68` |
| 21 | `21.log` | `2c9ec7fb347068d88cfae4a885fba6e87137f416120c0fbc316752c5dced0c32` |
| 22 | `22.log` | `40e5f46f36e9f0dbe4f0cf0967e9642e77fec3efce8c741c06253e40af35b77a` |
| 23 | `23.log` | `82a95f04154144caeb46772791a410f4390e3c15d8f1ce1d10061d79aee2993a` |
| 24 | `24.log` | `a1e959a381c753da9d107e54579d6d57c309702fb3dac22362a7ffe197f23a9f` |
| 25 | `25.log` | `cfb02ec80fa0e63d2d39bf9de419b76657e0a8903adb53bfc248959a4552cc14` |
| 26 | `26.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 27 | `27.log` | `9543d3936f5e5b2acd48656bab9e1f5cc91a4653c74582154145e8f2280e8b9a` |
| 28 | `28.log` | `b86490b52deebf4bb9e64e8cfc24479cc8b5f741125521f66ef0fbf6d083f46e` |
| 29 | `29.log` | `7b89cbc5cddd1e242268aba2304cdda699317a9362e8d0168f4fdcaca6d212de` |
| 30 | `30.log` | `2d981ec943081591d694b574978b564b43e52653995c5d64079e4cbd2a9aba27` |
| 31 | `31.log` | `ebc6bbca6ebc61875d3a9c894ea603168391be14792acea53eafd3858602d952` |
| 32 | `32.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 33 | `33.log` | `6d57bcba5bbbf1d8abb7c08e1522985e658bedf64e64611a29296c5e785ec189` |
| 34 | `34.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 35 | `35.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 36 | `36.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 37 | `37.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 38 | `38.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 39 | `39.log` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
