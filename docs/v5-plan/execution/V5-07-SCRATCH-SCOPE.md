# Bounded test scratch lifecycle — G-006

The official gate was interrupted at d1e28279c6f4ac058600ccac9dfe906dfd20ae29 by the owned scratch cap; it has no result. Repeated successful store/runtime/CLI tests retained per-run directories.

This narrow repair adds one test-only scope guard shared through the existing contract test support directory. Each affected test registers its unique private paths and holds the guard until stores, databases and children finish. Successful tests remove only registered directories; a panic retains them for diagnosis. A subprocess control must actually fail, retain its diagnostic bytes, and be checked by its parent before bounded cleanup. Production run roots and checkpoint retention are unchanged.

Exact source paths: `crates/crucible-contract/tests/common/scratch.rs`, `crates/crucible-contract/tests/scratch.rs`, `crates/crucible-runtime/tests/attempts.rs`, `crates/crucible-runtime/tests/recovery.rs`, `crates/crucible-store/tests/store.rs`, `crates/crucible-cli/tests/lifecycle.rs`. The integrator also repairs missing-file refusal in `tools/v5_plan.py` with `tests/test_v5_defect_closure.py`. No manifests, model defaults, physical admission or Claude-owned engineering paths change.

Before the next unchanged official eight-suite gate, measure worker scratch below 2.0 GB and keep the original 2.5 GiB limit. Preserve interrupted logs and historical artifacts through additive private retention mappings. Completion remains held pending current gate, exact-head CI and receipts; the original V5-00-03 criterion and audit hold remain in force.
