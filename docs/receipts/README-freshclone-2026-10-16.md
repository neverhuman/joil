# IR Fresh-Clone `README.md` Verification Receipt — 16 October 2026

- **Lane / Author**: `IR` (`joil_ir`)
- **Tested Integration Branch**: `origin/feat/joil-v4-truth-evidence`
- **Tested Head Commit SHA**: `82bc814b0e75a908f7446418da4f98d7a9c24283`
- **`README.md` SHA-256**: `8f6baba3c8376e25afa91548d7cd93e22c6906682a143a5f4e0085e1eedbbc83`
- **Temporary Clone Path**: `/tmp/joil-freshclone-ir` (created fresh, verified clean after execution, and deleted; `clone_removed: true`)
- **Raw Machine Transcript (ignored private storage)**: `.private/readme-freshclone-ir/commands-with-outputs.json` (`SHA-256: f87085ef9c9975e97077e201ee4a4754e1753c45b678d0fc3aec35729847774c`)
- **Command Execution Summary**: **18 / 18** literal `README.md` install + quick-start commands exited **`0`** (`0` command failures, `0` dirty/untracked non-ignored files after execution). Four text/command-output accuracy discrepancies (`README-FC-01`..`README-FC-04`) were identified between `README.md` and actual command behavior/outputs; exact line fixes for Lane `DOC` are provided in §3.

---

## 1. Environment & Isolation

