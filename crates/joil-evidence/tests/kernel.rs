use joil_evidence::{identity::*, *};

#[test]
fn exact_identity_and_strict_ingestion() {
    assert!(strict_json(br#"{"x":1,"x":2}"#).is_err());
    assert!(strict_json(br#"{"x":1e999}"#).is_err());
    assert!(strict_json(br#"{"x":1e-999}"#).is_err());
    assert_ne!(
        scientific_hash(&0.0).unwrap(),
        scientific_hash(&-0.0).unwrap()
    );
    let content = ContentRef::from_bytes("source", "1", b"bytes");
    content.check().unwrap();
    assert_ne!(content, ContentRef::from_bytes("source", "1", b"Bytes"));
}

#[test]
fn append_only_event_chain_detects_mutation() {
    let first = Event {
        sequence: 1,
        previous_sha256: None,
        payload: ContentRef::from_bytes("start", "1", b"start"),
    };
    let second = Event {
        sequence: 2,
        previous_sha256: Some(first.sha256().unwrap()),
        payload: ContentRef::from_bytes("end", "1", b"end"),
    };
    let mut events = vec![first, second];
    verify_event_chain(&events).unwrap();
    events[0].payload.sha256 = "0".repeat(64);
    assert!(verify_event_chain(&events).is_err());
}

#[test]
fn samples_do_not_depend_on_scheduling() {
    let ordered: Vec<_> = (0..20).map(|i| sample_identity("campaign", 7, i)).collect();
    for i in (0..20).rev() {
        assert_eq!(ordered[i], sample_identity("campaign", 7, i as u64));
    }
    assert_ne!(ordered[0], sample_identity("campaign", 8, 0));
}
