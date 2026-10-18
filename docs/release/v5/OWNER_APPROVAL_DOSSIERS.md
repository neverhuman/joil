# V5 Owner Approval Dossiers (Cause E)

Status: all five dossiers are **unsigned and pending owner decision**. No approval in this file has been given, and no approval is implied by software acceptance, by a test pass, or by the presence of a dossier.

Scope: five irreversible human sign-offs (V5-26-05, V5-28-01, V5-28-05, V5-30-01, V5-30-05). Each dossier states the decision requested, the prerequisites, the evidence that exists, what the signature does not authorize, and what remains open. Software evidence in the repository supports the review; it does not replace it.

Scientific admission for every packet covered here is false. Software acceptance and scientific admission are separate decisions.

## Reading rules for every dossier

1. The signer reads the decision text and the registry task text in `docs/v5-plan/TASKS.json` before signing.
2. Where the assignment label and the registry task text differ, the dossier records both. The signer must confirm the scope before signing.
3. A signature covers only the exact text and evidence references named in its dossier. A changed reference, a changed protocol or a changed subject invalidates the signature and requires a new dossier.
4. Signing is a one-way decision for the purpose of this register. Reversal is an owner decision and is recorded as a new dossier, not as an edit to this one.
5. Named owner roles are placeholders until the owner assigns a person. No pronoun or person is assumed.

---

## Dossier 1 — V5-26-05: external benchmark and collection budget authority

**Assignment label:** External benchmark and cloud collection budget and spending authority sign-off.

**Registry task text (`V5-26-05`, packet `V5-26`, owner role `render owner`):** "Deliver cached interrupted-safe plate generation." Instructions: cache by bundle, recipe, build and configuration; resume by completed frames; use separate low-priority leased resources; a human reviews layout after semantic tests. Acceptance: every rendered plate carries a manifest and explicit evidence limitations. Negative control: a beautiful image never upgrades source qualification.

**Mismatch to resolve before signing:** the registry text describes rendering cache and review, not external spending. The assignment label describes spending authority. The signer must state which scope is being authorized, or assign a new task ID for spending authority.

**Decision requested:** authorize, or decline, any external benchmark or cloud collection spend tied to V5 evidence collection. The signature would approve a budget ceiling, a named purpose and a named approver. It would not approve any purchase order.

**Prerequisites before signing:**
- The budget ceiling, currency and period are written in a separate owner record, not in this file.
- The external benchmark source list is available in `experiments/validation/v5/accession_register.json`, where candidate sources are recorded with sources not accessed.
- Resource leases for any compute are recorded in the coordination channel and are not implied by this signature.

**Evidence available in the repository:** render-plate manifests and the cached-generation design in the render packet; the external data request manifest `docs/release/v5/EXTERNAL_DATA_REQUESTS.json` listing the physical datasets still required.

**The signature does not authorize:** purchase, vendor contact, data collection, access to any paywalled source, or any cloud resource beyond the approved ceiling.

**Open items:** budget ceiling; approver identity; scope reconciliation with the registry text; confirmation that no external party has been contacted without direct owner authorization.

**Signature:** `[pending owner]`   **Date:** `[pending]`

---

## Dossier 2 — V5-28-01: frozen preregistration protocol signature

**Registry task text (`V5-28-01`, packet `V5-28`, owner role `research owner`):** "Freeze study arms and estimands prospectively." Instructions: use staged representation-by-optimizer controls, a competent conventional optimization baseline and targeted ablations; set independent seeds, pilot-based power, resource-vector normalization and confirmation budget before generation zero. Acceptance: the analysis, stop, exclusion and holdout policy is hashed in the experiment family. Negative control: an unavailable high-fidelity budget cannot trigger a post-hoc endpoint swap after winners are known.

**Decision requested:** sign the preregistration protocol that fixes study arms, estimands, seeds, power basis, resource normalization, confirmation budget, stop rules, exclusions and holdout policy.

**Evidence available in the repository:** `experiments/v5-study/preregistration-v1.json` and `experiments/v5-study/mechanism-interventions-v1.json`.

**Prerequisites before signing:**
- The preregistration file digest is recorded in the signature block, and any later edit to it creates a new protocol version.
- Pilot-based power is supported by a pilot record that exists and is cited by digest. A power figure without a pilot record is not accepted.
- The confirmation budget is stated as a resource vector with normalization declared.
- Stop and exclusion rules are written before any generation-zero result is seen.

**The signature does not authorize:** any experiment run, any endpoint change, any use of unavailable high-fidelity budget as an endpoint, or any scientific claim from the preregistered study.

**Open items:** whether the pilot record exists and is accepted; whether the high-fidelity budget is available; signer identity.

**Signature:** `[pending owner]`   **Date:** `[pending]`

---

## Dossier 3 — V5-28-05: publication and disclosure release sign-off

**Assignment label:** Physical plate and layout manufacturing release sign-off.

**Registry task text (`V5-28-05`, packet `V5-28`, owner role `research owner`):** "Build reproducible private and public review bundles." Instructions: retain full governing evidence privately; create an allowlisted offline reproduction and a dependency, licence and prior-design inventory. Publication and disclosure decisions remain external owner decisions. Acceptance: a negative result can produce a complete paper and seed package. Negative control: no legal novelty, commercial superiority or service qualification is inferred from a descriptor-hull result.

