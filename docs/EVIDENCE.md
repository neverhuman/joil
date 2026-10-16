# Evidence, identity and claims (as implemented)

This file describes the evidence kernel that is merged on `feat/joil-v4-truth-evidence` (EV-01–EV-05, EV-07,
EV-08, EV-09, EV-11). It was written and reviewed by the DOC lane (DOC-07) from the code. The crate's own
summary is [`crates/joil-evidence/README.md`](../crates/joil-evidence/README.md), and its provenance from
`neverhuman/jarambit@c988273` is pinned in [`crates/joil-evidence/UPSTREAM.json`](../crates/joil-evidence/UPSTREAM.json).
If this file and the code disagree, the code is authoritative.

The kernel answers the reviews' request for exact identity, typed observations, a claim DAG with revocation, and
generated status (S1 §11, S2 §16, S3 §9, S5 §17, S6 §12). It proves *identity and closure*, not truth: content
hashes show which bytes were used, not that they are right. The application must supply trusted review
authority, and missing independent evidence stays `Incomplete`.

## 1. Identity (`crates/joil-evidence/src/identity.rs`, `subject.rs`)

- **`ContentRef`**: id, revision and SHA-256 of exact bytes. Decoding is strict: duplicate and
  Unicode-NFC-colliding keys are rejected and numbers use an exact binary64 canonical wire. Runtime metadata (host,
  thread count) is kept outside scientific identity.
- **`ScientificIdentity`**: binds the subject to its requirements, compiler, evaluator, scenario, material state
  and dependencies. A different compiler or tolerance gives a different subject identity (EV-02 test
  `two_compilers_distinct_subject_ids`).
- **`SubjectBundle` and `stage_identity`**: separate genome, IR, geometry, mesh and case hashes, each in its own
  domain-separated namespace, so that one stage's hash cannot pose as another's.
- **`Lineage`**: parents, operator ID and version, seed, RNG span, repair records and compiler reference for
  every individual. Wiring it into `population.rs` is TR-13, which is still in progress.
- **`CaseManifest`**: binds an OpenFOAM/FEA case to compiler, IR, geometry, both meshes, solver, model, scenario,
  requirements, objectives, material/process and extraction context. `verify_artifacts` checks exact bytes; a
  missing mesh or case identity fails closed. The case *writer* (`cfd_case.rs`, IR-owned) and the historical run
  manifests (INT-owned) have not yet been migrated (EV-12 is BLOCKED on that; synthesis DOC-F06 shows the
  current manifests are copies).
- **`sample_identity(campaign, seed, index)`**: Monte Carlo sample identity that does not depend on worker or
  thread count.

## 2. Observations (`observation.rs`; Python mirror `analysis/evidence.py`)

An `Observation` is one of the following states. None of them is converted to zero, the baseline or a default:

| State | Meaning |
|---|---|
| `Measured` | A finite value with unit, quantity, frame, region, recovery method, fidelity, tier, origin, scientific identity, source evidence, applicability and at least one uncertainty component |
| `Failed` | Classed failure (`geometry`, `numerical`, `infrastructure`, `out_of_domain`) with reason and diagnostics. It has no value field, so it cannot become a performance number; TR-17 routes the OpenFOAM read path through it |
| `Unknown` | Not evaluated, with a reason |
| `NotApplicable` | Outside the observable's scope, with a reason |
| `Insufficient` | Evidence exists but is not enough; must list what is missing |

`Measurement::check` enforces the evidence ladder:

- Tier and fidelity must match: `geometry` ↔ `F0`, `analytic_screen`/`coupled_screen` ↔ `F1`,
  `numerically_verified` ↔ `F2`, `model_challenge` ↔ `F3`, `physical_validation` ↔ `F4`.
- `synthetic` origin can only be `software_contract`.
- Tiers from `numerically_verified` upward need numerical state `verified`.
- `physical_validation` needs a `measurement` origin, `F4`, a `Specimen` subject and a known material state.

The tier list (`software_contract`, `geometry`, `analytic_screen`, `coupled_screen`, `numerically_verified`,
`model_challenge`, `physical_validation`) is the vocabulary that `tools/claims_audit.py` uses for its wording
rule (DOC-08). [`crates/joil-evidence/schemas/observation-v1.schema.json`](../crates/joil-evidence/schemas/observation-v1.schema.json)
is the shared strict interchange shape. Rust and Python round-trip it and reject the same planted mutants (EV-11).

