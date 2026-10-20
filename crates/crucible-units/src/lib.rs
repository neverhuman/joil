//! Bounded SI arithmetic. Wire identity and strict parsing belong to joil-evidence.
use joil_evidence::observation::Finite;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnitError {
    Wire(String),
    NonFinite,
    NegativeZero,
    InvalidRange,
    Underflow,
    IncompatibleDimension,
    IncompatibleQuantityKind,
    DivisionByZero,
    MissingParameter(String),
    GasExhausted,
    InvalidTransform,
}
impl fmt::Display for UnitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for UnitError {}
fn finite(value: f64) -> Result<Finite, UnitError> {
    if value == 0.0 && value.is_sign_negative() {
        return Err(UnitError::NegativeZero);
    }
    Finite::new(value).map_err(|_| UnitError::NonFinite)
}
macro_rules! quantity {
    ($name:ident,$predicate:expr) => {
        #[derive(Clone, Copy, Debug, PartialEq, Serialize)]
        #[serde(transparent)]
        pub struct $name(Finite);
        impl $name {
            pub fn new(value: f64) -> Result<Self, UnitError> {
                let x = finite(value)?;
                if !($predicate)(value) {
                    return Err(UnitError::InvalidRange);
                }
                Ok(Self(x))
            }
            pub fn get(self) -> f64 {
                self.0.get()
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                Self::new(Finite::deserialize(d)?.get()).map_err(serde::de::Error::custom)
            }
        }
    };
}
quantity!(Metres, |_: f64| true);
quantity!(Millimetres, |_: f64| true);
// Signed gauge pressure is legitimate; absolute/budget constraints are separate.
quantity!(Pascals, |_: f64| true);
quantity!(Seconds, |v: f64| v >= 0.0);
quantity!(Kilograms, |v: f64| v >= 0.0);
quantity!(KilogramsPerCubicMetre, |v: f64| v > 0.0);
quantity!(MgPerLitreDispersedOil, |v: f64| v >= 0.0);
quantity!(OilVolumeFraction, |v: f64| (0.0..=1.0).contains(&v));
fn scaled(value: f64, scale: f64) -> Result<f64, UnitError> {
    let result = value * scale;
    finite(result)?;
    if value != 0.0 && result == 0.0 {
        return Err(UnitError::Underflow);
    }
    Ok(result)
}
impl Metres {
    pub fn from_mm(value: Millimetres) -> Result<Self, UnitError> {
        Self::new(scaled(value.get(), 1e-3)?)
    }
}
impl Millimetres {
    pub fn from_m(value: Metres) -> Result<Self, UnitError> {
        Self::new(scaled(value.get(), 1e3)?)
    }
}
pub fn volume_fraction_from_concentration(
    c: MgPerLitreDispersedOil,
    oil_density: KilogramsPerCubicMetre,
) -> Result<OilVolumeFraction, UnitError> {
    let mass_per_m3 = scaled(c.get(), 1e-3)?;
    let result = mass_per_m3 / oil_density.get();
    if mass_per_m3 != 0.0 && result == 0.0 {
        return Err(UnitError::Underflow);
    }
    OilVolumeFraction::new(result)
}
pub fn concentration_from_volume_fraction(
    phi: OilVolumeFraction,
    oil_density: KilogramsPerCubicMetre,
) -> Result<MgPerLitreDispersedOil, UnitError> {
    let mass_per_m3 = phi.get() * oil_density.get();
    if phi.get() != 0.0 && mass_per_m3 == 0.0 {
        return Err(UnitError::Underflow);
    }
    MgPerLitreDispersedOil::new(scaled(mass_per_m3, 1e3)?)
}

