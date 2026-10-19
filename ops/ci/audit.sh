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

# I019 adoption command catalogue (references, not executed proof).
# Source-workspace cargo examples below belong to the auditor; they are not Joil commands.
# Actual execution and artifacts are recorded in the packet receipt.
# audit-ci: jankurai audit . --mode ratchet --baseline target/jankurai/accepted-baseline.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md
# Artifacts: .jankurai/repo-score.json, .jankurai/repo-score.md
# proof-routing: jankurai audit . --mode ratchet --baseline target/jankurai/accepted-baseline.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md
# Artifacts: .jankurai/repo-score.json, .jankurai/repo-score.md, target/jankurai/repair-queue.jsonl
# proofbind: jankurai proofbind verify . --changed-from
# Artifacts: target/jankurai/proofbind/surface-witness.json, target/jankurai/proofbind/obligations.json
# proofmark-rust: jankurai proofmark rust .
# Artifacts: target/jankurai/proofmark/proofmark-receipt.json, target/jankurai/proofmark/proof-receipt.json
# copy-code: cargo run -p jankurai -- copy-code . --json target/jankurai/copy-code.json --md target/jankurai/copy-code.md
# Artifacts: target/jankurai/copy-code.json, target/jankurai/copy-code.md
# security: jankurai security run . --out target/jankurai/security/evidence.json
# Artifacts: target/jankurai/security/evidence.json
# ci-bad-behavior: cargo test -p jankurai --test language_bad_behavior
# Artifacts: target/jankurai/language-bad-behavior.log
# git-bad-behavior: cargo test -p jankurai --test language_bad_behavior
# Artifacts: target/jankurai/language-bad-behavior.log
# release-bad-behavior: cargo test -p jankurai --test language_bad_behavior
# Artifacts: target/jankurai/language-bad-behavior.log
# ux-qa: jankurai ux audit --config agent/ux-qa.toml --out target/jankurai/ux-qa.json
# Artifacts: target/jankurai/ux-qa.json
# db-migration-analyze: jankurai migrate . --analyze --json target/jankurai/migration-report.json
# Artifacts: target/jankurai/migration-report.json
# contract-drift: jankurai audit . --mode ratchet --baseline target/jankurai/accepted-baseline.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md
# Artifacts: .jankurai/repo-score.json, .jankurai/repo-score.md
# rust-witness: jankurai rust witness build .
# Artifacts: target/jankurai/rust/witness-graph.json
# vibe-coverage: jankurai vibe coverage --source agent/vibe-coverage.toml --tips tips/vibe_coding --json target/jankurai/vibe-coverage.json --md target/jankurai/vibe-coverage.md
# Artifacts: target/jankurai/vibe-coverage.json, target/jankurai/vibe-coverage.md
# coverage-evidence: jankurai coverage audit . --config agent/coverage-sources.toml --json target/jankurai/coverage/coverage-audit.json --md target/jankurai/coverage/coverage-audit.md
# Artifacts: target/jankurai/coverage/coverage-audit.json, target/jankurai/coverage/coverage-audit.md
# authz-matrix: jankurai audit . --mode ratchet --baseline target/jankurai/accepted-baseline.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md
# Artifacts: .jankurai/repo-score.json, .jankurai/repo-score.md
# input-boundary: jankurai audit . --mode ratchet --baseline target/jankurai/accepted-baseline.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md
# Artifacts: .jankurai/repo-score.json, .jankurai/repo-score.md
# agent-tool-supply: jankurai audit . --mode ratchet --baseline target/jankurai/accepted-baseline.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md
# Artifacts: .jankurai/repo-score.json, .jankurai/repo-score.md
# release-readiness: jankurai audit . --mode ratchet --baseline target/jankurai/accepted-baseline.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md
# Artifacts: .jankurai/repo-score.json, .jankurai/repo-score.md
# cost-budget: jankurai audit . --mode ratchet --baseline target/jankurai/accepted-baseline.json --json target/jankurai/repo-score.json --md target/jankurai/repo-score.md
# Artifacts: .jankurai/repo-score.json, .jankurai/repo-score.md
