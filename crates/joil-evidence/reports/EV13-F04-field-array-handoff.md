# Plate 10 source-array handoff (EV13-F04)

The selected field grids and profiles are in
[Plate10-field-arrays.json](../receipts/Plate10-field-arrays.json).
[The review](../receipts/EV13-F04-field-array-review.json) traces all 14 formerly
untraced plotting calls through 32 exact array bindings. Its JSON-only private
replay reproduces the existing PNG, including its SHA-256, without raw fields or
mesh access. Pixel values are never an extraction input.

Generator bindings:

| Existing variable | JSON pointer in `Plate10-field-arrays.json` |
|---|---|
| `w1_grid` | `#/cases/wave1/meridional` |
| `w2_grid` | `#/cases/wave2_tpms/meridional` |
| `w1_prof` | `#/cases/wave1/profiles` |
| `w2_prof` | `#/cases/wave2_tpms/profiles` |
| `base_prof` | `#/cases/baseline/profiles` |

Convert grid lists with `np.asarray(value, dtype=float)`; this restores JSON null
to NaN. Preserve missing bins rather than substituting zero. The artifact includes
bin counts/masks, mean `k` grids and axial `k` profiles, SI/engineering units,
actual percentile/window definitions, and SHA-256/byte inventories of U/p/k,
every retained polyMesh file, coordinate cache and physicalProperties. The mesh
inventory digest is explicitly a byte-inventory hash, not ScientificIdentity.
Keep the legacy keys for compatibility: radial `u_theta_p75` actually means P90
over z=2..55 mm; axial `u_theta_p90_m_s` actually means P95.

The independent review found material spatial/identity limitations:

| Subject | Retained mesh/field cells | Cells with no owned-face coordinate samples |
|---|---:|---:|
| Wave 1 | 305,522 | 12,670 |
| Wave 2 TPMS | 305,311 | 12,533 |
| CT-35 | 313,260 | 12,503 |

The historical coordinate method averages the first vertex of owned faces only;
it omits neighbour contributions and places zero-owner cells at [0,0,0]. The
retained caches exactly reproduce that method, **not finite-volume centroids**.
The retained CT-35 mesh count also differs from the archived dossier's 334,768.
No same-archived-mesh identity is certified. These are source-bound legacy
diagnostic arrays, not corrected spatial data, oil validation or physical proof.
TR must preserve these qualifications in the generator/manifest/caption, or make
a separate reviewed coordinate/data correction. Merely binding the JSON does
not resolve these defects or the historical/FEA/EV-09 remainders.

Reproduce extraction and tests with the script/helper versions identified by
their committed hashes in the JSON:

```bash
.private/ci-python/bin/python crates/joil-evidence/receipts/plate10_field_arrays.py --runs-root /home/ubuntu/joil/runs --output crates/joil-evidence/receipts/Plate10-field-arrays.json
.private/ci-python/bin/python crates/joil-evidence/receipts/plate10_field_array_review.py --bundle crates/joil-evidence/receipts/Plate10-field-arrays.json --capture .private/ev13-rendered/captured.json --output crates/joil-evidence/receipts/EV13-F04-field-array-review.json --replay-root .private/ev13-f04-json-replay
```

The first command inventories inputs before and after, rejects incompatible
dimensions/counts/nonfinite fields or changed caches, and never writes into raw
cases. Ten extractor checks and three altered-array/subject/mask negatives pass.
TR owns normal generator consumption and the authoritative manifest update;
EV independently re-verifies the committed regeneration afterward.
