#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/../ops/ci/lib.sh"
for tool in git cargo rustc python3 node npm; do
  command -v "$tool" >/dev/null || { echo "missing prerequisite: $tool" >&2; exit 1; }
done
rustc --version
cargo --version
python3 --version
node --version
npm --version
cargo metadata --locked --no-deps --format-version 1 >/dev/null
