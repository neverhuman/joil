# Bounded local proof lanes; the complete Foundation contract stays required.
export CARGO_INCREMENTAL := "0"
export CARGO_PROFILE_DEV_DEBUG := "line-tables-only"
export CARGO_BUILD_JOBS := "2"
export RAYON_NUM_THREADS := "1"

fast:
    cargo test --locked -p crucible-cli --bin check-public
    cargo run --locked -p crucible-cli --bin check-public -- .
    python3 tools/v5_plan.py check
    python3 tools/v5_explorer_manifest.py check

check: required

required:
    bash scripts/ci-local.sh

security:
    bash ops/ci/security.sh

score:
    bash ops/ci/audit.sh

portability:
    cargo run --locked -p crucible-cli --bin check-portability
