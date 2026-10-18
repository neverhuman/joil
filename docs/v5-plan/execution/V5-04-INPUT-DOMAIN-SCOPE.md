# Serialized input-domain contract migration

C007-F3 exposed an ambiguity: `EstimandKind::Mean` specifies a statistical
functional, while the requirement for a strictly positive ratio baseline is an
input-domain constraint. Optional data alone cannot declare that constraint.

The integrator adds `EstimandInputDomain` to `Estimand` and its content-bound,
reviewed `EstimandDefinition`, enforcing equality during profile validation.
`Unspecified` is the omitted legacy default and remains omitted on serialization,
so historical payload identities remain stable. Joint inference refuses an
unspecified domain. A declared positive-baseline domain requires baseline data;
the existing finite/positive checks apply to every supplied paired baseline block.

This additive contract owns only `crates/joil-evidence/src/decision.rs`, its
`tests/decision.rs`, and `schemas/v5/decision-profile.schema.json`. The integrator
also performs the necessary serialized caller migration in the released C007
`crates/crucible-stats/src/analysis/joint.rs`, `tests/common/mod.rs` and
`tests/joint.rs`. Claude's current V5-19 scope is disjoint. The signed-zero and
Pearson follow-ups retain their separate source commits and immutable receipts.

Controls must show the old missing-baseline acceptance, its new refusal, the
valid synthetic S02 family, refusal of a restated domain borrowing a reviewed
formula, and identity-preserving legacy decoding to an unspecified domain.
No requirement threshold, physical baseline, task criterion, calibration or
scientific status is changed. Integration gates and exact-head CI remain required
before acceptance; no packet completion follows from this contract alone.
