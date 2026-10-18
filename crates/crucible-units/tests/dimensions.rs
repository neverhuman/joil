use crucible_units::*;
use joil_evidence::identity::strict_decode;
use joil_evidence::observation::Finite;
use std::collections::BTreeMap;

fn constant(v: f64, unit: Unit) -> Expr {
    Expr::Constant {
        quantity: Quantity {
            value: Finite::new(v).unwrap(),
            unit,
        },
    }
}
fn evaluate(e: &Expr) -> Result<SiValue, UnitError> {
    e.evaluate(&BTreeMap::new(), 128)
}
#[test]
fn si_conversion_oracles_and_signed_gauge_pressure() {
    assert_eq!(
        Metres::from_mm(Millimetres::new(1000.0).unwrap())
            .unwrap()
            .get(),
        1.0
    );
    assert_eq!(
        Millimetres::from_m(Metres::new(0.035).unwrap())
            .unwrap()
            .get(),
        35.0
    );
    assert_eq!(
        evaluate(&constant(180.0, Unit::Kilopascal))
            .unwrap()
            .value
            .get(),
        180000.0
    );
    assert_eq!(Pascals::new(-180000.0).unwrap().get(), -180000.0);
    let add = Expr::Add {
        left: Box::new(constant(1000.0, Unit::Millimetre)),
        right: Box::new(constant(1.0, Unit::Metre)),
    };
    assert_eq!(evaluate(&add).unwrap().value.get(), 2.0);
    let wrong = Expr::Add {
        left: Box::new(constant(1.0, Unit::Metre)),
        right: Box::new(constant(1.0, Unit::Pascal)),
    };
    assert_eq!(evaluate(&wrong), Err(UnitError::IncompatibleDimension));
}
#[test]
fn mass_concentration_basis_is_not_density_or_an_oil_fraction() {
    let rho = KilogramsPerCubicMetre::new(850.0).unwrap();
    // 1000 mg/L is exactly 1 kg/m3, hence oil fraction=1/850.
    let phi = volume_fraction_from_concentration(MgPerLitreDispersedOil::new(1000.0).unwrap(), rho)
        .unwrap();
    assert!((phi.get() - 1.0 / 850.0).abs() < 1e-18);
    assert!(phi.get() > 0.001); // disproves the incompatible <=0.10vol% assertion.
    assert!((concentration_from_volume_fraction(phi, rho).unwrap().get() - 1000.0).abs() < 1e-10);
    let ast = Expr::ConcentrationToVolumeFraction {
        concentration: Box::new(constant(1000.0, Unit::MgPerLitreDispersedOil)),
        oil_density: Box::new(constant(850.0, Unit::KilogramPerCubicMetre)),
    };
    assert!((evaluate(&ast).unwrap().value.get() - 1.0 / 850.0).abs() < 1e-18);
    let wrong = Expr::Add {
        left: Box::new(constant(1.0, Unit::KilogramPerCubicMetre)),
        right: Box::new(constant(1.0, Unit::MgPerLitreDispersedOil)),
    };
    assert_eq!(evaluate(&wrong), Err(UnitError::IncompatibleQuantityKind));
    assert!(KilogramsPerCubicMetre::new(0.0).is_err());
}
#[test]
fn treated_energy_requires_loss_power_and_treated_flow() {
    let energy = Expr::EnergyPerTreatedVolume {
        mechanical_loss: Box::new(constant(10.0, Unit::MechanicalLossWatt)),
        treated_flow: Box::new(constant(0.002, Unit::TreatedCubicMetrePerSecond)),
    };
    let value = evaluate(&energy).unwrap();
    assert_eq!(value.value.get(), 5000.0);
    assert_eq!(
        value.quantity_type.kind,
        QuantityKind::EnergyPerTreatedVolume
    );
    let pressure_proxy = Expr::EnergyPerTreatedVolume {
        mechanical_loss: Box::new(constant(10000.0, Unit::Pascal)),
        treated_flow: Box::new(constant(0.002, Unit::TreatedCubicMetrePerSecond)),
    };
    assert_eq!(
        evaluate(&pressure_proxy),
        Err(UnitError::IncompatibleQuantityKind)
    );
}
#[test]
fn strict_wire_and_numeric_edge_controls() {
    assert!(strict_decode::<Metres>(b"-0.0").is_err());
    assert!(strict_decode::<Metres>(b"NaN").is_err());
    assert!(strict_decode::<Metres>(b"9007199254740993").is_err());
    assert!(Expr::from_json(br#"{"op":"parameter","name":"a","name":"b"}"#).is_err());
    assert!(Expr::from_json(br#"{"op":"parameter","name":"a","expected_bonus":10}"#).is_err());
    assert_eq!(
        evaluate(&Expr::Divide {
            left: Box::new(constant(1.0, Unit::Metre)),
            right: Box::new(constant(0.0, Unit::Dimensionless))
        }),
        Err(UnitError::DivisionByZero)
    );
    assert_eq!(
        evaluate(&Expr::Multiply {
            left: Box::new(constant(f64::MAX, Unit::Metre)),
            right: Box::new(constant(2.0, Unit::Dimensionless))
        }),
        Err(UnitError::NonFinite)
    );
    assert_eq!(
        Metres::from_mm(Millimetres::new(f64::from_bits(1)).unwrap()),
        Err(UnitError::Underflow)
    );
    assert_eq!(
        constant(1.0, Unit::Metre).evaluate(&BTreeMap::new(), 0),
        Err(UnitError::GasExhausted)
    );
    assert!(matches!(
        evaluate(&Expr::Parameter {
            name: "missing".into()
        }),
        Err(UnitError::MissingParameter(_))
    ));
}
#[test]
fn transforms_are_proper_rotations_in_declared_si_frames() {
    let f = |v| Finite::new(v).unwrap();
    let m = |v| Metres::new(v).unwrap();
    let mut transform = FrameTransform {
        from: CoordinateFrame::RightHandedSiZDownstream,
        to: CoordinateFrame::RightHandedSiZUp,
        translation_m: [m(1.0), m(2.0), m(3.0)],
        rotation: [
            [f(1.0), f(0.0), f(0.0)],
            [f(0.0), f(-1.0), f(0.0)],
            [f(0.0), f(0.0), f(-1.0)],
        ],
    };
    assert_eq!(
        transform.apply_point([m(2.0), m(3.0), m(4.0)]).unwrap(),
        [m(3.0), m(-1.0), m(-1.0)]
    );
    transform.rotation[2][2] = f(1.0);
    assert_eq!(transform.check(), Err(UnitError::InvalidTransform)); // reflection
    transform.rotation[0][0] = f(1000.0);
    assert_eq!(transform.check(), Err(UnitError::InvalidTransform)); // hidden scale
}
