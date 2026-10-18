# Joil v5 engineering workplan

Revision 1, 17 October 2026, America/Los_Angeles. Reviewed code baseline: `cc86edc90d06f0eac69022f84354abc94bb453c4`. This is the detailed execution annex of `docs/MASTER_PLAN.md`, not a competing master plan. It expands P00–P10 into V5-00–V5-31 without changing historical packet completion or admitting a physical campaign.

The current deliverable is a planning package: source reconciliation, 32 worker packets, 160 implementation tasks, progress tooling and one draft PR. All 160 engineering tasks begin `todo`. The planning receipt proves only preparation and checks of this package. It does not implement the future engine or qualify geometry, CFD, oil grade, structure, materials or physical performance.

## 1. Objective and boundaries

Build a restartable scientific design engine that develops variable-topology programs into one bounded physical subject, evaluates that subject through blind, independently qualified judges, and searches a constrained mass/reserve/capacity/energy trade-off under complete resource and evidence accounting. Joil remains the hydrocyclone domain pack; a self-contained structural bracket pack proves software reuse.

Success has four independent meanings:

| Release | Required evidence | Completion cannot be inferred from |
| --- | --- | --- |
| R1 software engine | Fresh clone, nonempty bounded fixture campaign, charged attempts, verified artifact closure, real kill/resume/fork/seed behavior, offline dossier and complete recipe registry | A compatible checkpoint struct or empty-island self-test |
| R2 computational comparator | Independently observed geometry, verified native numerics, validation by observable within an explicit coverage domain, applicable materials/loads, exact same-duty comparator and decision uncertainty | Green CI, an expensive solver label, matching pressure alone, or model-to-model agreement |
| R3 discovery method | Prospective representation/search controls at fair charged cost, competent conventional comparator, held-out confirmation and measured novelty/mechanism scope | A strange shape, authored recipe activation, one lucky seed or a large evaluation count |
| R4 physical candidate | As-built identity, independent builds/preparations, calibrated blinded paired tests and all mandatory application-scoped gates | A simulation, render, material name or research quality threshold |

Non-goals: work in another repository; modifying jarambit; a pressure-vessel design; industrial installation authorization; proving universal sour-service or regulatory eligibility; guaranteeing a positive discovery; universally resolving individual micron-scale interfaces; purchasing compute, prints or lab work; external messages; patent/legal conclusions; adopting all proposed new solvers at once. The plan does not authorize a new allocation or activate subagents. Future parallel execution needs the project’s usual named ownership acknowledgement.

Assumptions: original source files remain privately available; the public CT-35 reconstruction is the reproducibility baseline, not a current commercial optimum; reference work starts from the pinned native CPU environment; dilute isothermal oil-in-water is an initial model scope requiring explicit checks. All example tolerances, material values, loads and time/cost estimates remain proposals unless their authority is recorded as admitted. Missing external inputs restrict their dependent claims and do not block independent software fixtures.

## 2. Ordered reads and exact working locations

The requested destination is `/home/ubuntu/joil` on `xbabe2`. The existing Mac mirror is `/Users/bentaylor/joil`. The planning authoring checkout is `/Users/bentaylor/Documents/ChatGPT/joil/engineering-plan`; its single integration branch is `codex/v5-engineering-plan` from the reviewed main commit. Keep the older Mac branch and its untracked prototypes intact. Use the plan package mirrored into that checkout without replacing unrelated files.

Every worker reads, in order:

1. `AGENTS.md`, root `COORD.md`, `docs/COORDINATION.md`, current ignored `.private/coordination/` notices. Check current HEAD, index and dirty paths. The remote root coordination file remains authoritative until an acknowledged migration replaces it.
2. `docs/MASTER_PLAN.md`, `docs/SCIENCE.md`, `docs/WORK_PACKETS.json`, generated `docs/STATUS.json`. The legacy status embedded inside STATUS is explicitly historical.
3. This file, `SOURCE_REVIEW.md`, `INPUTS_AND_RISKS.md`, `VALIDATION.md` and `PR_HANDOFF.md` in this directory.
4. `TASKS.json`, the selected `todos/V5-XX.md`, its `progress/V5-XX.json`, dependency receipts and every packet-specific `read_first` path in the listed order.
5. Frozen baseline, scenario, requirement and model records that the packet consumes. Read actual implementation symbols; source-spec line numbers are clues tied to `cc86edc`, not durable references.

Paths in packet files are repository relative and resolve below the selected checkout. New crates, commands and schemas are explicitly future deliverables. A validation command targeting one cannot run until the corresponding package/CLI operation is registered. Existing checks remain available now: `rtk proxy python3 tools/check.py`, `rtk proxy python3 tools/check_public.py` and `rtk proxy python3 tools/gen_status.py --check`.

