# Release and rollback procedure

The integrator owns releases. Version source is workspace.package in Cargo.toml;
apps/explorer/package.json versions the UI. No release is physically qualified by
software tests. Follow MASTER_PLAN.md and SCIENCE.md for scientific admission.

Before a release, run `just required` and `just score`, inspect every receipt,
and record the exact source commit, dependency lock hashes, artifact SHA-256,
known failures and applicability limits in the release handoff. An audit failure
remains open and must not be described as passing. Review the complete public
file selection with `python3 tools/check_public.py`. Confirm CI on the exact head,
then obtain release authorization through the existing integrator process.

For rollback, identify the last accepted source and artifact hashes, preserve
current failed evidence, and revert the reviewed change in a new commit. Re-run
the required gates before restoring the known artifact. Never rewrite historical
receipts or delete running-case evidence. Stop affected owned jobs only under
their lease and operator authority; do not interrupt other projects or workers.

## Release checklist and evidence

The release owner is the integrator; a reviewer checks the exact commit and the
user authorizes publication. A PR is a review candidate, not a production release.
Record version, Git commit, Cargo.lock and UI lockfile digests, artifact manifest,
CI run URLs and the matching local receipt in the release notes. List changes,
compatibility/migration impact, known issues and unresolved scientific gates.
Do not issue a version tag until these checks and authorization are recorded.

Required checklist: clean selected diff; `just required`, `just security` and
`just score`; exact-head CI; public-content review; reproducible artifact hashes;
reviewer acknowledgement; rollback target and owner. A missing receipt blocks
release. The advisory audit score is not a waiver of its conformance blockers.

## Rollout, verification and rollback triggers

Stage the software-only fixture in an isolated project-local directory. Verify
its documented smoke command, generated status and immutable evidence identities
before making the release available. No deployment or physical campaign follows
automatically from merging a CLI port. Record the smoke exit code and log hash.
Stop rollout on a failing gate, changed artifact digest, stale evidence, protocol
incompatibility or resource-limit breach. Preserve failure logs and use the
rollback procedure above; verify the restored artifact with the same smoke and
required checks. The integrator records the incident, repair PR and verification
receipt before resuming. Historical scientific results never inherit release
qualification from a software rollback or a passing smoke test.
