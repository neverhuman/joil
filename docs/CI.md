# Foundation CI on the general runner pool

`Foundation` uses the existing `neverhuman-lan` organization runner group with
`self-hosted`, `Linux`, `X64`, and `lan-ci` labels. Repository access must also be
enabled in that group's selected-repository list. All labels are cumulative; a
hostname or Ubuntu-version label would unnecessarily narrow the shared pool.
Both qualified Ubuntu VM images can run these portable checks. Physical host
inventory and controller credentials stay outside this repository.

Eight independent suites become schedulable together: `fmt`, `clippy`,
`rust-tests`, `baseline`, `python-tests`, `public-check`, `claims-audit`, and `ui`.
The UI suite keeps `npm ci` before its build. Matrix failure does not cancel other
suites, so all diagnostics remain available. The final `foundation` check passes
only when every suite succeeds; failed, skipped, or cancelled work cannot pass it.
Retain `foundation` when configuring required branch checks.

PR updates run once through `pull_request`; pushes to `main` and manual dispatch
also run checks. New commits cancel obsolete runs for that PR/ref. Different PRs
retain separate concurrency groups. Jobs use whichever matching VM becomes free;
the workflow does not reserve hosts or change controller lane counts.

Rust suites install pinned Rust 1.95.0, bootstrapping rustup when absent. Only the
UI suite installs pinned Node 22.22.0. Rust caches are separated by OS release,
architecture, toolchain, suite, and lockfile, with a new entry per commit and
restore from the preceding compatible entry. npm's cache follows the UI lockfile.
The existing check runner caps Rust builds/Rayon at four threads and unrelated
math libraries at one. Each job stays inside its existing ephemeral VM budget;
no CFD, CAD campaign, or GPU allocation is implied.

Local commands retain the full default check and allow exact CI reproduction:

```bash
rtk proxy python3 tools/check.py
rtk proxy python3 tools/check.py --suite rust-tests
rtk proxy python3 tools/check.py --suite ui
rtk proxy python3 -m unittest discover -s tests -p test_check.py -v
rtk proxy actionlint .github/workflows/foundation.yml
```

Each invocation writes immutable logs and a receipt under ignored
`.private/checks/<run-id>/`. Receipts identify the requested suite, input HEAD,
source file hashes, exact commands, exits, timings, and log SHA-256 values. CI
uploads only the deliberately selected receipt JSON for 14 days. Raw private
archives are excluded. Command failures show their diagnostic tail in the Actions
log; missing setup tools fail their setup step and may leave no check receipt.
A suite receipt proves only its requested checks; it is not full foundation or
physical validation.

If jobs queue while the pool is idle, inspect repository access to the runner
group first, then matching labels and online status. CLI access to organization
runner settings requires appropriate organization permissions. Do not widen the
group to every repository or alter shared controller services to fix one repo.
