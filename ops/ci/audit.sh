#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
# The runner image must provision Jankurai; no floating network install in PR jobs.
export PATH="$HOME/.jeryu/bin:$PATH"
command -v jankurai
jankurai --version
jankurai audit . --read-only --no-badge --no-score-history --json .jankurai/repo-score.json --md .jankurai/repo-score.md