1. A fresh `git clone https://github.com/neverhuman/joil /tmp/joil-freshclone-ir` was created and checked out at `origin/feat/joil-v4-truth-evidence` (`82bc814b0e75a908f7446418da4f98d7a9c24283`).
2. All `README.md` Quick start commands (`README.md:127-169`) were executed verbatim in order inside `/tmp/joil-freshclone-ir`, with `. .private/quickstart-python/bin/activate` active across subsequent shell steps and `PIP_CACHE_DIR`, `NPM_CONFIG_CACHE`, `XDG_CACHE_HOME`, `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, and `MPLCONFIGDIR` isolated inside `/tmp/joil-freshclone-ir/.private/isolated-cache/`.
3. Resource bounds enforced: `CARGO_BUILD_JOBS=4`, `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=line-tables-only`, `RUST_TEST_THREADS=4`, `RAYON_NUM_THREADS=4`, `OMP_NUM_THREADS=1`, `OPENBLAS_NUM_THREADS=1`.
4. Toolchain versions recorded inside the fresh clone:
   - `rustc 1.95.0 (59807616e 2026-04-14)` / `cargo 1.95.0 (f2d3ce0bd 2026-03-21)`
   - `Python 3.12.3` (`.private/quickstart-python`: `numpy 2.5.4`, `matplotlib 3.11.2`; `.venv`: `Python 3.12.3` with `requirements-cad.txt` lock)
   - `node v26.1.0` / `npm 11.13.0`
   - `rtk 0.40.0`

---

## 2. Step-by-Step Command Ledger (`[01]`–`[31]`)

| # | Phase | Verbatim Command | Exit | Wall (s) | Combined Output SHA-256 |
|---:|---|---|---:|---:|---|
| 01 | `checkout` | `git clone https://github.com/neverhuman/joil /tmp/joil-freshclone-ir` | `0` | `3.615` | `aea19da8381f43f183196f82f66aba3f47072319eb01cdc9b4c7bddc7b81b88d` |
| 02 | `checkout` | `git checkout origin/feat/joil-v4-truth-evidence` | `0` | `0.107` | `3d869c30b3b0639bb7da9bbbe41bfa133680447d959a2225c62f609dc98bf9f6` |
| 03 | `readme-block-1` (`README.md:128`) | `python3 -m venv .private/quickstart-python` | `0` | `2.624` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 04 | `readme-block-1` (`README.md:129`) | `. .private/quickstart-python/bin/activate` | `0` | `0.003` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 05 | `readme-block-1` (`README.md:130`) | `python3 -m pip install numpy matplotlib` | `0` | `15.363` | `fa96ebc91164ffa63231486e7070490869be07f264bec2df8a1ef666e7868753` |
| 06 | `readme-block-2` (`README.md:136`) | `rtk cargo test --workspace --locked` | `0` | `134.954` | `6d858e835c6e909c6cdcaefb3dffd41fcad3f2a1cd011d74cbf30b2d3478df9d` |
| 07 | `readme-block-2` (`README.md:137`) | `rtk cargo test -p joil-core --test regression closed_catalog_validity_11family_sampling -- --nocapture --exact` | `0` | `32.101` | `052568c4ecb9516b5a016ae24a3dae2c5e2b3d0b990d72aaf401ea3adc8aa3e9` |
| 08 | `readme-block-2` (`README.md:138`) | `rtk cargo test -p joil-core --test regression operator_chain_validity_20step_11family -- --nocapture --exact` | `0` | `31.721` | `8a0ac5d40bd8871186de6a3928e16452a3d0d9debd6aca7979a03d35e7fc8262` |
| 09 | `readme-block-2` (`README.md:139`) | `rtk cargo run --locked -p joil-cli -- baseline` | `0` | `0.853` | `e4bb5ea5e85ae92a36cb99ab9d8b4d3d483ca7e1cc416f3befb8ac07d9b27de7` |
| 10 | `readme-block-2` (`README.md:140`) | `rtk proxy python3 analysis/kpi_fitness.py --wave3-results docs/results/wave3-evolution-proof-v1.json` | `0` | `0.217` | `a499e1759dbfbdac0933b51aeed9af845fc1f58d8b56487ba3e92ce948bc99a6` |
| 11 | `readme-block-2` (`README.md:141`) | `rtk proxy python3 tools/run_elite_3grid_gci_and_rsm.py --verify-existing` | `0` | `0.211` | `6219e364acbaeb600e76303d1931730021f04c41b6a221569224532f9accd23b` |
| 12 | `readme-block-2` (`README.md:142`) | `rtk proxy python3 -m unittest discover -s tests -v` | `0` | `68.797` | `3a440d242b38e0ccefc3da22581ba47ceab7380049e6afe3d81daf4ea90c7a8f` |
| 13 | `readme-block-2` (`README.md:143`) | `rtk proxy python3 tools/check_public.py` | `0` | `0.811` | `1b4faad43f0d3884103e5103b89382c6a1a7975683fb9e2fe5156a4f7d5ef420` |
| 14 | `readme-block-2` (`README.md:144`) | `rtk proxy python3 tools/claims_audit.py` | `0` | `19.873` | `c16413580d9729676a5772ed6c28a1c7caf26a9570b8fa32d0a029c297146d98` |
| 15 | `readme-block-2` (`README.md:145`) | `rtk proxy sh -c 'cd apps/explorer && npm ci && npm run build'` | `0` | `6.252` | `4f1917efc9bb05585fd4f6afa872c0ef06101661f16321b6415fb398898791b9` |
| 16 | `readme-block-2` (`README.md:146`) | `rtk proxy python3 tools/check.py` | `0` | `214.994` | `515c0fbbb8777c4482825d041b3a8cd0418f73688d8d2961ce3c6e0ed4c0d8c6` |
| 17 | `readme-block-3` (`README.md:152`) | `RAYON_NUM_THREADS=4 rtk cargo run --release --locked -p joil-cli -- wave2-benchmark --seeds 20261101,20261102,20261103,20261104,20261105 --mc 72 --out artifacts/wave2-rerun.json` | `0` | `80.807` | `0068a1ee92aa0ec1d18219446230bd0fc222584e613eb9aedc1b90fcfa7d8409` |
| 18 | `readme-block-4` (`README.md:166`) | `rtk proxy python3 -m venv .venv` | `0` | `3.502` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 19 | `readme-block-4` (`README.md:167`) | `rtk proxy .venv/bin/pip install -r requirements-cad.txt` | `0` | `51.382` | `1a388310997c228a0ca519210e5f7dfda3d5c71a99dfc32fab1a09beab061554` |
| 20 | `readme-block-4` (`README.md:168`) | `rtk proxy python3 tools/check.py --cad` | `0` | `205.657` | `472981de958fcdac41e431351e8c71b944e37a70872edcb2378f2658802a4300` |
| 21 | `diagnostic-nocapture` | `rtk proxy cargo test -p joil-core --test regression closed_catalog_validity_11family_sampling -- --nocapture --exact` | `0` | `30.041` | `9b4fc8f9a19137eafa40d7f714ec8d4b7aad0c94f3c45ffa85ebce0bd4a808aa` |
| 22 | `diagnostic-nocapture` | `rtk proxy cargo test -p joil-core --test regression operator_chain_validity_20step_11family -- --nocapture --exact` | `0` | `30.944` | `c48af0258d50f18550ab44ba9064a76cb3af9a58e0dfd94510cab0371ce4b93a` |
| 23 | `audit` | `rustc --version` | `0` | `0.024` | `f4d034f5660bb49f22bfcba3f3445be67e25b704c193ff421b53863b4faee148` |
| 24 | `audit` | `cargo --version` | `0` | `0.014` | `c10ec31b8c6e6e2693cf65fc1971b41edbc3da1ae7db6f9f3f36c4823f8dcab5` |
| 25 | `audit` | `node --version` | `0` | `0.008` | `2ca995cc259b7733f154600fc48973f1dcd71a53749f37176548c8516beeda94` |
| 26 | `audit` | `npm --version` | `0` | `0.084` | `3dbc807e13de125e909bea5877613d5d67989a43fb8e982fd79ba0efa46ac952` |
| 27 | `audit` | `rtk --version` | `0` | `0.005` | `2e209881302a5cff43cd752ee0e1bebf188bd9b6012f9b46a9b0d935f9cb2d14` |
| 28 | `audit` | `python3 -c "import sys,numpy,matplotlib; print(sys.executable); print(sys.version.splitlines()[0]); print(numpy.__version__); print(matplotlib.__version__)"` | `0` | `0.189` | `948deb0cf914a481cad5230f8543816343f8e64d0440a11536049d31534f969b` |
| 29 | `audit` | `.venv/bin/python --version` | `0` | `0.004` | `5b3e43dc38ca01e3f5a7854ba50d4330864b0c1e0f646650b2c78cf072acc366` |
| 30 | `audit` | `git status --porcelain=v1 --untracked-files=all` | `0` | `0.009` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| 31 | `cleanup` | `python3 -c "from pathlib import Path; import shutil; p=Path('/tmp/joil-freshclone-ir'); shutil.rmtree(p); assert not p.exists(); print('removed /tmp/joil-freshclone-ir')"` | `0` | `1.972` | `d758f82ac730b43ed57ef11861f31c3940788ba9bd79478f5a9351f5b9492b3e` |

