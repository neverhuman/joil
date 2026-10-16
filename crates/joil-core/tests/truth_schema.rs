use joil_core::program::{DesignProgram, Rng};

#[test]
fn truth_schema_roundtrip() {
    for version in [2, 3] {
        let mut fixture = DesignProgram::baseline_ct35();
        fixture.schema_version = version;
        fixture.family = format!("ct35-exotic-head-v{version}");
        fixture.inlet_ramp.axial_drop_mm = 1.234567;
        let before = fixture.clone();
        fixture.validate().unwrap();
        assert_eq!(fixture, before, "validation mutated its input");
        let canonical = fixture.canonicalize().unwrap();
        assert_eq!(canonical.schema_version, version);
        assert_eq!(canonical.family, fixture.family);
        let json = serde_json::to_string(&canonical).unwrap();
        let loaded: DesignProgram = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.canonical_sha256(), canonical.canonical_sha256());
        assert_eq!(loaded.canonicalize().unwrap(), canonical);
    }
    let v2 = DesignProgram::baseline_ct35();
    let (v4, receipt) = v2.migrate_v2_to_v4().unwrap();
    assert_eq!(receipt.source_hash, v2.canonical_sha256().unwrap());
    assert_eq!(receipt.target_hash, v4.canonical_sha256().unwrap());
    assert_ne!(receipt.source_hash, receipt.target_hash);
    assert_eq!(receipt.implementation_hash.len(), 64);
    assert!(receipt.information_loss.is_empty());
    assert_eq!(v4.schema_version, 4);
    assert_eq!(v4.family, "ct35-legacy-macro-v4");
    assert!(v4.migrate_v2_to_v4().is_err());
    let mut v3 = v2;
    v3.schema_version = 3;
    v3.family = "ct35-exotic-head-v3".into();
    assert!(v3.migrate_v2_to_v4().is_err());
}

#[test]
fn truth_invalid_branch_counts_and_schema_mismatch() {
    let mut rng = Rng::new(0xabc);
    for enabled in [false, true] {
        for count in [0, 5, u32::MAX].into_iter().chain((0..1024).map(|_| {
            // Every generated count lies outside the representation's valid interval.
            5 + (rng.next_u64() % (u32::MAX as u64 - 4)) as u32
        })) {
            let mut p = DesignProgram::baseline_ct35();
            p.inlet_split.enabled = enabled;
            p.inlet_split.branches_per_port = count;
            assert!(p.validate().is_err(), "accepted {count} branches");
        }
    }
    for count in 2..=4 {
        let mut p = DesignProgram::baseline_ct35();
        p.inlet_split.enabled = true;
        p.inlet_split.branches_per_port = count;
        assert_eq!(
            p.canonicalize().unwrap().inlet_split.branches_per_port,
            count
        );
    }
    for (schema, family) in [
        (2, "ct35-exotic-head-v3"),
        (3, "ct35-exotic-head-v2"),
        (4, "ct35-exotic-head-v3"),
        (99, "ct35-exotic-head-v2"),
    ] {
        let mut p = DesignProgram::baseline_ct35();
        p.schema_version = schema;
        p.family = family.into();
        assert!(p.validate().is_err());
        assert!(p.canonical_sha256().is_err());
    }
}
