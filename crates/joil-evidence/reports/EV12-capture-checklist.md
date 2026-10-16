# EV-12: capture Wave-4 inputs before cleanup

INT executes this checklist; IR wires the writer. This is exact artifact identity,
not numerical or physical qualification. A prepared surface bundle is incomplete.

**Preserve these exact paths in each `runs/wave4-*` case before any deletion:**

| Paths, relative to the case | Purpose |
|---|---|
| `input-manifest.json`, `design-program.json`, `design-ir.json` | Prepared subject/compiler references and original source bytes. |
| The file at `input-manifest.json:surface_stl_relpath` (normally `constant/triSurface/ct35.stl`) | Exact compiled surface bytes. |
| `constant/polyMesh/**` | Executed volume mesh: require `points`, `faces`, `owner`, `neighbour`, `boundary`, plain or `.gz`; also retain every zone/set/other file. Dictionaries and logs cannot reconstruct its identity. |
| `0/**`, `system/**`, `constant/**` | All executed initial/boundary fields, solver dictionaries and properties, including the surface. Require `0/U`, `0/p`, `system/controlDict`, `system/fvSchemes`, `system/fvSolution`. |
| `case-identity.json` | Immutable sidecar produced below. Retain with all indexed files. |

**Finalize after the last input edit and before solver execution or cleanup.**
Finish meshing and any `reconstructPar -constant` first. For an RSM continuation,
finish mesh copying, field copying/conversion (`U`, `p`, `phi`, `k`, `omega`, `nut`,
`epsilon`, `R` as applicable), and dictionary edits before finalizing. Do not
change any indexed input afterward. A changed input set needs a new identified
case, rather than overwriting the previous sidecar.

From the repository root, with the actual case path:

```bash
case_dir=runs/wave4-ct35-control-g1-medium
python3 - "$case_dir" <<'PY'
import sys
from analysis.case_identity import finalize_case_identity, verify_case_identity
finalize_case_identity(sys.argv[1])
print(verify_case_identity(sys.argv[1]))
PY
python3 -m analysis.case_identity --runs-root runs
```

The writer refuses missing artifacts and stale receipts; an identical retry is
allowed. Stop on an error. The final command audits **every** `wave4-*` case
directory; missing sidecars, malformed references, symlinks and missing meshes
fail. Prepared-only and RSM cases are included. A passing copied-input test does
not finalize the original run.

**Sidecar schema:** `joil.wave4-case-identity.v1`, file `case-identity.json`.
Required root chain: `genome_hash`, `ir_hash`, `geometry_hash` are SHA-256 of exact
program, IR and surface file bytes; `mesh_hash` hashes the domain-separated
`mesh_files` inventory; `case_hash` binds that upstream chain, `compiler` and the
`solver_input_files` inventory. Inventory entries map relative paths to
`{sha256, bytes}`. Also retain `source_artifacts`, `prepared_manifest`, `kind`,
`hash_basis` and separate `canonical_genome_hash`, `canonical_ir_hash`,
`canonical_ir_source_genome_hash`. The two program canonical encodings can differ;
neither substitutes for the raw-file identity. Compiler/kernel/config references
require nonempty `id`, `revision` and a lowercase 64-digit `sha256`.

**Before pruning:** archive the entire indexed input set plus sidecar, restore it
to a separate case directory, and run `verify_case_identity` there. Keep the
archive and digest/verification receipt. Keeping only the sidecar cannot support
later byte verification. Outputs, solver/build identity and scientific
qualification evidence require their own retained receipts beyond this scoped
sidecar.

**Already-pruned cases stay incomplete.** Capture original retained artifacts if
available; otherwise create a separately identified rerun and finalize before
cleanup. Never backfill old mesh hashes from dictionaries, logs or a new mesh.
Hand EV the retained/restored case root and audit receipt; EV independently
verifies every manifest before any DONE-SCOPED decision.
