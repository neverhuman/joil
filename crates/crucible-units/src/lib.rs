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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Expr {
    Constant {
        quantity: Quantity,
    },
    Parameter {
        name: String,
    },
    Add {
        left: Box<Self>,
        right: Box<Self>,
    },
    Subtract {
        left: Box<Self>,
        right: Box<Self>,
    },
    Multiply {
        left: Box<Self>,
        right: Box<Self>,
    },
    Divide {
        left: Box<Self>,
        right: Box<Self>,
    },
    Negate {
        value: Box<Self>,
    },
    ConcentrationToVolumeFraction {
        concentration: Box<Self>,
        oil_density: Box<Self>,
    },
    EnergyPerTreatedVolume {
        mechanical_loss: Box<Self>,
        treated_flow: Box<Self>,
    },
}
impl Expr {
    pub fn from_json(bytes: &[u8]) -> Result<Self, UnitError> {
        joil_evidence::identity::strict_decode(bytes).map_err(UnitError::Wire)
    }
    /// No scripts, file I/O, dynamic calls or unbounded recursion exist in this AST.
    pub fn evaluate(
        &self,
        parameters: &BTreeMap<String, Quantity>,
        gas: u32,
    ) -> Result<SiValue, UnitError> {
        fn evaluate_expr(
            e: &Expr,
            p: &BTreeMap<String, Quantity>,
            gas: &mut u32,
            depth: u32,
        ) -> Result<SiValue, UnitError> {
            if *gas == 0 || depth > 32 {
                return Err(UnitError::GasExhausted);
            }
            *gas -= 1;
            let recurse = |e: &Expr, gas: &mut u32| evaluate_expr(e, p, gas, depth + 1);
            match e {
                Expr::Constant { quantity } => quantity.si(),
                Expr::Parameter { name } => p
                    .get(name)
                    .ok_or_else(|| UnitError::MissingParameter(name.clone()))?
                    .si(),
                Expr::Negate { value } => {
                    let v = recurse(value, gas)?;
                    Ok(SiValue {
                        value: finite(-v.value.get())?,
                        ..v
                    })
                }
                Expr::ConcentrationToVolumeFraction {
                    concentration,
                    oil_density,
                } => {
                    let c = recurse(concentration, gas)?;
                    let rho = recurse(oil_density, gas)?;
                    if c.quantity_type.kind != QuantityKind::DispersedOilMassConcentration
                        || rho.quantity_type.kind != QuantityKind::MaterialDensity
                    {
                        return Err(UnitError::IncompatibleQuantityKind);
                    }
                    if c.value.get() < 0.0 || rho.value.get() <= 0.0 {
                        return Err(UnitError::InvalidRange);
                    }
                    let result = c.value.get() / rho.value.get();
                    if c.value.get() != 0.0 && result == 0.0 {
                        return Err(UnitError::Underflow);
                    }
                    let phi = OilVolumeFraction::new(result)?;
                    Ok(SiValue {
                        value: finite(phi.get())?,
                        quantity_type: QuantityType {
                            dimensions: DIMENSIONLESS,
                            kind: QuantityKind::OilVolumeFraction,
                        },
                    })
                }
                Expr::EnergyPerTreatedVolume {
                    mechanical_loss,
                    treated_flow,
                } => {
                    let power = recurse(mechanical_loss, gas)?;
                    let flow = recurse(treated_flow, gas)?;
                    if power.quantity_type.kind != QuantityKind::MechanicalLossPower
                        || flow.quantity_type.kind != QuantityKind::TreatedFlow
                    {
                        return Err(UnitError::IncompatibleQuantityKind);
                    }
                    if power.value.get() < 0.0 || flow.value.get() <= 0.0 {
                        return Err(UnitError::InvalidRange);
                    }
                    let result = power.value.get() / flow.value.get();
                    if power.value.get() != 0.0 && result == 0.0 {
                        return Err(UnitError::Underflow);
                    }
                    Ok(SiValue {
                        value: finite(result)?,
                        quantity_type: Unit::JoulePerTreatedCubicMetre.quantity_type(),
                    })
                }
                Expr::Add { left, right }
                | Expr::Subtract { left, right }
                | Expr::Multiply { left, right }
                | Expr::Divide { left, right } => {
                    let l = recurse(left, gas)?;
                    let r = recurse(right, gas)?;
                    let (value, ty) = match e {
                        Expr::Add { .. } | Expr::Subtract { .. } => {
                            if l.quantity_type.dimensions != r.quantity_type.dimensions {
                                return Err(UnitError::IncompatibleDimension);
                            }
                            if l.quantity_type.kind != r.quantity_type.kind {
                                return Err(UnitError::IncompatibleQuantityKind);
                            }
                            (
                                if matches!(e, Expr::Add { .. }) {
                                    l.value.get() + r.value.get()
                                } else {
                                    l.value.get() - r.value.get()
                                },
                                l.quantity_type,
                            )
                        }
                        _ => {
                            if l.quantity_type.kind != QuantityKind::GenericSi
                                || r.quantity_type.kind != QuantityKind::GenericSi
                            {
                                return Err(UnitError::IncompatibleQuantityKind);
                            }
                            let divide = matches!(e, Expr::Divide { .. });
                            if divide && r.value.get() == 0.0 {
                                return Err(UnitError::DivisionByZero);
                            }
                            let v = if divide {
                                l.value.get() / r.value.get()
                            } else {
                                l.value.get() * r.value.get()
                            };
                            if l.value.get() != 0.0 && r.value.get() != 0.0 && v == 0.0 {
                                return Err(UnitError::Underflow);
                            }
                            (
                                v,
                                QuantityType {
                                    dimensions: l
                                        .quantity_type
                                        .dimensions
                                        .combine(r.quantity_type.dimensions, divide)?,
                                    kind: QuantityKind::GenericSi,
                                },
                            )
                        }
                    };
                    Ok(SiValue {
                        value: finite(value)?,
                        quantity_type: ty,
                    })
                }
            }
        }
        let mut remaining = gas;
        evaluate_expr(self, parameters, &mut remaining, 0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinateFrame {
    RightHandedSiZDownstream,
    RightHandedSiZUp,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrameTransform {
    pub from: CoordinateFrame,
    pub to: CoordinateFrame,
    pub translation_m: [Metres; 3],
    pub rotation: [[Finite; 3]; 3],
}
impl FrameTransform {
    pub fn check(&self) -> Result<(), UnitError> {
        for row in &self.rotation {
            for v in row {
                finite(v.get())?;
            }
        }
        let r = self.rotation.map(|row| row.map(Finite::get));
        for i in 0..3 {
            for j in 0..3 {
                let dot = (0..3).map(|k| r[i][k] * r[j][k]).sum::<f64>();
                if (dot - if i == j { 1.0 } else { 0.0 }).abs() > 2e-12 {
                    return Err(UnitError::InvalidTransform);
                }
            }
        }
        let det = r[0][0] * (r[1][1] * r[2][2] - r[1][2] * r[2][1])
            - r[0][1] * (r[1][0] * r[2][2] - r[1][2] * r[2][0])
            + r[0][2] * (r[1][0] * r[2][1] - r[1][1] * r[2][0]);
        if !det.is_finite() || (det - 1.0).abs() > 2e-12 {
            return Err(UnitError::InvalidTransform);
        }
        Ok(())
    }
    pub fn apply_point(&self, point: [Metres; 3]) -> Result<[Metres; 3], UnitError> {
        self.check()?;
        let mut out = [Metres::new(0.0)?; 3];
        for (i, value) in out.iter_mut().enumerate() {
            *value = Metres::new(
                self.translation_m[i].get()
                    + (0..3)
                        .map(|j| self.rotation[i][j].get() * point[j].get())
                        .sum::<f64>(),
            )?;
        }
        Ok(out)
    }
}