**Mismatch to resolve before signing:** the assignment label refers to physical plate or layout manufacturing release. The registry text refers to publication and disclosure bundles. Manufacturing release belongs to the physical protocol packet (see Dossier 4), not to this one. The signer must confirm which decision is being signed here.

**Decision requested (registry scope):** approve, or decline, release of a reproduction and review bundle for publication or disclosure outside the project.

**Evidence available in the repository:** the publication builder and the query-built paper packet; the allowlisted export rules in `analysis/validation/v5/public_export.schema.json`; the dossier schema in `analysis/validation/v5/dossier.schema.json`.

**Prerequisites before signing:**
- Every bundle file is allowlisted, and the allowlist review is recorded.
- The dependency, licence and prior-design inventory is complete. Any unresolved licence is listed as unresolved, not assumed permissive.
- The bundle contains no private raw data, no customer identity and no proprietary interface.
- Negative results are included in the bundle when they exist.

**The signature does not authorize:** any claim of legal novelty, commercial superiority, industrial installation or service qualification.

**Open items:** scope reconciliation (publication or manufacturing); licence inventory completeness; signer identity.

**Signature:** `[pending owner]`   **Date:** `[pending]`

---

## Dossier 4 — V5-30-01: rig protocol and release governance signature

**Registry task text (`V5-30-01`, packet `V5-30`, owner role `experimental lead`):** "Finalize a reviewable build and rig protocol." Instructions: specify independent containment, responsible review, comparator choice, build quantities from pilot power, randomized blinded A-B-A blocks and calibration and holdout separation. Acceptance: the protocol is concrete before external approval or spending. Negative control: an unsigned draft cannot be described as a registered or approved experiment.

**Decision requested:** sign the rig and build protocol for release, covering containment review, comparator selection, build quantities, blinded block design and calibration or holdout separation.

**Evidence available in the repository:** `experiments/rig/v5/protocol.json` (status recorded as an unsigned draft, not registered, not approved); `experiments/rig/v5/README.md`; `experiments/rig/v5/calibration_manifest.json`; `experiments/holdout/v5/manifest.json`; `analysis/validation/v5/protocol.py` and its tests.

**Prerequisites before signing:**
- Containment: an independent pressure-rated containment assembly is designed and reviewed by a responsible pressure-systems engineer. Proof testing before any wet test is required by the current draft and must be recorded.
- Site-specific design review is recorded. The current draft states that it is required.
- Comparator selection is frozen and justified from the available conventional options.
- Build quantities are derived from a pilot power record, cited by digest.
- Calibration and holdout sets are separated and their separation check is recorded.

**The signature does not authorize:** fabrication, purchase, laboratory booking, data collection, external messaging or any wet test. The protocol draft states that it is planning only.

**Open items:** containment review; pilot power record; comparator freeze; holdout separation check; signer identity. The current status must stay "unsigned draft" until this dossier is signed and the signature is recorded in the protocol file.

**Signature:** `[pending owner]`   **Date:** `[pending]`

---

## Dossier 5 — V5-30-05: legal, disclosure and licence review sign-off

**Assignment label:** Legal, disclosure and licence review sign-off.

**Registry task text (`V5-30-05`, packet `V5-30`, owner role `experimental lead`):** "Prepare private technical and disclosure dossiers." Instructions: store lineage, first-seen time, exact subject and variant, evidence, controls and interventions and human decisions privately; the owner chooses any filing, publication or commercial next step. Acceptance: public export contains only deliberately reviewed, allowlisted material. Negative control: neither software novelty nor a research-quality threshold authorizes industrial installation.

**Decision requested:** approve the private technical and disclosure dossier for legal and licence review, and approve the public export allowlist, or decline either.

**Evidence available in the repository:** the dossier schema `analysis/validation/v5/dossier.schema.json`, which fixes the private classification and the rule that industrial installation is not authorized; the public export schema `analysis/validation/v5/public_export.schema.json`.

**Prerequisites before signing:**
- Lineage, first-seen record and exact subject identity are stored in private storage with digests.
- Every licence of every dependency and every external source is reviewed; unresolved licences are listed as unresolved.
- The public export allowlist is reviewed item by item. Nothing enters public Git without that review.
- Customer identity and proprietary interfaces are excluded from public material.

**The signature does not authorize:** filing, publication, commercial use, licensing to a third party, or industrial installation. Those are separate owner decisions.

**Open items:** legal review outcome; licence inventory completeness; allowlist review; signer identity.

**Signature:** `[pending owner]`   **Date:** `[pending]`

---

## Register summary

| Dossier | Task | Decision | Signed |
| --- | --- | --- | --- |
| 1 | V5-26-05 | External benchmark and collection budget authority (scope to reconcile) | No |
| 2 | V5-28-01 | Frozen preregistration protocol | No |
| 3 | V5-28-05 | Publication and disclosure release (scope to reconcile) | No |
| 4 | V5-30-01 | Rig protocol and release governance | No |
| 5 | V5-30-05 | Legal, disclosure and licence review | No |

No dossier in this file is signed. Each remains open until the owner signs it, and the signature is recorded in the protocol or register named in that dossier.
