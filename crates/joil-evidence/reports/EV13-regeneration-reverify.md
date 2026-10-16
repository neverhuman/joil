# Independent EV-13 regeneration verification

Target: acting integrator TR's committed `11033dd59c1ed32c1f4765e7b2c3971d17b27a8b`.

All **nine PNGs reproduce byte for byte**, including all six regenerated
figures. All six reported PNG/manifest digests match independently read git
blobs. The [value trace](../receipts/EV13-regenerated-figure-value-trace.json)
has **791 successful explicit checks, zero numeric differences, and 14
UNTRACED rows** for Plate 10's private-source calls at this audited head.
The [receipt](../receipts/EV13-regeneration-reverify.json) records hashes,
source JSON pointers, exact commands, negative checks and scope.

| Finding | Independently verified repair |
|---|---|
| F01 | Shared A1/A2/B3 Gen0; B2 conventionalized copies; B1 random-search wording |
| F02 | Five search arms plus frozen reference; milestones and authored-family diagnostics in manifest |
| F05 | Blanket cell-count legends removed; pressure/half-span arrays still match |
| F06 ordering/replay | Historical Wave-3 has B3 > B1, Holm p=0.01017, median delta=+7.76; both heroes use corrected v2 CFD half-spans; historical replay now exact |
| F07 | Legacy HV telemetry, executed duty protocol deviation, unsupported omission gate removed |
| F08 | Authored research-screen threshold from committed research policy |
| F09 | Unsupported mass-closure gate annotation removed; plotted closure values unchanged |

Old `1c9c3b4` audit receipts stay unchanged. Its replay still returns 783
successful checks / zero numeric differences / 16 UNTRACED rows. The reviewed
source-line map handles statement movement and preserves actual new source
coordinates; three literal-30-to-policy-variable changes were inspected
explicitly. Five altered-value/coverage/identity negatives reject on the new pin.

Remaining handoffs:

- **F03:** the three-arm equal-budget figure is absent. The displayed CT-35 value is a frozen reference; the new repeated CT-35 arm belongs to the separate raw-capture campaign.
- **F04:** EV delivered [source arrays](../receipts/Plate10-field-arrays.json) in `02ed2fd`, with all 32 argument arrays matching and JSON-only replay exact. At `11033dd`, the normal renderer/manifest still depend on private fields. TR must consume the JSON and bind its SHA. The 14 UNTRACED rows describe that audited head; they do not claim that the separately committed arrays are missing.
- **F04 spatial/identity:** the legacy owned-face-first-vertex coordinate method puts 12,670 / 12,533 / 12,503 cells at the origin and is not finite-volume centroid sampling. Retained CT-35 has 313,260 cells versus archived 334,768. Keep these caveats or make a separate reviewed correction; source traceability is not physical validation.
- **Historical budget wording:** DOC's 13:02:31Z NEED remains. Wave-3's 450 / 22,500 are recorded evaluations, not the repaired Wave-4 charged-proposal accounting. Historical figure wording needs that distinction.
- **Normal-stress wording:** DOC's same NEED remains. Plate 3's 100% measures positive diagonal normal stresses, not complete tensor realizability.
- **Evidence scope:** EV-09 is BLOCKED-SCOPED on absent independent labels and family/mechanism holdouts. EV-12 accounts for twelve cases through three exact active chains and nine non-claim exclusions. Historical mesh, FEA/material/load-transfer, oil and physical validation remainders stay deferred; the original historical raw campaign was never captured.

This verifies requested numerical/raster repairs with explicit residuals. It is
not an all-claims-traced or scientific/physical PASS. DOC owns prose; TR owns
integration and further generator changes. EV authored only its receipts/reports.
