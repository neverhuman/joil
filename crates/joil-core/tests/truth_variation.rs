use joil_core::program::{DesignProgram, Rng};
use sha2::{Digest, Sha256};

#[test]
fn truth_variation_outcome() {
    let p = DesignProgram::baseline_ct35();
    let mut rng = Rng::new(44);
    let before = rng.0;
    let outcome = p.mutate(&mut rng).unwrap();
    assert!(outcome.failure.is_none());
    assert!(outcome.child.is_some());
    assert!(!outcome.edits.is_empty());
    assert_eq!(outcome.rng_span.before, before);
    assert_eq!(outcome.rng_span.after, rng.0);
    assert_ne!(before, rng.0);
    assert_eq!(outcome.parent_hashes, vec![p.canonical_sha256().unwrap()]);
    assert_eq!(outcome.locality.geometric_distance, None);
    let mut replay_rng = Rng::new(before);
    assert_eq!(outcome, p.mutate(&mut replay_rng).unwrap());
    let mut v3 = p.clone();
    v3.schema_version = 3;
    v3.family = "ct35-exotic-head-v3".into();
    assert_eq!(
        v3.mutate_conventional_axisymmetric(&mut rng)
            .unwrap()
            .into_child()
            .unwrap()
            .schema_version,
        3
    );
    assert!(p.crossover(&v3, &mut rng).unwrap().failure.is_some());
}

#[test]
fn truth_no_baseline_fallback() {
    let parent = DesignProgram::baseline_ct35();
    let mut invalid = parent.clone();
    invalid.inlet_split.enabled = true;
    invalid.inlet_split.branches_per_port = 99;
    let mut rng = Rng::new(1);
    let outcome = parent.propose_edit(invalid.clone(), &mut rng).unwrap();
    assert!(outcome.failure.is_some());
    assert!(outcome.child.is_none());
    assert_eq!(outcome.proposal, Some(invalid.clone()));
    for outcome in [
        invalid.mutate(&mut rng).unwrap(),
        invalid.mutate_branch_topology(&mut rng).unwrap(),
        invalid.mutate_diffeomorphic_spline(&mut rng).unwrap(),
        invalid.mutate_tpms_grading(&mut rng).unwrap(),
        invalid.mutate_wave3_progressive(&mut rng).unwrap(),
        invalid.mutate_conventional_axisymmetric(&mut rng).unwrap(),
        parent.crossover(&invalid, &mut rng).unwrap(),
    ] {
        assert!(outcome.failure.is_some());
        assert!(outcome.child.is_none());
        assert!(outcome.repairs.is_empty());
    }
    invalid.tpms_insert.sheet_thickness_mm = f64::NAN;
    assert!(invalid.validate().is_err());
}

#[test]
fn truth_repair_receipt_replays_complete_proposal() {
    let p = DesignProgram::baseline_ct35();
    let mut rng = Rng::new(8);
    let outcome = (0..100)
        .map(|_| p.mutate_wave3_progressive(&mut rng).unwrap())
        .find(|outcome| !outcome.repairs.is_empty())
        .expect("fixture must exercise bounded projection");
    let raw = outcome.proposal.unwrap();
    let mut wire = serde_json::to_value(&raw).unwrap();
    for repair in outcome.repairs {
        assert_eq!(
            repair.before_hash,
            format!("{:x}", Sha256::digest(serde_json::to_vec(&raw).unwrap()))
        );
        assert_ne!(repair.before_hash, repair.after_hash);
        assert!(!repair.edits.is_empty());
        for edit in repair.edits {
            let key = edit.path.strip_prefix('/').unwrap();
            assert_eq!(
                wire.get(key).cloned().unwrap_or(serde_json::Value::Null),
                edit.before
            );
            if edit.after.is_null() {
                wire.as_object_mut().unwrap().remove(key);
            } else {
                wire[key] = edit.after;
            }
        }
        let repaired: DesignProgram = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(
            repair.after_hash,
            format!(
                "{:x}",
                Sha256::digest(serde_json::to_vec(&repaired).unwrap())
            )
        );
        assert_eq!(
            repaired.canonicalize().unwrap(),
            outcome.child.clone().unwrap()
        );
    }
}
