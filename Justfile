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

# Forge lane (docs/forge/README.md): evolve, resume exactly, start from the last best, report, random-search control.
forge-evolve run="runs/forge/demo" gens="300" threads="4":
    cargo run --release --locked -p joil-forge -- evolve --campaign campaigns/forge-v2.json --run {{run}} --generations {{gens}} --threads {{threads}}

forge-resume run="runs/forge/demo" gens="600" threads="4":
    cargo run --release --locked -p joil-forge -- evolve --run {{run}} --resume --generations {{gens}} --threads {{threads}}

forge-from-best from="runs/forge/demo" run="runs/forge/next" gens="300" threads="4":
    cargo run --release --locked -p joil-forge -- evolve --campaign campaigns/forge-v2.json --seed-from {{from}} --run {{run}} --generations {{gens}} --threads {{threads}}

forge-report run="runs/forge/demo":
    cargo run --release --locked -p joil-forge -- report --run {{run}}

forge-control out="runs/forge/control.json" evals="28896" threads="4":
    cargo run --release --locked -p joil-forge -- control --campaign campaigns/forge-v2.json --evaluations {{evals}} --out {{out}} --threads {{threads}}

forge-test:
    cargo test --locked -p joil-forge-judges -p joil-forge