## 3. What the source study changes

`SOURCE_MANIFEST.json` identifies all ten supplied inputs, including the 38-page PDF, by exact SHA-256. `SOURCE_REVIEW.md` records what is adopted, deferred or corrected; `SOURCE_COVERAGE.json` indexes source sections to work packets. `DEFECT_MAP.json` maps all 31 detailed source allegations to implementation tasks and current evidence class. The original files are retained in ignored private storage, not copied into the public PR.

No filename or self-score makes a specification authoritative. The two xfinal files disagree on substantial matters. This workplan uses the detailed S09 catalogue and operational requirements, the corrective S10 two-track design, and the S02/S03 treatment of statistical inference, physical applicability, durable accounting and independent evidence. The shorter briefs supply product intent rather than numerical qualification.

Decisions adopted for implementation planning:

- One material-labelled physical partition and bounded derived artifacts; no independently filtered fluid and solid authorities.
- Judges receive physical artifacts, approved scenarios/materials and contract-level tags. They cannot reach genotype, expected mechanism or lineage through imports or serialized side channels.
- Mature corrected native CFD and structural adapters establish the reference first. Custom GPU LBM/FEA and a frozen-liner response operator are optional admitted accelerators, not prerequisites for all useful work.
- PDR is observed under a declared bounded split controller. The existing anomaly warrants a controlled study; negative gauge pressure and a commercial range from another liner do not prove the cause or define CT-35 acceptance.
- Mass, common-demand structural reserve, common-flow compliant treated capacity and common-duty mechanical energy are distinct bounded objectives behind hard gates. The proposed 0.35/0.20/0.30/0.15 preference is a versioned sensitivity profile, not a license to trade feasibility.
- Start with an executable fixed `reference-search/1` policy. Bandits, learned descriptors, motifs and neural fields enter separately tested policy versions after the reference works.
- Separate planning precision from inferential qualification; weighted/clustered oil statistics from Bernoulli toy counts; logical replay from native rerun and actual resource costs.
- Preserve all historical observations and corrections. New interpretation or extraction creates a new identity; old bytes are never rewritten to match the repaired model.

## 4. Architecture and integration contracts

The existing `joil-evidence` crate remains the one semantic canonicalization/evidence kernel. Avoid moving it merely to change branding: a name-only rewrite increases risk without improving science. Add domain-neutral `crucible-*` crates incrementally. The integrator registers their workspace membership, package tests and dependencies before packet commands run.

| Boundary | Planned owner and paths | Responsibility |
| --- | --- | --- |
| Units and contracts | V5-03; `crates/crucible-units/`, `crates/crucible-contract/`, `schemas/v5/` | Typed quantities, strict schemas, restricted expression AST, subject view, outcome semantics |
| Evidence and inference | Existing `crates/joil-evidence/`, V5-02/04 | Identity, claim admission, DecisionProfile, experiment-family error policy, independent statistics |
| Private objects | V5-05; `crates/crucible-store/` | Immutable CAS, dependency closure, cache namespaces, revocation, retention, disclosure classification |
| Runtime and CLI | V5-06/07/23; `crates/crucible-runtime/`, `crates/crucible-cli/`, `joil` | Transactional attempts, leases, reservations, checkpoints, replay and workers |
| Physical geometry | V5-08/09/10; `crates/crucible-geom/` | Partition, bounded field interpreter, independent artifact observer, surfaces/tets/exports |
| Representation | V5-11/12; `crates/crucible-genome/`, `packs/hydrocyclone/modules/` | Typed MPG, gas-metered development, legacy lowering and operators |
| Native reference | V5-14/16/19; `crates/crucible-adapters/`, `crates/crucible-particles/` | Qualified process adapters, carrier/oil/structural production fixtures |
| Domain meaning | V5-13/15/18/20; `packs/hydrocyclone/{judges,materials}/` | Material/process, observables, common-flow maps, objective/gate policy, validation |
| Search | V5-21/22; `crates/crucible-search/`, `policies/` | Fixed reference policy, measured QD, fair controls, acquisition and audit stream |
| Optional acceleration | V5-24/25 | GPU kernels and reduced tail operator, each with reference fallback |
| Presentation | V5-26/27/28; `crates/crucible-render/`, `render/blender/`, `apps/explorer/`, `crates/crucible-paper/` | Evidence-bound scenes, read-only explorer, exact claim queries |
| Independent reuse | V5-29; `packs/bracket/` | A second domain without engine edits or work in another project |