### Key Command Output Excerpts

- **Command 06 (`rtk cargo test --workspace --locked`)**:
  ```text
  cargo test: 107 passed (27 suites, 118.48s)
  ```
- **Commands 07–08 (`rtk cargo test ... -- --nocapture --exact`) vs Diagnostic Commands 21–22 (`rtk proxy cargo test ... -- --nocapture --exact`)**:
  - Command 07 (`rtk cargo test ... closed_catalog_validity_11family_sampling -- --nocapture --exact`):
    ```text
    cargo test: 1 passed, 37 filtered out (1 suite, 31.54s)
    ```
  - Command 08 (`rtk cargo test ... operator_chain_validity_20step_11family -- --nocapture --exact`):
    ```text
    cargo test: 1 passed, 37 filtered out (1 suite, 31.51s)
    ```
  - Command 22 (`rtk proxy cargo test ... operator_chain_validity_20step_11family -- --nocapture --exact`):
    ```text
    running 1 test
    OPERATOR_RANDOM_WALK_SUMMARY: total_steps=220 total_passed=220 pass_fraction=1 transitions=85 global_families=11 min_clearance_mm=1.0000
    test operator_chain_validity_20step_11family ... ok
    ```
- **Command 10 (`rtk proxy python3 analysis/kpi_fitness.py --wave3-results docs/results/wave3-evolution-proof-v1.json`)**:
  ```json
  {
    "passed": true,
    "wave3_results_file": "docs/results/wave3-evolution-proof-v1.json",
    "wave3_results_sha256": "9a21229bd1c6762e21970a22c1bf2c9dd75e4a5540353f5b5bd20f6c4bb35168",
    "n_seeds": 10,
    "verified_seed_runs": 50,
    "baseline_b0_ct35_kpi_score": 26.486473,
    "promoted_wave3_top_elite_kpi_score": 47.745321,
    "qualification_tier": "prior_laden_reduced_order_f1_screen_search_diagnostics_not_f2_or_f4_qualified",
    "qualified_multiobjective_vector": null,
    "qualified_capacity_gain": null
  }
  ```
