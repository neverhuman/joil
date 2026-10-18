//! Exercise the shared kernel through the compatibility package entry point.
#[path = "../../joil-geometry/tests/kernel.rs"]
mod shared;

#[test]
fn compatibility_aliases_use_the_same_validated_types() {
    fn accepts_primary(p: joil_geometry::Point3) -> [f64; 3] {
        p.coordinates_m()
    }
    let point = crucible_geom::Point3::metres([1.0, 2.0, 3.0]).unwrap();
    assert_eq!(accepts_primary(point), [1.0, 2.0, 3.0]);
}
