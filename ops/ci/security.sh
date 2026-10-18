#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
# Missing scanners fail closed; provision cargo-audit explicitly, project-locally.
cargo run --locked -p crucible-cli --bin check-public -- .
cargo audit --version
cargo audit --db .private/security-advisory-db --file Cargo.lock
npm audit --prefix apps/explorer --cache .private/npm-cache --omit=dev --audit-level=high
