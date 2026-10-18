#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/lib.sh"
# Retain the compatibility runner until every CI suite provisions Rust.
exec python3 tools/check.py "$@"