Each new crate uses a normal `Cargo.toml` and library/binary entry point. Domain judge paths listed as `packs/hydrocyclone/judges/*.rs` form a package rooted there with explicit `[lib] path = "lib.rs"`; tests live under `judges/tests/`. The integrator may choose conventional `src/` layout once, but must update registry paths/commands before dispatch. Root architecture tests live in a real package test target, not an unregistered virtual-workspace `tests/` directory.

The conceptual execution pipeline is:

```rust
let developed = developer.develop(&genome, &interface, &budgets)?;
let subject = compiler.compile(&developed.partition, &realization_policy)?;
let geometry = independent_observer.observe(&subject, &geometry_request)?;
let prepared = evaluator.prepare(&subject.restricted_view(), &scenario, &capability)?;
let raw = runtime.execute(prepared, &current_lease)?;
let observations = evaluator.extract(&raw.verified_artifacts(), &subject)?;
let decision = policy.decide(&observations, &requirements, &decision_profile)?;
```

These signatures are guidance, not current compiling APIs. Serialization and crate graph jointly enforce the boundary. A renderer may read an origin-trace sidecar; the physical evaluator may not. Near-duplicate geometry fingerprints can order acquisition but cannot authorize a cached evaluation. Cache identities include the actual derived artifact/tolerance and case settings, not only an IR hash and generic level.

## 5. Ownership and conflict policy

One integrator owns `Cargo.toml`, `Cargo.lock`, all root dependency/container manifests, schemas, requirements, frozen baselines, generated clients, shared public types, scientific status generation and this plan registry. Workers request a contract change through a small reviewed integrator commit before changing consumers. They do not edit root files opportunistically.

Packet `write_scope` is an exclusive allowance, not a suggestion. A worker reads other packet code but never edits it without transferring the named path. Parent scopes such as `crates/crucible-geom/` overlap every child path: V5-08, V5-09 and V5-10 may overlap only after the integrator assigns the individual listed modules and confirms library exports are serialized. V5-01 touches legacy monoliths and must land before later consumers; do not run multiple writers in `evolve.rs`, `joil-ir/src/lib.rs`, `main.tsx`, or root JSON files.

Forbidden/unpublishable zones: another project tree/process/environment; `.private/`, `runs/`, `artifacts/`, host inventory, credentials, customer documents, raw papers and licensed standards. Raw solver outputs, candidate genomes/IR/meshes and internal-design renders belong in the private store by default. The publication scan must never be weakened to get a PR through. Generated meshes, fields, clients, lockfiles and frozen holdouts are not manually edited. Existing receipts are append-only: corrections create new records.

This request does not dispatch agents. If the owner later authorizes parallel execution, these are candidate disjoint work windows:

| Prerequisites | Potential packets | Hard serialization point |
| --- | --- | --- |
| V5-02/03 contracts committed | V5-04 statistics; V5-05 store; V5-08 kernel prototype | Workspace and shared schemas remain integrator-only |
| V5-08 partition contract committed | V5-09 `src/observe/**`; V5-10 `surface.rs`, `tet.rs`, `export.rs`; V5-11 separate genome crate | `crucible-geom/src/lib.rs` and contract edits serialized |
| Accepted geometry and native contracts | V5-14 carrier; V5-13 passports; V5-06 runtime | No shared numerical fixtures or manifests without explicit split |
| Carrier evidence and native fixtures | V5-16 oil; V5-19 structural mapping; V5-26 scene contract | `crucible-adapters` shared exports/configurations serialized |
| Frozen scene/API contracts | V5-27 UI; V5-28 paper; V5-29 bracket | Integration of generated TS and capability registry serialized |

No other parallel packet pairing is implied. Shared-path overlap defeats the pairing even if workers believe their edits are small.

## 6. Dependency order and milestone exits

`TASKS.json` is the canonical ordered packet/task registry. `todos/` and `dashboard.html` are generated views. Dependencies are packet-level gates; tasks inside a packet run in listed order unless a worker documents a safe independent substep. Dependencies mean accepted packet evidence, not merely all checkboxes being checked. A software packet can close without a physical claim; a physics packet must state its exact capability and remaining unknowns.

