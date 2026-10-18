# Software verification and resource boundary

The required gate remains eight official suites: fmt, clippy, rust-tests, baseline, python-tests, public-check, claims-audit and ui. Together these execute ten distinct required steps, including generated scientific status, npm installation and the UI build. UI build now runs explorer contract tests; ordinary Python discovery includes the imported protocol and renderer-boundary tests. Software CI cannot promote scientific status.

Use the pinned Rust toolchain and CI Node version in `.github/workflows/foundation.yml`, plus the pinned plotting dependency in its isolated Python environment. Respect the current named CPU/RAM/scratch lease, the provided per-worker Cargo target, two build jobs and one numerical thread. No GPU, native solver or physical campaign is part of this gate. Native qualification requires a separately authorized tier and actual accepted evidence; no empty native tier is labelled passing.

```sh
rtk proxy python3 tools/v5_plan.py build
rtk proxy python3 tools/v5_plan.py check
rtk proxy python3 tools/v5_explorer_manifest.py build
rtk proxy python3 tools/v5_explorer_manifest.py check
rtk proxy python3 -m unittest discover -s tests -p test_v5_plan.py
rtk proxy python3 tools/check.py
rtk proxy python3 tools/gen_status.py --check
rtk proxy python3 tools/check_public.py
rtk proxy git diff --check
```

On the leased integration worker, prelink tests inside the supplied target and strip debug sections from newly linked executables while retaining nanosecond mtimes and hardlinks. Preserve Cargo `.rlib`/`.rmeta`/`.d` metadata: deleting a needed hash can force relinking and breach the scratch allocation. Keep measured scratch below the authorized pre-gate threshold and record peak resource samples. Successful test scopes clean their own directories; failed scopes remain for diagnosis. Raw outputs and inventories stay private, with SHA-256 references in public receipts.

The historical frozen-source controls require their exact source and producer binding. A rebuilt producer is new build evidence, not recovered old artifacts. The local resumed b9966a2 gate and GitHub Actions run 39109153451 passed on that exact checkpoint. Later source must receive its own complete gate and pushed-head CI before publication claims. The final PR body and private publication receipt identify the exact tested/pushed head, avoiding a self-referential receipt hash.
