# Architecture navigation

The governing design is MASTER_PLAN.md and v5-plan/ENGINEERING_WORKPLAN.md.
Rust crates own backend, CLI, tools and computation. apps/explorer owns the
Vite/TypeScript/React read-only research UI. Python migration scope and exact
source inventory live in ../PY_TO_RUST_PLAN.md; exceptions are per-file in
../PYTHON_EXCEPTIONS.md. These documents do not change scientific admission.

crucible-cli orchestrates execution; crucible-runtime owns resource leases and
process lifecycle; crucible-store owns durable software evidence; joil-evidence
owns claims and provenance; joil-geometry owns geometry; crucible-stats owns
statistical operations. Follow each packet's exclusive write paths in
WORK_PACKETS.json. Integrator owns shared contracts and manifests.

Generated plan/dashboard/explorer views must be regenerated through their
canonical commands, never edited to change progress. Frozen historical receipts
are immutable inputs. .private/, runs/ and artifacts/ are never public content.
See testing.md for proof commands and ../AGENTS.md for coordination boundaries.
