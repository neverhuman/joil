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

## Release Readiness, Rollback, and Cost/Budget Controls

The launch gate is a successful isolated synthetic smoke with source/artifact
hashes and an acknowledged resource lease. The release gate is the complete
required/security gate, reviewed advisory audit, exact-commit CI and explicit
integrator release approval. Before rollout, retain a backup of the accepted
source, locks, artifacts and receipt hashes. Test restore by verifying those
hashes and the smoke command. Rollback uses a reviewed revert and repeats the
release gates while preserving failed evidence and historical qualifications.

Monitoring records exit status, wall time, memory pressure and output growth.
Enforce a rate limit of two Cargo jobs and two test threads in this lane; treat
abuse or unexpected workload expansion as a stop condition. The budget and spend
cap are zero paid operations; quota is 12 GiB RAM and 5 GiB new outputs, with no
GPU. The token limit for paid API/model calls is zero. These checks require no
paid calls. The kill switch stops owned jobs under operator authority; it must
not affect other projects or workers. Any missing receipt, failed gate, digest
change, expired lease or quota breach is a stop condition. The integrator records
the rollback target, restore evidence, incident and repair receipt before resume.