- **Command 11 (`rtk proxy python3 tools/run_elite_3grid_gci_and_rsm.py --verify-existing`)**:
  ```text
  PASS: elite GCI+RSM gate certified for xbabe2-gen043-isl10-ind030 (SNR_dp=4.86x, SNR_eloss=6.76x, LRR-vs-SSG dp_u delta=0.256%)
  ```
- **Command 12 (`rtk proxy python3 -m unittest discover -s tests -v`)**:
  ```text
  Ran 86 tests in 65.413s
  OK (skipped=2)
  ```
- **Command 13 (`rtk proxy python3 tools/check_public.py`)**:
  ```text
  PASS: 339 public candidate files checked; private deny rules applied=False
  ```
- **Command 14 (`rtk proxy python3 tools/claims_audit.py`)**:
  ```text
  claims_audit: OK (152 structured receipt claims + 1423 markdown numeric tokens verified across WHITEPAPER.md, SCIENCE.md, README.md; 6 review docs checked for banned words; 5/5 figure hash manifests verified; 6/6 Wave-3 proof invariants verified; 6/6 Wave-4 proof invariants verified; 3/3 Wave-4 GCI control invariants verified; 4/4 Wave-2 v2 oscillatory GCI invariants verified; status synced with docs/STATUS.json)
  ```
- **Command 16 (`rtk proxy python3 tools/check.py`)**:
  ```text
  generated-status: exit 0, 0.124s
  fmt: exit 0, 0.413s
  clippy: exit 0, 12.650s
  rust-tests: exit 0, 119.266s
  baseline: exit 0, 1.549s
  python-tests: exit 0, 54.442s
  public-check: exit 0, 0.146s
  claims-audit: exit 0, 16.272s
  npm-ci: exit 0, 2.279s
  ui-build: exit 0, 6.927s
  receipt: /tmp/joil-freshclone-ir/.private/checks/20261016T111453Z-eb961e23/receipt.json
  ```
- **Command 20 (`rtk proxy python3 tools/check.py --cad`)**:
  ```text
  generated-status: exit 0, 0.042s
  fmt: exit 0, 0.318s
  clippy: exit 0, 0.165s
  rust-tests: exit 0, 118.303s
  baseline: exit 0, 1.484s
  python-tests: exit 0, 56.110s
  public-check: exit 0, 0.155s
  claims-audit: exit 0, 15.952s
  npm-ci: exit 0, 0.946s
  ui-build: exit 0, 2.963s
  cad: exit 0, 9.189s
  receipt: /tmp/joil-freshclone-ir/.private/checks/20261016T112043Z-f9038990/receipt.json
  ```

---

## 3. Discrepancies Found & Exact `README.md` Line Fixes for Lane `DOC`

While all 18 literal commands exit `0`, four discrepancies exist between `README.md` text/flags and actual runtime behavior at `82bc814b0e75a908f7446418da4f98d7a9c24283`:

