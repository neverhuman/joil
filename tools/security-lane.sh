#!/usr/bin/env bash
set -euo pipefail
# Shared security entrypoint. Required publication/dependency checks fail closed.
# Optional, declared supply-chain checks (not provisioned or executed here):
# syft dir:. -o spdx-json=target/security/sbom.spdx.json
# grype sbom:target/security/sbom.spdx.json -o json > target/security/grype.json
# Bind SBOM/provenance to the reviewed commit and scanner versions before release.
# Optional workflow-lint checks: actionlint and zizmor .github/workflows
# These declarations are not scan results or signed provenance attestations.
repo_root="$(cd "$(dirname "$0")/.." && pwd)"
exec bash "$repo_root/ops/ci/security.sh" "$@"
