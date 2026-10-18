# I010 Jankurai remediation tracker

Initial audit after I010 source edits: score 45, raw 45, 17 caps, 68 open findings,
42 blockers. These are scan findings, not physical validation. Final counts and
report SHA-256 are in the I010 handoff; the table preserves this triage snapshot.
The subsequent scan reports score 52, 16 caps, 66 findings and 41 blockers;
toolchain pinning and release/rollback guidance are now present. Re-audit by
fingerprint before closing a task; changed aggregate scores alone are not closure.
Owner for each item is the repository integrator, with subsystem routing below.
Dates are review targets. No finding is suppressed, waived or called fixed merely
because it has a task. The unchanged Foundation suites remain required.

| Task | Rule | Source | Finding | Owner / review target | Action |
| --- | --- | --- | --- | --- | --- |
| I010-J001 | HLT-001-DEAD-MARKER | `.` | Code shape and semantic surface scored 2 below the standard floor of 85 | Integrator → integration / 2026-10-25 | Inspect cited evidence, add focused proof or remediate under exclusive packet scope; retain unresolved status until re-audit. |
| I010-J002 | HLT-000-SCORE-DIMENSION:audit | `.github/workflows` | CI does not run the jankurai audit lane | Integrator → CI / 2026-10-22 | Verify runner isolation, provisioning, supply-chain and local parity; preserve all Foundation checks. No shared runner changes in I010. |
| I010-J003 | HLT-009-GENERATED-SECURITY | `.github/workflows` | high-risk repo has no explicit security lane | Integrator → CI / 2026-10-22 | Verify runner isolation, provisioning, supply-chain and local parity; preserve all Foundation checks. No shared runner changes in I010. |
| I010-J004 | HLT-016-SUPPLY-CHAIN-DRIFT | `.github/workflows` | no secret or dependency scanning was found in CI | Integrator → CI / 2026-10-22 | Verify runner isolation, provisioning, supply-chain and local parity; preserve all Foundation checks. No shared runner changes in I010. |
| I010-J005 | HLT-042-CI-LOCAL-PARITY | `.github/workflows/foundation.yml` | ops/ci/lib.sh is missing | Integrator → CI / 2026-10-22 | Verify runner isolation, provisioning, supply-chain and local parity; preserve all Foundation checks. No shared runner changes in I010. |
| I010-J006 | HLT-042-CI-LOCAL-PARITY | `.github/workflows/foundation.yml` | rust-toolchain.toml is missing | Integrator → CI / 2026-10-22 | Verify runner isolation, provisioning, supply-chain and local parity; preserve all Foundation checks. No shared runner changes in I010. |
| I010-J007 | HLT-042-CI-LOCAL-PARITY | `.github/workflows/foundation.yml` | scripts/ci-doctor.sh is missing | Integrator → CI / 2026-10-22 | Verify runner isolation, provisioning, supply-chain and local parity; preserve all Foundation checks. No shared runner changes in I010. |
| I010-J008 | HLT-042-CI-LOCAL-PARITY | `.github/workflows/foundation.yml` | scripts/ci-local.sh is missing | Integrator → CI / 2026-10-22 | Verify runner isolation, provisioning, supply-chain and local parity; preserve all Foundation checks. No shared runner changes in I010. |
| I010-J009 | HLT-042-CI-LOCAL-PARITY | `.github/workflows/foundation.yml` | workflow inlines CI commands instead of delegating to ops/ci/*.sh | Integrator → CI / 2026-10-22 | Verify runner isolation, provisioning, supply-chain and local parity; preserve all Foundation checks. No shared runner changes in I010. |
| I010-J010 | HLT-034-CI-BAD-BEHAVIOR | `.github/workflows/foundation.yml` | untrusted CI job runs with privileged Docker access | Integrator → CI / 2026-10-22 | Verify runner isolation, provisioning, supply-chain and local parity; preserve all Foundation checks. No shared runner changes in I010. |
| I010-J011 | HLT-016-SUPPLY-CHAIN-DRIFT | `.github/workflows/jankurai.yml` | Security and supply-chain posture scored 38 below the standard floor of 85 | Integrator → CI / 2026-10-22 | Verify runner isolation, provisioning, supply-chain and local parity; preserve all Foundation checks. No shared runner changes in I010. |
| I010-J012 | HLT-034-CI-BAD-BEHAVIOR | `.github/workflows/jankurai.yml` | untrusted CI job runs with privileged Docker access | Integrator → CI / 2026-10-22 | Verify runner isolation, provisioning, supply-chain and local parity; preserve all Foundation checks. No shared runner changes in I010. |
| I010-J013 | HLT-007-HANDWRITTEN-CONTRACT | `agent/boundaries.toml` | Contract and boundary integrity scored 53 below the standard floor of 85 | Integrator → integration / 2026-10-25 | Inspect cited evidence, add focused proof or remediate under exclusive packet scope; retain unresolved status until re-audit. |
| I010-J014 | HLT-003-OWNERLESS-PATH | `agent/owner-map.json` | Ownership and navigation surface scored 60 below the standard floor of 85 | Integrator → integration / 2026-10-25 | Inspect cited evidence, add focused proof or remediate under exclusive packet scope; retain unresolved status until re-audit. |
| I010-J015 | HLT-004-UNMAPPED-PROOF | `agent/test-map.json` | Proof lanes and test routing scored 52 below the standard floor of 85 | Integrator → integration / 2026-10-25 | Inspect cited evidence, add focused proof or remediate under exclusive packet scope; retain unresolved status until re-audit. |
| I010-J016 | HLT-000-SCORE-DIMENSION:stack | `analysis/__init__.py` | runtime code uses a language outside the chosen optimal stack | Integrator → integration / 2026-10-25 | Inspect cited evidence, add focused proof or remediate under exclusive packet scope; retain unresolved status until re-audit. |
| I010-J017 | HLT-001-DEAD-MARKER | `analysis/kpi_fitness.py` | fallback soup detected in product code | Integrator → integration / 2026-10-25 | Inspect cited evidence, add focused proof or remediate under exclusive packet scope; retain unresolved status until re-audit. |
| I010-J018 | HLT-031-TYPESCRIPT-BAD-BEHAVIOR | `apps/explorer/src/main.tsx` | unchecked boundary cast or parse result crosses a trust boundary | Integrator → integration / 2026-10-25 | Inspect cited evidence, add focused proof or remediate under exclusive packet scope; retain unresolved status until re-audit. |
| I010-J019 | HLT-031-TYPESCRIPT-BAD-BEHAVIOR | `apps/explorer/src/main.tsx` | raw shell or SQL text is built from untrusted TypeScript input | Integrator → integration / 2026-10-25 | Inspect cited evidence, add focused proof or remediate under exclusive packet scope; retain unresolved status until re-audit. |
| I010-J020 | HLT-023-INPUT-BOUNDARY-GAP | `apps/explorer/src/main.tsx` | unsafe or unvalidated input boundary marker appears in product code | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J021 | HLT-013-RENDERED-UX-GAP | `apps/web` | web surface lacks layered rendered UX QA evidence | Integrator → integration / 2026-10-25 | Inspect cited evidence, add focused proof or remediate under exclusive packet scope; retain unresolved status until re-audit. |
| I010-J022 | HLT-008-FALSE-GREEN-RISK | `crates/` | Rust surface lacks required property and/or integration tests | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J023 | HLT-027-HUMAN-REVIEW-EVIDENCE-GAP | `crates/crucible-cli/src/explore.rs` | human review or proof claim lacks reproducible evidence | Integrator → integration / 2026-10-25 | Inspect cited evidence, add focused proof or remediate under exclusive packet scope; retain unresolved status until re-audit. |
| I010-J024 | HLT-029-RUST-BAD-BEHAVIOR | `crates/crucible-cli/src/lib.rs` | unsafe block lacks a nearby local safety argument | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J025 | HLT-029-RUST-BAD-BEHAVIOR | `crates/crucible-cli/src/lib.rs` | unsafe block lacks a nearby local safety argument | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J026 | HLT-029-RUST-BAD-BEHAVIOR | `crates/crucible-cli/src/lib.rs` | unsafe block lacks a nearby local safety argument | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J027 | HLT-008-FALSE-GREEN-RISK | `crates/crucible-cli/tests/lifecycle.rs` | test code contains disabled, focused, tautological, or snapshot-only proof | Integrator → integration / 2026-10-25 | Inspect cited evidence, add focused proof or remediate under exclusive packet scope; retain unresolved status until re-audit. |
| I010-J028 | HLT-046-UNNECESSARY-VARIETY | `crates/crucible-contract/src/subject.rs` | enum LengthUnit has 2 divergent definitions across modules where one consistent definition is expected | Integrator → crate owners / 2026-10-29 | Determine whether semantics differ before consolidating types; require consumer parity. |
| I010-J029 | HLT-046-UNNECESSARY-VARIETY | `crates/crucible-contract/src/subject.rs` | enum MaterialState has 2 divergent definitions across modules where one consistent definition is expected | Integrator → crate owners / 2026-10-29 | Determine whether semantics differ before consolidating types; require consumer parity. |
| I010-J030 | HLT-046-UNNECESSARY-VARIETY | `crates/crucible-geom/src/export.rs` | enum AttemptOutcome has 2 divergent definitions across modules where one consistent definition is expected | Integrator → crate owners / 2026-10-29 | Determine whether semantics differ before consolidating types; require consumer parity. |
| I010-J031 | HLT-046-UNNECESSARY-VARIETY | `crates/crucible-gpu/src/lib.rs` | enum Admission has 2 divergent definitions across modules where one consistent definition is expected | Integrator → crate owners / 2026-10-29 | Determine whether semantics differ before consolidating types; require consumer parity. |
| I010-J032 | HLT-046-UNNECESSARY-VARIETY | `crates/crucible-render/src/lib.rs` | enum EvidenceClass has 2 divergent definitions across modules where one consistent definition is expected | Integrator → crate owners / 2026-10-29 | Determine whether semantics differ before consolidating types; require consumer parity. |
| I010-J033 | HLT-046-UNNECESSARY-VARIETY | `crates/crucible-render/src/lib.rs` | enum Disclosure has 2 divergent definitions across modules where one consistent definition is expected | Integrator → crate owners / 2026-10-29 | Determine whether semantics differ before consolidating types; require consumer parity. |
| I010-J034 | HLT-029-RUST-BAD-BEHAVIOR | `crates/crucible-runtime/src/lease.rs` | unsafe block lacks a nearby local safety argument | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J035 | HLT-029-RUST-BAD-BEHAVIOR | `crates/crucible-runtime/src/lease.rs` | unsafe block lacks a nearby local safety argument | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J036 | HLT-029-RUST-BAD-BEHAVIOR | `crates/crucible-runtime/src/lease.rs` | unsafe block lacks a nearby local safety argument | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J037 | HLT-029-RUST-BAD-BEHAVIOR | `crates/crucible-runtime/src/lease.rs` | unsafe block lacks a nearby local safety argument | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J038 | HLT-029-RUST-BAD-BEHAVIOR | `crates/crucible-runtime/src/lease.rs` | unsafe block lacks a nearby local safety argument | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J039 | HLT-029-RUST-BAD-BEHAVIOR | `crates/crucible-runtime/src/lease.rs` | unsafe block lacks a nearby local safety argument | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J040 | HLT-029-RUST-BAD-BEHAVIOR | `crates/crucible-runtime/src/lease.rs` | unsafe block lacks a nearby local safety argument | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J041 | HLT-029-RUST-BAD-BEHAVIOR | `crates/crucible-runtime/src/lease.rs` | unsafe block lacks a nearby local safety argument | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J042 | HLT-029-RUST-BAD-BEHAVIOR | `crates/crucible-runtime/src/lease.rs` | unsafe block lacks a nearby local safety argument | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J043 | HLT-029-RUST-BAD-BEHAVIOR | `crates/crucible-runtime/src/lease.rs` | unsafe block lacks a nearby local safety argument | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J044 | HLT-029-RUST-BAD-BEHAVIOR | `crates/crucible-runtime/src/lease.rs` | unsafe block lacks a nearby local safety argument | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J045 | HLT-029-RUST-BAD-BEHAVIOR | `crates/crucible-runtime/src/lease.rs` | unsafe block lacks a nearby local safety argument | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J046 | HLT-029-RUST-BAD-BEHAVIOR | `crates/crucible-runtime/src/lease.rs` | unsafe block lacks a nearby local safety argument | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J047 | HLT-029-RUST-BAD-BEHAVIOR | `crates/crucible-runtime/src/lease.rs` | unsafe block lacks a nearby local safety argument | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J048 | HLT-029-RUST-BAD-BEHAVIOR | `crates/crucible-runtime/src/lease.rs` | unsafe block lacks a nearby local safety argument | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J049 | HLT-029-RUST-BAD-BEHAVIOR | `crates/crucible-runtime/src/lease.rs` | unsafe block lacks a nearby local safety argument | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J050 | HLT-029-RUST-BAD-BEHAVIOR | `crates/crucible-runtime/src/lease.rs` | unsafe block lacks a nearby local safety argument | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J051 | HLT-029-RUST-BAD-BEHAVIOR | `crates/crucible-runtime/src/lease.rs` | unsafe block lacks a nearby local safety argument | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J052 | HLT-029-RUST-BAD-BEHAVIOR | `crates/crucible-runtime/src/lease.rs` | unsafe block lacks a nearby local safety argument | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J053 | HLT-029-RUST-BAD-BEHAVIOR | `crates/crucible-runtime/src/task.rs` | unsafe block lacks a nearby local safety argument | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J054 | HLT-029-RUST-BAD-BEHAVIOR | `crates/crucible-runtime/src/task.rs` | unsafe block lacks a nearby local safety argument | Integrator → Rust runtime / 2026-10-25 | Inspect each unsafe precondition and test failure/cleanup paths before adding local safety arguments. |
| I010-J055 | HLT-046-UNNECESSARY-VARIETY | `crates/crucible-stats/src/analysis/order.rs` | enum Observation has 2 divergent definitions across modules where one consistent definition is expected | Integrator → crate owners / 2026-10-29 | Determine whether semantics differ before consolidating types; require consumer parity. |
| I010-J056 | HLT-046-UNNECESSARY-VARIETY | `crates/crucible-units/src/lib.rs` | enum Unit has 2 divergent definitions across modules where one consistent definition is expected | Integrator → crate owners / 2026-10-29 | Determine whether semantics differ before consolidating types; require consumer parity. |
| I010-J057 | HLT-017-OPAQUE-OBSERVABILITY | `crates/domain` | no agent-friendly exception/error pattern was detected | Integrator → integration / 2026-10-25 | Inspect cited evidence, add focused proof or remediate under exclusive packet scope; retain unresolved status until re-audit. |
| I010-J058 | HLT-033-PYTHON-BAD-BEHAVIOR | `crates/joil-evidence/receipts/ev13_figure_trace.py` | dynamic code execution runs attacker-controlled source directly | Integrator → language migration / 2026-10-29 | P1–P5 per PY_TO_RUST_PLAN.md; preserve frozen receipts and replace active gates with Rust. |
| I010-J059 | HLT-040-REPO-ROT-BAD-BEHAVIOR | `crates/joil-evidence/registry/legacy-status-before-v4.json` | active source path looks like an old, backup, copied, or parked implementation | Integrator → integration / 2026-10-25 | Inspect cited evidence, add focused proof or remediate under exclusive packet scope; retain unresolved status until re-audit. |
| I010-J060 | HLT-046-UNNECESSARY-VARIETY | `crates/joil-evidence/src/campaign.rs` | enum CampaignMode has 2 divergent definitions across modules where one consistent definition is expected | Integrator → crate owners / 2026-10-29 | Determine whether semantics differ before consolidating types; require consumer parity. |
| I010-J061 | HLT-046-UNNECESSARY-VARIETY | `crates/joil-evidence/src/lib.rs` | enum Verdict has 2 divergent definitions across modules where one consistent definition is expected | Integrator → crate owners / 2026-10-29 | Determine whether semantics differ before consolidating types; require consumer parity. |
| I010-J062 | HLT-006-DIRECT-DB-WRONG-LAYER | `db/` | Data truth and workflow safety scored 50 below the standard floor of 85 | Integrator → integration / 2026-10-25 | Inspect cited evidence, add focused proof or remediate under exclusive packet scope; retain unresolved status until re-audit. |
| I010-J063 | HLT-025-RELEASE-READINESS-GAP | `docs/release.md` | release management structure is incomplete | Integrator → integration / 2026-10-25 | Inspect cited evidence, add focused proof or remediate under exclusive packet scope; retain unresolved status until re-audit. |
| I010-J064 | HLT-017-OPAQUE-OBSERVABILITY | `docs/testing.md` | Observability and repair evidence scored 56 below the standard floor of 85 | Integrator → integration / 2026-10-25 | Inspect cited evidence, add focused proof or remediate under exclusive packet scope; retain unresolved status until re-audit. |
| I010-J065 | HLT-026-COST-BUDGET-GAP | `docs/testing.md` | cost or spend-risk surface lacks budget proof | Integrator → integration / 2026-10-25 | Inspect cited evidence, add focused proof or remediate under exclusive packet scope; retain unresolved status until re-audit. |
| I010-J066 | HLT-005-PYTHON-PRODUCT-TRUTH | `python/` | Python appears without a rare advanced-ML/data exception or owns product truth | Integrator → language migration / 2026-10-29 | P1–P5 per PY_TO_RUST_PLAN.md; preserve frozen receipts and replace active gates with Rust. |
| I010-J067 | HLT-005-PYTHON-PRODUCT-TRUTH | `python/ai-service` | Python containment and polyglot hygiene scored 0 below the standard floor of 85 | Integrator → language migration / 2026-10-29 | P1–P5 per PY_TO_RUST_PLAN.md; preserve frozen receipts and replace active gates with Rust. |
| I010-J068 | HLT-033-PYTHON-BAD-BEHAVIOR | `tools/claims_audit.py` | dynamic code execution runs attacker-controlled source directly | Integrator → language migration / 2026-10-29 | P1–P5 per PY_TO_RUST_PLAN.md; preserve frozen receipts and replace active gates with Rust. |

## I012 re-audit (18 October 2026)

Measured advisory score **55**, raw **55**, **14 caps**, **61 findings**, **40 conformance blockers**. `bash ops/ci/audit.sh` passes its existing score-45 threshold; this is not full conformance. Report SHA-256: `1c1e84508968e4ac6ab90af59cc3490832977834f97c050e36bb41a03ab89267`.

The four target dimensions now score ownership 78, boundaries 63, proof routing 78, security 60 (I010 snapshot: 60, 53, 52, 38). Source and generated-evidence scan scope can change aggregate findings; fingerprints below determine closure.

I010-J003/J004 now have a versioned Security workflow and secret/dependency scan entrypoint. J005/J007/J008/J009 have shared executable shell entrypoints; J006 was already addressed by the pinned Rust toolchain. J010/J012 have explicit same-repository PR guards, but the scanner still flags self-hosted runners: these findings remain open for runner-policy review. J001 remains outside this scoped port. J002 remains a detector finding despite the existing executable audit workflow. J011/J013/J014/J015 improve but remain below the audit floor. No finding is waived.

The full Foundation gate fails on the unchanged recovery execution-clock assertion (TimedOut vs Committed); the exact workspace-feature test also fails on clean pinned main `2aba9ea5700669813c71156802a7af8f7a6d1403`, exit 101. No deadline or scientific threshold was relaxed.

### Prior target fingerprints absent in this scan

| Rule | Path | Fingerprint |
| --- | --- | --- |
| HLT-009-GENERATED-SECURITY | `.github/workflows` | `sha256:c249be982d975721833fe396cdfff422f53a2d61819df881968fba63fdd6b9bf` |
| HLT-016-SUPPLY-CHAIN-DRIFT | `.github/workflows` | `sha256:2e22551cbdbd8da1f6fedd2d509dba064990dc4b1505df71609f431d11901099` |
| HLT-042-CI-LOCAL-PARITY | `.github/workflows/foundation.yml` | `sha256:113fcf74bc0676605648f3d2cbfedd30e98c674f99f9d1b2ccae3b93c79c2035` |
| HLT-042-CI-LOCAL-PARITY | `.github/workflows/foundation.yml` | `sha256:d79e126522011191f8d2cc9a3c7adf22b2295f0cbe491226f8bd563689b44647` |
| HLT-042-CI-LOCAL-PARITY | `.github/workflows/foundation.yml` | `sha256:b9e9377cfbd74accd3be0a70bae913c13084682683c972026866e420ce2fa4cd` |
| HLT-042-CI-LOCAL-PARITY | `.github/workflows/foundation.yml` | `sha256:3ca72b43d78f894459032ae79277bd1a4ec73f56aae105552f08ccff9bc38e8c` |
| HLT-034-CI-BAD-BEHAVIOR | `.github/workflows/foundation.yml` | `sha256:8dad8bad78b739db22ccfb0ca756cfc53e374625a9e5309e1cedd2ed1c201afa` |
| HLT-016-SUPPLY-CHAIN-DRIFT | `.github/workflows/jankurai.yml` | `sha256:5d723962d61319b08997b1d4a4053fdd8666e6602bb682944026942da98f455e` |
| HLT-034-CI-BAD-BEHAVIOR | `.github/workflows/jankurai.yml` | `sha256:48a55f18eaecd95e09b7537fbfed29545fd39105b33c01e51b666989016a694e` |
| HLT-007-HANDWRITTEN-CONTRACT | `agent/boundaries.toml` | `sha256:597090ac1c3e503fd3ee8b22c5511acc687a6cdc699cb2c05c1455a7828db8c3` |
| HLT-003-OWNERLESS-PATH | `agent/owner-map.json` | `sha256:967c135cb8c46bf79ed4b1db7bb20a5c7ac77ca176f6ce93196e5bed4a5b6713` |
| HLT-004-UNMAPPED-PROOF | `agent/test-map.json` | `sha256:644f7fde2fd9c012ff91ab4e81fdc034f2850ea588d31bcfb4eaa6697ce6990f` |

Absence means the old exact fingerprint is no longer observed; dimension-score changes are replaced by the current findings below, not declared resolved.

### Remaining target fingerprints

| Finding | Fingerprint |
| --- | --- |
| .: `Code shape and semantic surface` scored 2 below the standard floor of 85 | `sha256:ed90e9b81d323b7b66384f3cd6bca54cc0751ef8ed3bb0096636b7fe26edd7c0` |
| .github/workflows: CI does not run the jankurai audit lane | `sha256:dcdd57510181477041c184ca8e3087ad93965fd92fe3357b1be5d73986e805ae` |
| .github/workflows/foundation.yml: untrusted CI job runs with privileged Docker access | `sha256:289aadc74ca4b9d3d58c90b43932546bfaf2033efca10fb35c2435dff29c5077` |
| .github/workflows/jankurai.yml: `Security and supply-chain posture` scored 60 below the standard floor of 85 | `sha256:7ebd628238a6519f74ee16a89f65c868fbe3782f2aaa14d2a7843e54b2e86b00` |
| .github/workflows/jankurai.yml: untrusted CI job runs with privileged Docker access | `sha256:4498ca8c1256e8e49a97c1a670d3b8e6efb814c937e7badb82d76766fb790a07` |
| .github/workflows/security.yml: untrusted CI job runs with privileged Docker access | `sha256:bb0285ec2b243cdcd8814d700e0a0a7dcbcf27ac532af67e240c995262a29270` |
| agent/boundaries.toml: `Contract and boundary integrity` scored 63 below the standard floor of 85 | `sha256:ed023acdcb8f3c1710aea494ab21c2989c629e6a50074915ca7e12b28a3846ef` |
| agent/owner-map.json: `Ownership and navigation surface` scored 78 below the standard floor of 85 | `sha256:91cfdb02b0a23fe5036eb59a8d50ece86f01196f41dffc4e2d5c76f8c5805f4d` |
| agent/test-map.json: `Proof lanes and test routing` scored 78 below the standard floor of 85 | `sha256:58be05a7ef8771f9dd43f9de38ff8776c926e255df1be4e6778a986dcfbfd5b3` |

## I013 scoped dispositions (18 October 2026)

These dispositions preserve scientific contracts and distinguish detector closure
from technical resolution. No rule is suppressed and no audit threshold is lowered.
The receipt `docs/receipts/i013-rust-ci-20261018.json` binds commands and logs.

| IDs | Inspection, action, and remaining boundary |
| --- | --- |
| I010-J016 | `analysis/__init__.py` is part of the frozen Python inventory. Two additional native CLI ports reduce migration work, but do not retire analysis or close this stack finding. P2/P4 consumer migration remains required. |
| I010-J017 | `analysis/kpi_fitness.py` mixes legacy KPI extraction/fallback behavior. It is outside this packet's implementation paths; changing its fallback policy requires source-bound numerical negatives and the P4 port. Remains open. |
| I010-J018 | SSE data now passes `MessageEvent`/string checks, parses as `unknown`, and narrows with `isRegistryEvent` before use. Tests reject null, arrays, missing fields, string/fractional/negative/nonfinite/unsafe-integer sequences and non-string revoked IDs; accept zero and maximum safe integer. |
| I010-J019/J020 | The cited long JSX line is a browser subject selector, with no shell or SQL execution. Selection now checks membership in validated subjects, then builds navigation with `URL`/`URLSearchParams`. The handler is separate from JSX. Existing encoded navigation and read-only semantics remain. |
| I010-J021 | The scanner's `apps/web` rendered-QA requirement is not satisfied by a TypeScript build. The actual surface is `apps/explorer`; this packet adds runtime boundary tests and runs the UI suite, but does not claim new browser/assistive-technology review. Remains open. |
| I010-J022 | Existing Rust integration/fault tests and new CLI parity controls provide scoped proof. Repository-wide property-test routing/conformance remains a separate requirement; no broad closure claimed. |
| I010-J023 | Detector evidence is the error return `explorer_missing_value_fabricated` in `verify_accessibility_and_truthful_content`, not a claim of human review. That static checker rejects invented missing values and explicitly disclaims browser certification. Preserve the error contract; retain the fingerprint pending detector review. No human-review receipt is fabricated. |
| I010-J024/J025/J026 | Each `unsafe` block now records exact local preconditions: C ABI/static signal handler using only a lock-free atomic store; owned live descriptor and valid flock flags; unlock before File drop. New lock test covers contention, failed open, release, reacquisition and a second contention. Existing lifecycle test exercises SIGINT cleanup and worker failure/replay. |
| I010-J027 | The original fingerprint cites `bounded_wait_drains_small_pipes_before_child_exit`. Its `--exact` subprocess is a deliberate self-reexec fixture, not a focused top-level test run. It forces 4096-byte pipes and asserts successful exit plus both full output streams. Keep that deadlock regression and its bounded timeout; the new lock regression adds failure/cleanup coverage. |
| I010-J028 | Both LengthUnit enums serialize metre/millimetre, but live in separate identity systems with different variant ordering/trait surfaces. Consolidation needs shared-contract owner approval, dependency review and canonical/hash regression coverage for both consumers. No rename or frozen identity change in I013. |
| I010-J029 | Contract MaterialState uses `Identified { passport: BlobDigest }` / unit `Unknown`; evidence MaterialState uses `ContentRef` and `Unknown { reason }`. Unification would alter wire shape and evidence requirements. Blocked on an explicit versioned migration. |
| I010-J030 | Geometry AttemptOutcome records artifact acceptance plus error strings; runtime bench AttemptOutcome records qualified/rejected/timeout outcomes without those payloads and with snake_case serialization. Different state machines; preserve both until a domain-qualified API migration. |
| I010-J031 | GPU Admission carries reasons or a judge identity; evidence decision Admission is SandboxOnly/AdmittedForEvidenceReview. They are not interchangeable gates. No shared enum without a redesigned contract. |
| I010-J032 | Render EvidenceClass distinguishes measured/model/synthetic/illustration availability; search EvidenceClass distinguishes exploration/screen/qualified/failure admission. Consolidation would conflate presentation provenance with search eligibility. |
| I010-J033 | Render Disclosure is PublicResearch/PrivateGeometry; store Disclosure is PublicCandidate/Private. Public-candidate storage is not reviewed publication authority. Retain the explicit conversion boundary; a shared enum would require policy and wire migration. |

The new native validator retains the canonical `todo` status string. A dead-marker
finding on that literal is a detector limitation, not permission to rename a frozen
progress state. General malformed-JSON and OS error wording remains native to each
language; tested semantic failures and supported D28 controls have exact output/exit
parity. Existing Python entrypoints remain for legacy consumers.

### I013 measured re-audit

`jankurai 1.7.5`: **55 score / 55 raw**, **14 caps**, **57 findings**,
**36 conformance blockers**. Advisory threshold 45 passes; conformance does not.
Report SHA-256: `4de467b2e5b9dcefd607428e7c8b260057f0c5543307bcaf1ad16511b17e788e`.
The complete report and failed/successful logs are retained in the private I013
evidence archive. `.jankurai/` scratch was moved there before committing.

| ID | Historical fingerprint | Current scan |
| --- | --- | --- |
| I010-J016 | `sha256:e3de85f3144b1d5a9831cce0a0528dfdf63804b2a2c34c62e86fea281f63e9d1` | present |
| I010-J017 | `sha256:6544b82d5044a87069667f774e4cad6ac2c980bc8f0c24d17eec6949e1de760e` | present |
| I010-J018 | `sha256:abe7d03a3432e9df83f076bf84dd0987ebe20f600072f7a086a077ab5f25eb6e` | absent |
| I010-J019 | `sha256:020a52c1d53c4c876af9365262e36dfdc99423eebdbde62d3a0ad3f6ab76889c` | absent |
| I010-J020 | `sha256:2a3d89e192ea6247ffaa27249d827cb9aa939b5ee75ccb2466c37c80997ff517` | absent |
| I010-J021 | `sha256:571d35c2e730a393b782bac14825b197c0543920bb21967079d264ac602ea5b1` | present |
| I010-J022 | `sha256:8ece7234070a20910736663e65a530625acd16dac7fa57476cfc7c9a74bd745c` | absent |
| I010-J023 | `sha256:a57016dbcde35220ab3f3c167b4c2c59c00ff50130ee6ffa3080c9894ff15689` | present |
| I010-J024 | `sha256:b75a06daa9b211224f9b57cb3038ba6029ce99aff44a7d13c34b25063c0959c7` | absent |
| I010-J025 | `sha256:1d37e5eb654abd16bea4f38678e03f240bdc6fe6d065a1ec19293dae0755703e` | absent |
| I010-J026 | `sha256:b75a06daa9b211224f9b57cb3038ba6029ce99aff44a7d13c34b25063c0959c7` | absent |
| I010-J027 | `sha256:4fbefe565f3248efb31ffc389c4cf29e39711c9daf396d96a7e22963f8bcfcfa` | absent |
| I010-J028 | `sha256:5902e53ae4fd5d10052e6bb556142ff7dd48be86bd5292c4b7eb9664f380de02` | present |
| I010-J029 | `sha256:8be4ab72acf7fcca8056149c8c4ca26b03328de6f984ce943f94891f37cfd5e8` | present |
| I010-J030 | `sha256:5e32ae5a903307c9f0d251af2b14406971a1eb0e3787386f245018527b48193c` | present |
| I010-J031 | `sha256:caa1ac9db64ce14b3a9063b592a1b0193c8a12d9eabc596c6e65736b65361c0f` | present |
| I010-J032 | `sha256:e8f3c394d9dfef3bbe6483aa7bfbbaa0cdaab1de3e45f940425f8bb712665501` | present |
| I010-J033 | `sha256:5397fac53d136d19a1a64647a144755a80f58329e3b33b939c292c3fe6e07af8` | present |

Absence is exact detector-fingerprint closure only. J027 was already absent in
the I012 report; the pipe-drain regression remains intact. J024/J026 share one
fingerprint because the detector normalized both block openings identically.
The new native-validator `todo` finding is
`sha256:741fe87f3f83c6741de5817a0d6c0bcbe6a2cf601e7ff6a1225bf9aba1e74dc8`;
the wire status is intentionally retained. Existing dossier-parity test text also
remains flagged (`sha256:be881250e51bef76182fbf0a09685c286fed3477e673a7bb5be90c18ac3eb277`).

I013 verification: the full Foundation gate passed all 10 steps (Rust suite
576.567 s; entire gate 754.160 s), along with the requested parity, security and
canonical checks. An earlier standalone CLI lifecycle run timed out; its failed
scratch and log remain archived. The same lifecycle tests, including the new
lock cleanup check, passed in the full workspace run. No timeouts or acceptance
thresholds were relaxed. This is software verification only.


## I014 runtime safety review (I010-J034–J054)

All 19 `lease.rs` unsafe blocks and both `task.rs` blocks now have local
`// SAFETY:` arguments. Runtime behavior and acceptance thresholds are unchanged.

| Historical IDs | Operation and precondition / failure handling |
| --- | --- |
| J034–J036 | `current_affinity`: zeroed integer bitset, exact writable buffer size, checked syscall return, CPU index strictly below CPU_SETSIZE. |
| J037 | `still_matches`: pointer-free getpgid query; boot/start/group comparisons reject replacement and syscall failure. |
| J038–J041 | Recorded cancellation: range-checked PID, fresh pidfd transferred once to File, live pidfd for signal/poll, null siginfo and zero flags. Identity mismatch exits before signaling; RAII closes the descriptor on every error. ESRCH means already exited; EINTR retries only within the poll deadline. |
| J042–J044 | Spawn: valid zeroed affinity mask, checked CPU_SET index, stack-owned affinity/rlimit buffers in the pre-exec closure. No allocation or locks after fork; failed native setup aborts exec. Output files and Command-owned handles close on errors. |
| J045–J046 | Poll: initialized status/rusage output buffers and an owned unreaped child PID. WNOHANG/EINTR do not decode output; successful wait sets reaped before fallible usage conversion, preventing later signaling after a decode error. |
| J047–J049 | Cancel/Drop: verify the live owned group before signaling; reap polling has a deadline. Drop uses a nonblocking best-effort wait only: it does not certify reaping or release a durable reservation. An unresolved child remains an explicit recovery limitation. |
| J050–J052 | Process audit: initialized integer rlimit output, checked getrlimit return, pointer-free getpgrp/getpid queries. |
| J053–J054 | Thread accounting: valid zeroed rusage output, exclusive mutable pointer and RUSAGE_THREAD; syscall failure precedes arithmetic, negative/overflow totals fail conversion. |

Existing runtime integration controls exercise bounded affinity/resources, expiry,
GPU/rank/SMT rejection, successful/numerical/unsupported/timeout accounting,
cancellation that leaves a second process group alive, pidfd rejection of a changed
start time without signaling, crash reservations, and actual checkpoint kills.
These tests and local arguments are software evidence, not proof of arbitrary
process-tree containment: recovery supports the single-process fixture only.

### I014 measured re-audit

`jankurai 1.7.5`: **55 score / 55 raw**, **15 caps**, **42 findings**,
**21 conformance blockers**. Advisory threshold 45 passes; conformance does not.
Report SHA-256: `f10a9d98b9409cdffcf209dbd26656e81499f34d2a1e9f911387c4a7db521c4f`.

| ID | Historical fingerprint | Current scan |
| --- | --- | --- |
| I010-J034 | `sha256:83821d7963d62afdab5e3bb375c8f937ea5052e065051fc7888f708c5d018560` | absent |
| I010-J035 | `sha256:1dc2c0f2272ea43f08aca79d5dfb5c0d467605cb886059e43f93512db2c6460b` | absent |
| I010-J036 | `sha256:441896ca75260bfd1562f03d3174e4454fc29dfa8181f79e273780b146e25d13` | absent |
| I010-J037 | `sha256:363e170b49b56489c80d817d8da4ff61e8b5fc4bd891157d1340f120071a895d` | absent |
| I010-J038 | `sha256:3e8bc6dd321735a036d584570d5980b88d86b5c21b7d39aedcab2e69c648725a` | absent |
| I010-J039 | `sha256:8fabbd27e8b9d1410eee4bd9eee12aaf7f7087b726935fcb18b10ba959418e53` | absent |
| I010-J040 | `sha256:c27b903072546b8d8300e7a46e54209775465a0665a4288b86695736d9b34d65` | absent |
| I010-J041 | `sha256:7e7cc5e7267c43c5ffd9e2e6793fc22f8a21d30b2a69002c7a82f4b4b3b6bc1b` | absent |
| I010-J042 | `sha256:d0430e036a04313a022ecc83106da75695b5bd7596ad66f0a33e59a279b4cc6f` | absent |
| I010-J043 | `sha256:3448b193bd118030846ed089f30cf02490550a3b7e9a8baea55ad256fb1d2009` | absent |
| I010-J044 | `sha256:fa830e0fb294867d66cfd061494b365bf80e2c4ef1a7792ac05725d2c143ab44` | absent |
| I010-J045 | `sha256:4dc5e175a6d45e5db5115d3efacd8ad90d947e6c0c7352d5c21b953013243187` | absent |
| I010-J046 | `sha256:0d24242ce6d0d328168e7f38dfb4b4c72c470c4a87fd6411206e5121e8b5ab14` | absent |
| I010-J047 | `sha256:6c198cd2b5924ce1f3437a9a691023f82afd20d3f5e7cda0091948ded4f1b053` | absent |
| I010-J048 | `sha256:aa4fdea5725515c1910e667ebfd6f4430104bd5a3e458c3eccff1f0c1d2dc4e1` | absent |
| I010-J049 | `sha256:da63a99a2940d25e755edfda52bb3daea6b6eac22447dd639a2292c84f9cb825` | absent |
| I010-J050 | `sha256:836dd6ab8d0adbc4fff60a72b6906a3de6f6a3faac6fd3cd43d25cfeb4fdbf77` | absent |
| I010-J051 | `sha256:d22ec739962f2eb7f61ab5a32785115b992e4c13c5eb2a65f74bfb43686841d6` | absent |
| I010-J052 | `sha256:38986527653efc6a68af5ec50db516c56f907241a3c1608514cc634b0d5795d9` | absent |
| I010-J053 | `sha256:cd7a8e8451f7bf96706e3d986782dfd2061ce596fc1f8e64fe11dd115285ca91` | absent |
| I010-J054 | `sha256:77574c8e1b49d3c52a137a22ee19efdbfcaf2bdaed6bb00715f2a2171551cc25` | absent |

All 21 historical undocumented-block fingerprints are absent. The scanner now
reports five separate `rust.unsafe.zeroed` findings at the same valid C integer
buffer initializers; these remain open detector findings despite the local validity
arguments. The sixth new finding flags the required `TODO.md` output filename
as a placeholder. The filename is part of the byte-parity contract and is retained.
No suppression, wire rename, zeroing rewrite or broad conformance closure is claimed.

Retained caps: `python-direct-product-truth-or-db-ownership`, `no-jankurai-audit-lane-in-ci`, `non-optimal-product-language-found`, `vibe-placeholders-in-product-code`, `fallback-soup-in-product-code`, `future-hostile-dead-language-in-product-code`, `missing-rendered-ux-qa-lane`, `false-green-test-risk`, `input-boundary-gap`, `release-readiness-gap`, `no-agent-friendly-exception-pattern`, `rust-bad-behavior`, `python-bad-behavior`, `ci-bad-behavior`, `repo-rot-bad-behavior`.

The raw report and logs are retained in the private I014 evidence archive;
`.jankurai/` scratch was moved there before the evidence commit. Exact command
results, source hashes and limitations are in `docs/receipts/i014-rust-ci-20261018.json`.

I014 verification: all ten requested standalone checks and the full Foundation
gate passed. Foundation completed all 10 steps, including workspace Rust tests
(484.855 s), Python regressions (129.805 s), publication/claims checks and UI build.
The initial Clippy failure for redundant renderer string conversions is retained;
those conversions were removed before successful checks. No runtime thresholds
or scientific acceptance rules changed.