| Milestone | Packets | Exit artifact and failure interpretation |
| --- | --- | --- |
| M0 truth/contracts | 00–03 | Reproduced defect register, corrected current claim semantics, unresolved-limit authority and strict blind contracts. Missing input blocks affected admission, not all development. |
| M1 durable fixture | 04–07 | Verified statistics, immutable object closure, fenced attempt ledger, nonempty kill/resume/fork/seed fixture. A compatible struct is insufficient. |
| M2 geometry/genome | 08–13 | CT-35 bounded round trip, artifact-only negative corpus, shared fluid/solid realization, MPG operators and qualified process-status cards. Unsupported active geometry blocks its consumers. |
| M3 reference science | 14–17,19–20 | Native carrier/oil/structural fixture suites, source accession and validation coverage. Missing validation keeps affected results exploratory even if numerical tests pass. |
| M4 decisions/search | 18,21–23 | Common-flow/gain semantics, four bounded objectives, fair fixed search, random audit and measured leased runtime. No physical ranking before corresponding qualification. |
| M5 acceleration/views | 24–27 | Optional accelerator/operator admission or explicit reference fallback; complete evidence scene registry and read-only UI. Recipe coverage does not mean evidence/render availability. |
| M6 research/reuse | 28–29 | Prospective study and exact-query paper, valid negative outcome path, independent bracket pack and offline reproduction. |
| M7 confirmation/release | 30–31 | As-built/rig analysis when authorized; exact-head release inventory and one reviewable PR. R1–R4 assessed independently. |

Critical path to an R1 fixture is 00→02→03→05→06→07. Critical path to qualified search includes accepted geometry, reference validation, full oil transport, material/load/structural evidence, and V5-18 metrics before V5-21. V5-24 and V5-25 are optional branches and do not appear as mandatory V5-31 dependencies. Their absence needs a recorded reference-only disposition, not fabricated completion.

Durations in supplied specifications vary from roughly thirty software weeks to longer physical programmes. They are not adopted as delivery promises. Re-estimate after M1 storage/recovery measurements, M2 compile/mesh yield and M3 native point cost. A complete trustworthy baseline has priority over broad parallel solver construction. Laboratory/measurement/data access has external lead time; record it immediately without assuming permission to book or buy.

## 7. Hard implementation details and edge cases

### Physical geometry

Use the public baseline’s original dimensions and region contract. A byte-identical genotype round trip and numerical agreement between approximate SDF and B-rep are different acceptance tests. Do not claim identical IR bytes across different canonical expressions merely because physical shapes appear equivalent. A shared material partition must prevent fluid/solid overlap after all conformance operations. Finite-grid re-distancing carries an error estimate; ambiguous interval cells or underresolved features remain unresolved. A theoretical tet-quality theorem does not certify projected P2 elements or a modified implementation. Minimum geometric bounds and topology must be checked after extraction.

Topology fixtures distinguish volume and boundary homology. A volume ball has `(1,0,0)` and a solid torus `(1,1,0)`; their boundary surfaces have different tuples. Compare volume complexes with volume complexes, and boundary quantities separately. A connected powder void can still be inaccessible at the required escape radius. Every active module must survive into the accepted geometry or be classified Neutral/Unsupported with no score credit.

### Reference flow and oil

Every comparison uses the same compiler, mesher, boundary/controller/tap policy, solver and extractor identities, or explicitly quantifies cross-backend offsets. Set inlet flow/profile and one outlet pressure; solve bounded reject backpressure. Freeze settled settings for sampling unless a controlled-system duty is declared. Reconstruct absolute pressure from dimensions, gravity and datum before cavitation/gas checks. Investigate multiple stable states and startup hysteresis rather than selecting the favorable branch.

No fixed iteration count, cell count, convergence ratio or small residual proves adequate CFD. Measure mesh/time/corrector/wall/domain and statistical contributions against the intended decision margin. GCI/Richardson require appropriate observed convergence; an oscillatory span is a diagnostic bound, not automatically a verified error certificate. Validation is scoped to observable, mechanism, geometry and conditions. A literature curve with missing raw metadata stays a comparison.

Oil injection uses inward normal face flux, area, time and concentration; represented mass is separate from physical drop size. Track ancestry, strata, time blocks and native random state. First terminal crossing and recrossings need a declared rule. Wall-held/in-domain oil is inventory, not removal. A continuous-injection outlet-flux estimator cannot be replaced by complete-cohort algebra before inventory closes. Breakup conserves mass with exposure history; Hinze is diagnostic. Transport kernels can be reweighted only inside demonstrated linear noninteracting coverage.

### Decisions and uncertainty

At `Ci=500 mg/L`, `E=.94`, `r=.02`, `Cu=30.6122449 mg/L`, so a 30 mg/L gate fails; `E=.9412` is the exact passing threshold for the complete dilute cohort. Do not clip negative reduced efficiency. At every capacity flow, intersect all required scenarios first and retain gaps/failure islands. Without a valid continuous-band argument report sampled capacity only. No feasible/positive baseline means relative gain is undefined; adding epsilon would invent a result.

