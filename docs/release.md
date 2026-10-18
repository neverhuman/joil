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
