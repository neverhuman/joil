# Python exceptions and retirement policy (`docs/` index)

The canonical root exception table and SHA-256 baseline pins are in
[`/PYTHON_EXCEPTIONS.md`](../PYTHON_EXCEPTIONS.md), paired with the full migration
inventory in [`/PY_TO_RUST_PLAN.md`](../PY_TO_RUST_PLAN.md).

- **Allowed languages:** Rust for backend, CLI, tools, and compute; Vite + TypeScript + React for `apps/explorer`.
- **No new Python:** Added `.py` files are rejected in review unless explicitly covered by [`/PYTHON_EXCEPTIONS.md`](../PYTHON_EXCEPTIONS.md).
- **Retained exceptions (17 files):** 16 frozen historical audit/receipt scripts (`crates/joil-evidence/receipts/*.py`, `docs/receipts/v5-00-*.py`, `docs/receipts/v5-g012-*.py`) preserved for immutable receipt replay, plus 1 embedded Blender `bpy` adapter (`render/blender/render_job.py`). All other Python files are scheduled for Rust replacement in Waves P1–P5.
