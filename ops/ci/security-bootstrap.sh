#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
cargo install cargo-audit --version 0.22.1 --locked --root .private/security-tools
printf '%s\n' "$JOIL_ROOT/.private/security-tools/bin" >> "${GITHUB_PATH:?CI path file required}"
