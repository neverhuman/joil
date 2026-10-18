# Python exceptions and retirement policy

Allowed implementation languages in neverhuman/joil are **Rust** for backend,
CLI, tools and compute, and **Vite + TypeScript + React** for apps/explorer.
**No new Python is permitted.** Review rejects added .py files. Existing migration
sources must not grow new product behavior. Workflow shell only dispatches tools.

The complete frozen inventory and active Wave P1–P5 migration tracker are in
[PY_TO_RUST_PLAN.md](PY_TO_RUST_PLAN.md). All files not listed below are
port-to-Rust, not permanent exceptions. Until their assigned wave migrates callers,
existing gates remain operational to avoid dropping regression coverage. P1a adds
Rust publication and portability controls; P1b removes Python compatibility after
Foundation consumer migration. Remaining waves target 2026-10-29, 2026-11-05,
2026-11-15 and 2026-11-22; the integrator reviews overdue items explicitly.

Only minimal Blender bpy integration is a runtime exception. Frozen audit scripts
are historical evidence exceptions, not active gate implementations: active
replacement gates run in Rust. Never rewrite historical receipt hashes to bless
new bytes. The SHA-256 below pins every exception at the dispatch baseline and
supplements existing historical receipt references. Verify both when replaying
history. Blender coordinator and test logic are not bpy exceptions; they move to Rust in P5.
No exception grants scientific validation or permission to execute raw history.

| Existing file | One-line technical justification | Baseline SHA-256 |
| --- | --- | --- |
| `crates/joil-evidence/receipts/ev12_disposition_audit.py` | Frozen historical audit source; preserve receipt identity, never use as a new active gate | `b6292ab2586a4d0efb84a5192d4f363269458259c3acc48b01890ab72f55ad57` |
| `crates/joil-evidence/receipts/ev12_every_run_audit.py` | Frozen historical audit source; preserve receipt identity, never use as a new active gate | `3df9ae532840086b156d730abc91c74d0ee66fbbeae7a62bf1e62a176ca30756` |
| `crates/joil-evidence/receipts/ev12_writer_review.py` | Frozen historical audit source; preserve receipt identity, never use as a new active gate | `ab572b1423c908ffbb46ca8dee6945b3d7daf83a5dbd5da457de5ecbf79d14d0` |
| `crates/joil-evidence/receipts/ev13_figure_trace.py` | Frozen historical audit source; preserve receipt identity, never use as a new active gate | `5e617b75ec415a47077013acac51219473a825e17bd91bf0a7726f64a1ea9f5b` |
| `crates/joil-evidence/receipts/ev13_figure_values.py` | Frozen historical audit source; preserve receipt identity, never use as a new active gate | `68569aa8dba595cffc43d179afd6e8fc125836d55159e846b85e0828a0ba6467` |
| `crates/joil-evidence/receipts/ev13_followup_reverify.py` | Frozen historical audit source; preserve receipt identity, never use as a new active gate | `77e00325cbcb0203fd531200f2822dec9bceb464007ff16e2718abccc5f6dd3a` |
| `crates/joil-evidence/receipts/ev_final_caption_audit.py` | Frozen historical audit source; preserve receipt identity, never use as a new active gate | `fc06bd0b494f4ad7c1070ff40a4abecf06a09663c44ac2a8732379b3a186ae29` |
| `crates/joil-evidence/receipts/plate10_field_array_review.py` | Frozen historical audit source; preserve receipt identity, never use as a new active gate | `7e97b59563f507dc460a6045ab4417e4c6a62df7abf10625ed91f156d443579d` |
| `crates/joil-evidence/receipts/plate10_field_arrays.py` | Frozen historical audit source; preserve receipt identity, never use as a new active gate | `a2cafe575d0677e0a11ccba60eed6c092d1344db64b0a04bfda0e3eae8e9654f` |
| `crates/joil-evidence/receipts/wave4_gci_trace.py` | Frozen historical audit source; preserve receipt identity, never use as a new active gate | `e298adeb968cd52c03f0a08c0779909f4b08c86aba06cba7e3f34aec70cc537f` |
| `crates/joil-evidence/receipts/wave4_number_trace.py` | Frozen historical audit source; preserve receipt identity, never use as a new active gate | `a6794534e2a086edccc758a1bf4d8e85b777990e50d999edcf916d3c4dfb3f69` |
| `crates/joil-evidence/receipts/wave4_raw_proof.py` | Frozen historical audit source; preserve receipt identity, never use as a new active gate | `e029da2443231eeaee8e107f59bf49fff7714fd221e491c922d631b996bff210` |
| `crates/joil-evidence/receipts/wave4_raw_proof_tests.py` | Frozen historical audit source; preserve receipt identity, never use as a new active gate | `21e09183b9bf201b3e53f1346a10a5fd232335b9daa8f40609a2c5da0e95bdc0` |
| `docs/receipts/v5-00-checks.py` | Frozen historical audit source; preserve receipt identity, never use as a new active gate | `d8341794ccfecd53f3b2b0c585675f03357caaa56fab67e67b1f9acb3fed0cf4` |
| `docs/receipts/v5-00-python-probes.py` | Frozen historical audit source; preserve receipt identity, never use as a new active gate | `2e93532926d2ebc24b6199027c8af58eea4b388888affd1e7d2763024d736cf5` |
| `docs/receipts/v5-g012-frozen-wave6-control-driver.py` | Frozen historical audit source; preserve receipt identity, never use as a new active gate | `dd6c56583a54a33905a85923d3d2df5e4dfd3d86247d08242948c680cb361c3f` |
| `render/blender/render_job.py` | Blender embedded bpy scene/render integration | `776d8795e2b362467fc193fff46798603fcc249f4ba2cd98d31d49e61c75ead6` |
