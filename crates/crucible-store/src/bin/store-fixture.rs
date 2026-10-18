//! Owned synthetic crash child used only by the durability regression.
use crucible_store::{
    manifest::{Metadata, ObjectKind},
    store::PublishPhase,
    Store,
};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let root = std::path::Path::new(&args[1]);
    let selected = &args[2];
    let store = Store::open(root).unwrap();
    store
        .publish_checked(
            b"synthetic crash payload",
            Metadata::private(ObjectKind::Input),
            None,
            |phase| {
                if format!("{phase:?}") == *selected {
                    std::fs::write(root.join("child-ready"), format!("{phase:?}")).unwrap();
                    loop {
                        std::thread::sleep(std::time::Duration::from_millis(20));
                    }
                }
                Ok(())
            },
        )
        .unwrap();
    let _ = PublishPhase::ParentSynced;
}
