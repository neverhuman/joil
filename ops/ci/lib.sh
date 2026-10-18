#!/usr/bin/env bash
set -euo pipefail
JOIL_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export CARGO_INCREMENTAL="${CARGO_INCREMENTAL:-0}"
export CARGO_PROFILE_DEV_DEBUG="${CARGO_PROFILE_DEV_DEBUG:-line-tables-only}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
export RAYON_NUM_THREADS="${RAYON_NUM_THREADS:-1}"
cd "$JOIL_ROOT"