/// Exponents of mass, length and time; bounded integer arithmetic never wraps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dimensions {
    pub mass: i8,
    pub length: i8,
    pub time: i8,
}
impl Dimensions {
    fn combine(self, b: Self, subtract: bool) -> Result<Self, UnitError> {
        let op = |a: i8, b: i8| {
            if subtract {
                a.checked_sub(b)
            } else {
                a.checked_add(b)
            }
            .ok_or(UnitError::IncompatibleDimension)
        };
        Ok(Self {
            mass: op(self.mass, b.mass)?,
            length: op(self.length, b.length)?,
            time: op(self.time, b.time)?,
        })
    }
}
const DIMENSIONLESS: Dimensions = Dimensions {
    mass: 0,
    length: 0,
    time: 0,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuantityKind {
    GenericSi,
    MaterialDensity,
    DispersedOilMassConcentration,
    OilVolumeFraction,
    MechanicalLossPower,
    TreatedFlow,
    EnergyPerTreatedVolume,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhysicalUnit {
    Metre,
    Millimetre,
    Second,
    Kilogram,
    CubicMetre,
    CubicMetrePerSecond,
    Pascal,
    Kilopascal,
    KilogramPerCubicMetre,
    MgPerLitreDispersedOil,
    Dimensionless,
    MechanicalLossWatt,
    TreatedCubicMetrePerSecond,
    JoulePerTreatedCubicMetre,
}
pub type Unit = PhysicalUnit;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuantityType {
    pub dimensions: Dimensions,
    pub kind: QuantityKind,
}
impl Unit {
    pub fn quantity_type(self) -> QuantityType {
        let (dimensions, kind) = match self {
            Self::Metre | Self::Millimetre => (
                Dimensions {
                    mass: 0,
                    length: 1,
                    time: 0,
                },
                QuantityKind::GenericSi,
            ),
            Self::Second => (
                Dimensions {
                    mass: 0,
                    length: 0,
                    time: 1,
                },
                QuantityKind::GenericSi,
            ),
            Self::Kilogram => (
                Dimensions {
                    mass: 1,
                    length: 0,
                    time: 0,
                },
                QuantityKind::GenericSi,
            ),
            Self::CubicMetre => (
                Dimensions {
                    mass: 0,
                    length: 3,
                    time: 0,
                },
                QuantityKind::GenericSi,
            ),
            Self::CubicMetrePerSecond => (
                Dimensions {
                    mass: 0,
                    length: 3,
                    time: -1,
                },
                QuantityKind::GenericSi,
            ),
            Self::Pascal | Self::Kilopascal => (
                Dimensions {
                    mass: 1,
                    length: -1,
                    time: -2,
                },
                QuantityKind::GenericSi,
            ),
            Self::KilogramPerCubicMetre => (
                Dimensions {
                    mass: 1,
                    length: -3,
                    time: 0,
                },
                QuantityKind::MaterialDensity,
            ),
            Self::MgPerLitreDispersedOil => (
                Dimensions {
                    mass: 1,
                    length: -3,
                    time: 0,
                },
                QuantityKind::DispersedOilMassConcentration,
            ),
            Self::Dimensionless => (DIMENSIONLESS, QuantityKind::GenericSi),
            Self::MechanicalLossWatt => (
                Dimensions {
                    mass: 1,
                    length: 2,
                    time: -3,
                },
                QuantityKind::MechanicalLossPower,
            ),
            Self::TreatedCubicMetrePerSecond => (
                Dimensions {
                    mass: 0,
                    length: 3,
                    time: -1,
                },
                QuantityKind::TreatedFlow,
            ),
            Self::JoulePerTreatedCubicMetre => (
                Dimensions {
                    mass: 1,
                    length: -1,
                    time: -2,
                },
                QuantityKind::EnergyPerTreatedVolume,
            ),
        };
        QuantityType { dimensions, kind }
    }
    fn scale(self) -> f64 {
        match self {
            Self::Millimetre | Self::MgPerLitreDispersedOil => 1e-3,
            Self::Kilopascal => 1e3,
            _ => 1.0,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Quantity {
    pub value: Finite,
    pub unit: Unit,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SiValue {
    pub value: Finite,
    pub quantity_type: QuantityType,
}
impl Quantity {
    pub fn si(&self) -> Result<SiValue, UnitError> {
        finite(self.value.get())?;
        match self.unit {
            Unit::KilogramPerCubicMetre if self.value.get() <= 0.0 => {
                return Err(UnitError::InvalidRange)
            }
            Unit::MgPerLitreDispersedOil | Unit::MechanicalLossWatt if self.value.get() < 0.0 => {
                return Err(UnitError::InvalidRange)
            }
            Unit::TreatedCubicMetrePerSecond if self.value.get() <= 0.0 => {
                return Err(UnitError::InvalidRange)
            }
            _ => (),
        }
        Ok(SiValue {
            value: finite(scaled(self.value.get(), self.unit.scale())?)?,
            quantity_type: self.unit.quantity_type(),
        })
    }
}

mod expr;
pub use expr::*;
