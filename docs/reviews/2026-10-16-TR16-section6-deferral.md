# TR-16 physical admission: proposed section-6 deferral

Owner request: 16 October 2026, 08:06 UTC. DOC owns insertion into the workplan; this handoff does not edit the workplan, requirement values, or admission code.

## Exact text for workplan section 6

**TR-16 physical requirement binding and campaign admission — deferred, BLOCKED-SCOPED.** The sandbox requirement/objective binding and refusal of unadmitted claim-bearing campaigns are delivered. Physical admission remains blocked because the research requirement set is draft, its fluid properties and manufacturing limits are provisional, and no source-backed physical limit set has been admitted. `RS-joil-research-v1.json` still has null pressure budget, energy budget per treated volume, minimum absolute pressure, and required continuous duty band. The executable TOML contract remains `admission = "sandbox_only"`. Historical pressure ratios, draft PDR/wall/envelope settings, observed comparator performance, and synthetic admission-test fixtures are not substitutes for those missing limits.

To unblock the physical remainder, the requirements owner must supply versioned, content-bound sources and admit their applicability to the same baseline, installation, fluids, operating scenarios and concentration basis. Required limits are: maximum allowed pressure loss in Pa with its pressure definition and boundary planes; maximum energy in J per treated cubic metre with its treated-flow and system boundary; minimum absolute pressure in Pa with reference location and applicable fluid/temperature conditions; and the continuous duty interval in m³/h with permitted outlet/reject conditions and the applicable pressure-flow curve. The accompanying installation/port/envelope, measured fluid/DSD, material/process allowables, oil-transport validation and independent validation requirements must also be resolved or explicitly remain blocking. Every limit needs its source identifier/revision/digest, units, uncertainty/tolerance and applicability envelope. No numerical value is selected by this deferral.

**Acceptance to close the physical remainder:** an integration test must load that owner-admitted, source-bound contract, verify that the executed ordered objectives and objective hash equal the frozen contract, and demonstrate that the physical runtime constraints consume its pressure, energy, absolute-pressure and duty limits. Changing a limit must create a new requirement identity and invalidate a stale admission; removing a value/source, using an incompatible unit/basis, or changing the objective contract must refuse admission before computation or artifacts. A claim-bearing campaign may run only when the complete mandatory P0 inventory resolves to current `Pass` in the trusted evidence registry. Missing, failed, incomplete, invalid, reopened or revoked dependencies must refuse it. Supplying the limits alone does not clear independent geometry, numerical, structural, label or validation gates. Until this acceptance passes with real admitted sources, retain the current fail-closed path and run only explicitly labelled sandbox campaigns. DOC/INT may close INT-08 on this explicit section-6 deferral without declaring TR-16 physically complete.

## Implementation and test handoff

The existing runtime in `crates/joil-core/src/experiment.rs::admit_search_campaign` supplies an empty P0 inventory and an `Incomplete` resolver. This is intentional: there is no trusted physical receipt resolver for the legacy prior screen. Changing only the TOML admission flag cannot open the path.

The deferred work spans EV's admitted requirement/evidence contract and TR's runtime consumer. It needs a trusted registry resolver and source-bound physical constraints before its positive acceptance case can be implemented. It is not authorized by this wording to replace absent sources with assumed values or booleans.

The future integration test, to be added to `truth_requirements.rs` when the inputs and resolver exist, must exercise:

1. The real admitted contract and full current P0 receipt inventory, including exact scenario/baseline/objective identities; verify preflight admission and a minimal run's `campaign.json` requirement/objective references. Check each physical constraint against its admitted value and declared comparison semantics.
2. Each required limit/source removed in turn, stale source/requirement identity, invalid units/basis, objective order/hash drift, an absent gate, and every non-Pass/revoked gate; each must fail before campaign computation or output-directory creation.
3. Rust/Python receipt parity: both reject the same requirement/objective drift. Synthetic positive fixtures remain labelled software tests and cannot establish real physical admission.

Existing regression checks must remain green:

```bash
cargo test -p joil-core --test truth_requirements --locked -j 4
cargo test -p joil-evidence --test campaign --locked -j 4
python3 tools/check.py
```

The first command currently verifies sandbox identity binding, objective-drift rejection and claim-bearing refusal before artifacts. The second includes explicitly synthetic admission-algorithm tests. Passing these existing checks does **not** supply the missing physical sources or satisfy the future positive integration acceptance.

## Source anchors

- `requirements/RS-joil-research-v1.json`: `status`, `reference.all_properties_status`, the four null entries under `limits`, `limits.envelope_status`, `robustness.interpretation`, and `blocked_on`.
- `requirements/joil-hydrocyclone-v1.toml`: `admission`, `scope`, `blockers`, the ordered objective contract, sandbox-only thresholds, and declared quantile policy.
- `crates/joil-evidence/src/campaign.rs`: `Thresholds`, `Requirements::check`, `verify_executed_objectives`, `REQUIRED_P0_GATES`, and `admit_campaign`.
- `crates/joil-core/src/experiment.rs`: `research_requirements`, `CampaignAdmissionReceipt`, and `admit_search_campaign`.
- `crates/joil-core/tests/truth_requirements.rs` and `crates/joil-evidence/tests/campaign.rs`: current positive sandbox/synthetic checks and negative admission checks.

No code or requirement value changes accompany this deferral text.