## 3. Claims, review and revocation (`claims.rs`, `lib.rs`)

- **`ClaimInventory`**: a frozen list of criteria (observation reference, quantity, unit, minimum tier and
  limit) plus dependencies on other claims, under one `ScientificIdentity`.
- **`Registry::resolve`**: walks the claim DAG. Every observation must exist, match its reference byte for byte,
  carry the inventory's identity, and have all referenced artifacts present. Results combine with precedence
  `Revoked` > `Invalid` > `Reopened` > `Fail` > `Incomplete` > `Pass`. A missing observation gives `Incomplete`,
  never `Pass`; a revoked dependency revokes every downstream claim (EV-05).
- **Review events** are append-only and hash-chained (`Event`, `verify_event_chain`). The application must
  authenticate the approving authority; `ApprovedInventory` cannot appoint itself.

## 4. Requirements, campaigns and generated status (`campaign.rs`, `tools/gen_status.py`)

- [`requirements/joil-hydrocyclone-v1.toml`](../requirements/joil-hydrocyclone-v1.toml) is read by
  `campaign::Requirements`. Its five objectives preserve the Wave-2 preregistration, and
  `verify_executed_objectives` rejects a campaign whose executed objective hash differs (S4 §2.6).
- The contract admits **sandbox** campaigns only. Pressure, energy and absolute-pressure limits, the continuous
  duty band, material/process data and independent validation inputs are missing. Claim-bearing admission
  therefore fails even with passing P0 receipts (EV-06 is BLOCKED on those external inputs; workplan §5
  decision 9).
- [`docs/STATUS.json`](STATUS.json) is generated by `tools/gen_status.py` from the evidence registry, and the
  generator's check rejects hand edits, objective drift and stale status (EV-07). The pre-v4 hand-written status is
  kept only as a historical record. The README status table is rendered from `docs/STATUS.json`, and the claims
  audit fails if it is stale (DOC-05).

## 5. Surrogate training admission (`training.rs`)

`admit_sample` needs a reviewed closure for every label and observed feature, a byte-pinned feature definition and
reviewed observer-derived geometry and mechanism groups. Prior/analytic and synthetic targets are rejected. No
independent $C_{\text{out}}$ dataset exists, so `crates/joil-evidence/reports/surrogate-independent-ood-v1.json`
reports insufficient evidence rather than out-of-distribution scores (EV-09). A consequence for the search code:
`joil-cli wave3-proof` now stops at the surrogate fit (an INT-04 decision, see the README quick start).

## 6. Review notes (DOC-07, 2026-10-16)

| # | Observation | Effect | Suggested follow-up |
|---|---|---|---|
| R-1 | `Observation::compare_at_most` decides `Pass`/`Fail` on the point value. The required `uncertainty` components are not used. | A criterion `≤ 30 mg/L` passes at `29.9 mg/L` with ±2 mg/L uncertainty. The reviews ask for conservative-bound decisions: pass when the adverse bound passes, fail when the favourable bound fails, otherwise indeterminate (S4 §10.3, S6 D9). | EV: add a declared combination rule and an indeterminate/`Incomplete` band; do not root-sum-square correlated components (`docs/SCIENCE.md`). |
| R-2 | `Criterion` expresses upper limits only (`at_most`). | Lower-bound requirements (capture fraction, treated-capacity gain with LCB > 0) can only be written by negating the quantity, which hides the sense of the requirement. | EV: add a `sense` (at most / at least) to `Criterion`. |
| R-3 | `Fidelity` has `F0`–`F4`. | The workplan ladder is F−1…F5, and `F2-carrier` and `F2-Oil` are distinct evidence classes in `docs/SCIENCE.md`; both currently map to `F2`. | EV: decide whether `F2` needs carrier/oil sub-levels before `F2-Oil` evidence exists. |
| R-4 | Content hashes and closure checks are sound; identity cannot be upgraded by a caller. | No issue found in the identity, revocation or tier/fidelity rules. | none |