Weighted parcels and shared daughters are not Bernoulli trials. Scrambled Sobol points cannot inherit IID order-statistic sample-count guarantees. A preallocated one-third precision budget helps design experiments; it does not itself control false qualification. DecisionProfile fixes estimands, independent units, correlation/bias treatment, multiplicity, adaptive attempts and holdout use. A conjunction test for one fixed candidate and simultaneous coverage of all displayed bounds are different targets. Experiment-family identity survives forks so repeated attempts cannot reset spent error. Unknown model discrepancy blocks the affected claim.

Structural reserve refers to admissible multipliers under common external demand; flow-dependent loads come from each candidate’s accepted flow. Modal frequency ratios are not load multipliers. Fatigue needs cycle spectrum and process-specific evidence; RMS alone is insufficient. A finite `1e7`-cycle/run-out result is not infinite-life proof. Nonlinear/contact/buckling behavior may require actual re-solves instead of linear load scaling. Mass is integrated from the same accepted solid; no analytic credit for an unmeshed lattice.

### Durability and reproducibility

Use at-least-once execution and exactly-once accepted commitment, while charging every physically executed attempt. Fencing tokens prevent a late worker overwriting a retry. Publish immutable verified artifacts before committing references; same-filesystem rename is not object-store conditional-publication semantics. SQLite WAL stays on local storage; its official documentation requires same-host processes rather than a shared network filesystem. [SQLite WAL documentation](https://www.sqlite.org/wal.html).

Persist complete scientific/search state, not just elites and an RNG seed. Logical replay with an identical committed evaluator-result stream can be bitwise stable while native reruns agree only within justified tolerances. Actual elapsed/retry costs can differ under shuffled scheduling. Resume never resets budgets. Native solver restart is separately qualified: if particles/controller/averaging cannot be restored consistently, restart the task from a complete boundary or scratch and charge it.

## 8. Validation, logging and receipts

See `VALIDATION.md` for current executable plan checks, expected outputs, future validation tiers and failure interpretation. Each packet repeats exact target commands and named negative controls. Project checks use `rtk`; the planning helper uses Python standard library only and does not run scientific evaluation.

Before implementation, append a claim with packet/task IDs, tested base, exact write paths, dependencies, lease and stop conditions. During work, append material progress to authoritative COORD and the packet-owned progress file; do not replace someone else’s notice or claim an allocation by posting it. Explain discoveries that change scope before modifying shared contracts. Keep long raw logs private and publish only reviewed hashes/receipts.

A completed task needs a receipt referencing its task ID, tested commit, changed paths, exact command arrays and exit codes, log/artifact SHA-256, positive checks, negative controls, resource allocation, limitations and next dependency. The plan tool checks receipt structure and stored-file hash; that is an integrity check, not independent verification that scientific assertions are true. Reviewer acceptance and source evidence still determine qualification. A `done` task cannot promote scientific STATUS. Only the existing registry-derived mechanism may change science claims.

Update `progress/V5-XX.json` through `tools/v5_plan.py update` or a reviewed scoped edit. One worker owns each progress file. The integrator regenerates all TODO views/dashboard, checks their diff and commits them together. Browser notes/draft marks are local scratch information; they never count as authoritative completed tasks. The dashboard starts from an embedded snapshot and shows its data hash/date; regenerate after progress commits. It does not poll xbabe2 or alter repository files.

## 9. Stop conditions and handoff

Stop the affected packet on scope collision, mismatched identity/units, unresolved mandatory inputs, non-closing inventory, numerical/model error above the budget, unsupported active geometry, failed native fixtures, missing raw provenance, invalid statistical assumptions, stale/revoked calibration, full disk/resource pressure or expired lease. Preserve artifacts and diagnostics, record the exact blocker and continue only independent authorized work. Never loosen a threshold, change the baseline/duty, relabel a failure or remove failed attempts to get PASS.

No simulation or GPU work is part of preparing this plan. No historic allocation is assumed active. Physical builds, purchases, fleet expansion, public design disclosure and external contact require their appropriate explicit authorization after a concrete review package exists.

Final integration requires one PR, exact-head local and CI evidence, preserved journals/unrelated work, a source/changed-path inventory, immutable receipt, a current dashboard and explicit R1–R4 eligibility. Do not merge, force-push, clean other worktrees, or delete prior branches as part of this request. The owner reviews and merges. `PR_HANDOFF.md` gives current concrete commands and continuation rules.
