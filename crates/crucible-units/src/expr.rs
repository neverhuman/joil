use super::*;

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
