#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."

TMP_OUT="$(mktemp -d)"
trap 'rm -rf "$TMP_OUT"' EXIT

JANKURAI_BIN=""
if [[ "${JANKURAI_FORCE_DOWNLOAD:-0}" != "1" ]]; then
  if [[ -x "/home/ubuntu/.jeryu/bin/jankurai" ]]; then
    JANKURAI_BIN="/home/ubuntu/.jeryu/bin/jankurai"
  elif command -v jankurai >/dev/null 2>&1; then
    JANKURAI_BIN="$(command -v jankurai)"
  fi
fi

if [[ -z "$JANKURAI_BIN" ]]; then
  ARCHIVE="$TMP_OUT/jankurai-1.7.5-x86_64-unknown-linux-gnu.tar.gz"
  curl --fail --location --silent --show-error --retry 3 \
    "https://github.com/neverhuman/jankurai-audit/releases/download/v1.7.5/jankurai-1.7.5-x86_64-unknown-linux-gnu.tar.gz" \
    --output "$ARCHIVE"
  echo "792c16465c274a266f64541152eb5c796fdf22113120133789d28bf99bcf915c  $ARCHIVE" | sha256sum --check --status
  tar -xzf "$ARCHIVE" -C "$TMP_OUT"
  JANKURAI_BIN="$TMP_OUT/jankurai-1.7.5-x86_64-unknown-linux-gnu/jankurai"
fi

"$JANKURAI_BIN" --version
"$JANKURAI_BIN" audit . \
  --read-only \
  --no-badge \
  --no-score-history \
  --mode advisory \
  --json "$TMP_OUT/repo-score.json" \
  --md "$TMP_OUT/repo-score.md" \
  --repair-queue-jsonl "$TMP_OUT/repair-queue.jsonl" \
  --github-step-summary "$TMP_OUT/summary.md"

mkdir -p .jankurai
cp "$TMP_OUT/repo-score.json" .jankurai/repo-score.json
cp "$TMP_OUT/repo-score.md" .jankurai/repo-score.md
if [[ -f "$TMP_OUT/repair-queue.jsonl" ]]; then
  cp "$TMP_OUT/repair-queue.jsonl" .jankurai/repair-queue.jsonl
fi
if [[ -n "${GITHUB_STEP_SUMMARY:-}" && -f "$TMP_OUT/summary.md" ]]; then
  cat "$TMP_OUT/summary.md" >> "$GITHUB_STEP_SUMMARY"
fi

test -s .jankurai/repo-score.json
test -s .jankurai/repo-score.md
jq -r '"score=\(.score) raw=\(.raw_score) caps=\(.caps_applied | length) findings=\(.findings | length)"' .jankurai/repo-score.json
jq -e '.score >= 45' .jankurai/repo-score.json >/dev/null
