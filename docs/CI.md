# Foundation CI on the general runner pool

`Foundation` uses the existing `neverhuman-lan` organization runner group with
`self-hosted`, `Linux`, and `lan-ci` labels. Repository access must also be
enabled in that group's selected-repository list. All labels are cumulative; a
hostname or Ubuntu-version label would unnecessarily narrow the shared pool.
X64 and ARM64 Ubuntu VM images can run these portable checks. An architecture
label would exclude compatible hosts from the general pool. Every job, including
the aggregate `foundation` gate, must use this group and these labels; do not add
GitHub-hosted runner labels or a hosted fallback to new workflows. Physical host
inventory and controller credentials stay outside this repository.

Eight independent suites become schedulable together: `fmt`, `clippy`,
`rust-tests`, `baseline`, `python-tests`, `public-check`, `claims-audit`, and `ui`.
The UI suite keeps `npm ci` before its build. Matrix failure does not cancel other
suites, so all diagnostics remain available. The final `foundation` check passes
only when every suite succeeds; failed, skipped, or cancelled work cannot pass it.
Retain `foundation` when configuring required branch checks.

PR updates run once through `pull_request`; pushes to `main` also run the eight
parallel suites. Manual dispatch selects one `all` lane and executes the exact
`bash scripts/ci-local.sh` entrypoint. It provisions all pinned prerequisites,
fetches the complete locked Cargo graph and precompiles tests before producing
one full receipt with all requested steps. Its two test threads stay within the
existing isolated VM allocation. It does not combine partial suite receipts or
grant scientific qualification. No runner service, pool or permissions change.

Dispatch `gh workflow run Foundation --ref BRANCH` against the proposed branch,
then verify the returned run's exact head. A passing `check (all)` and aggregate
`foundation (full receipt)` require the actual complete run; a queued, skipped or
failed lane is not acceptance. Its distinct name cannot replace the PR's normal
`foundation` check. Security and independent review remain separate required gates.
The receipt artifact is `foundation-all-ATTEMPT`, retaining selected JSON only.

New commits cancel obsolete runs for that PR/ref. Different PRs
retain separate concurrency groups. Jobs use whichever matching VM becomes free;
the workflow does not reserve hosts or change controller lane counts. ARM64
workers run inside disposable Linux VMs, including when their physical host runs
macOS. Host files, SSH agents and controller credentials are not available to jobs.

Rust suites install pinned Rust 1.95.0, bootstrapping rustup when absent. The UI
and full-entrypoint suites install pinned Node 22.22.0. Rust caches are separated by OS release,
architecture, toolchain, suite, and lockfile, with a new entry per commit and
restore from the preceding compatible entry. The `rust-tests` and `all` lanes
build directly instead of restoring/saving the large workspace target cache:
observed restore time exceeded twelve minutes on the general pool, while fresh
precompilation completed in under two minutes. Their locked fetch, workspace
precompile and complete test commands remain required. npm's cache follows the UI lockfile.
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

Lifecycle CLI tests exercise bounded synthetic execution and process-crash/replay
protocols. CI gives only those fixtures a private root on the guest's existing
tmpfs through `JOIL_LIFECYCLE_TEST_SCRATCH`; ordinary local runs retain their disk
default. Other store/runtime durability tests stay on disk. The job records small
comparative file-fsync observations, each bounded at 15 seconds, and fails if the
declared tmpfs prerequisite or 512 MiB available capacity is absent. A final
always-run check records retained scratch size and fails above 512 MiB; this is a
retained-output check, not a claim of a separately mounted filesystem quota.
No host mount or runner configuration changes.
These tests do not qualify power-loss persistence or filesystem performance.
Native execution deadlines, CLI watchdog bounds, retries, charging and real
injected-timeout controls remain unchanged. Watchdog failures record their actual
command/output and bounded context before raising the original timeout failure.

If jobs queue while the pool is idle, inspect repository access to the runner
group first, then matching labels and online status. CLI access to organization
runner settings requires appropriate organization permissions. Do not widen the
group to every repository or alter shared controller services to fix one repo.