### `README-FC-01` (`README.md:137-138`): `rtk cargo test` suppresses `--nocapture` stdout
- **Observation**: `README.md:137-138` passes `-- --nocapture --exact` to `rtk cargo test`, but `rtk`'s cargo-test wrapper filters out test `println!` output on passing tests (`cargo test: 1 passed, 37 filtered out (1 suite, 31.51s)`), hiding `OPERATOR_RANDOM_WALK_SUMMARY: total_steps=220 total_passed=220 pass_fraction=1 transitions=85 global_families=11 min_clearance_mm=1.0000`. Using `rtk proxy cargo test` preserves `--nocapture` stdout.
- **Exact `README.md` line fix (`README.md:137-138`)**:
  - Replace:
    ```bash
    rtk cargo test -p joil-core --test regression closed_catalog_validity_11family_sampling -- --nocapture --exact
    rtk cargo test -p joil-core --test regression operator_chain_validity_20step_11family -- --nocapture --exact
    ```
  - With:
    ```bash
    rtk proxy cargo test -p joil-core --test regression closed_catalog_validity_11family_sampling -- --nocapture --exact
    rtk proxy cargo test -p joil-core --test regression operator_chain_validity_20step_11family -- --nocapture --exact
    ```

### `README-FC-02` (`README.md:161`): Seed mismatch for `closed_catalog_validity` (`20261115` in code vs `20261116` in `README.md`)
- **Observation**: `README.md:161` states `DesignProgram::audit_11family_genome_expressiveness(20, 20261116) (since renamed closed_catalog_validity)`. In `crates/joil-core/tests/regression.rs:988` (`DesignProgram::closed_catalog_validity(20, 20261115)`) and `docs/receipts/wave2-genome-and-10seed-ablation-20261015.json`, `closed_catalog_validity` runs with seed **`20261115`** (`20261116` is the seed for `operator_chain_validity` at `regression.rs:1035`).
- **Exact `README.md` line fix (`README.md:161`)**:
  - Replace `DesignProgram::audit_11family_genome_expressiveness(20, 20261116)` with `DesignProgram::audit_11family_genome_expressiveness(20, 20261115)` on `README.md:161`.

### `README-FC-03` (`README.md:133`): Cold-clone timing description ("Every command below is fast")
- **Observation**: `README.md:133` states: `Every command below is fast and writes only to ignored paths.` On a fresh clone, the 11 commands in Block 2 take **`510.8 s` (~8.5 minutes)** in total because `rtk cargo test --workspace --locked` takes `135.0 s`, `python3 -m unittest discover -s tests -v` takes `68.8 s`, and `tools/check.py` takes `215.0 s` (re-running all 10 gate stages).
- **Exact `README.md` line fix (`README.md:133`)**:
  - Replace:
    ```markdown
    **1. Check the repository.** Every command below is fast and writes only to ignored paths. Workplan item DOC-05 runs this list from a clean checkout; the log is kept in ignored `.private/checks/`.
    ```
  - With:
    ```markdown
    **1. Check the repository.** Every command below writes only to ignored paths (single-target checks take `<1–32 s`; the full workspace test suite and `tools/check.py` 10-stage gate take `~2–3.5 min` each on a cold clone). Workplan item DOC-05 runs this list from a clean checkout; the log is kept in ignored `.private/checks/`.
    ```

### `README-FC-04` (`README.md:141`): `run_elite_3grid_gci_and_rsm.py --verify-existing` prints historical v1 single-point `SNR_dp=4.86x`
- **Observation**: Command 11 (`rtk proxy python3 tools/run_elite_3grid_gci_and_rsm.py --verify-existing`) prints `PASS: elite GCI+RSM gate certified for xbabe2-gen043-isl10-ind030 (SNR_dp=4.86x, SNR_eloss=6.76x, LRR-vs-SSG dp_u delta=0.256%)` from the historical v1 single-iteration receipt (`docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v1.json`), whereas `README.md:55` and `WHITEPAPER.md` report the v2 50-step window-averaged oscillatory GCI (`docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v2.json`, where `R = -0.373` and `GCI_fine = 0.251%`).
- **Exact `README.md` line fix (`README.md:141` or note after `README.md:147`)**:
  - Add an inline comment on `README.md:141`:
    ```bash
    rtk proxy python3 tools/run_elite_3grid_gci_and_rsm.py --verify-existing  # checks historical v1 receipt; see v2 oscillatory GCI in docs/results/wave2-tpms-elite-gci-rsm-and-f2-oil-v2.json
    ```
