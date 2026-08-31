use std::f64::consts::{E, PI};

use crate::error::EvaError;
use crate::parser::Expr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AngleUnit {
    Degree,
    Radian,
    Gradian,
}

impl AngleUnit {
    pub fn from_str(s: &str) -> Self {
        match s {
            "radian" => AngleUnit::Radian,
            "gradian" => AngleUnit::Gradian,
            _ => AngleUnit::Degree,
        }
    }

    fn to_radians(&self, angle: f64) -> f64 {
        match self {
            AngleUnit::Degree => angle.to_radians(),
            AngleUnit::Radian => angle,
            AngleUnit::Gradian => angle * PI / 200.0,
        }
    }

    fn from_radians(&self, rad: f64) -> f64 {
        match self {
            AngleUnit::Degree => rad.to_degrees(),
            AngleUnit::Radian => rad,
            AngleUnit::Gradian => rad * 200.0 / PI,
        }
    }
}

pub struct Evaluator {
    pub angle_unit: AngleUnit,
    pub prev_answer: f64,
}

impl Evaluator {
    pub fn new(angle_unit: AngleUnit) -> Self {
        Self {
            angle_unit,
            prev_answer: 0.0,
        }
    }

    pub fn eval(&mut self, expr: &Expr) -> Result<f64, EvaError> {
        match expr {
            Expr::Number(n) => Ok(*n),
            Expr::Const(name) => match name.as_str() {
                "pi" => Ok(PI),
                "e" => Ok(E),
                "_" => Ok(self.prev_answer),
                _ => Err(EvaError::Parser(
                    "Too many operators, too few operands".into(),
                )),
            },
            Expr::Unary { op, expr } => {
                let val = self.eval(expr)?;
                match op {
                    '+' => Ok(val),
                    '-' => Ok(-val),
                    _ => Err(EvaError::Parser(
                        "Too many operators, too few operands".into(),
                    )),
                }
            }
            Expr::Binary { op, left, right } => {
                let l = self.eval(left)?;
                let r = self.eval(right)?;
                match op.as_str() {
                    "+" => Ok(l + r),
                    "-" => Ok(l - r),
                    "*" => Ok(l * r),
                    "/" => {
                        if r == 0.0 {
                            Err(EvaError::MathDivideByZero)
                        } else {
                            Ok(l / r)
                        }
                    }
                    "^" | "**" => Ok(l.powf(r)),
                    _ => Err(EvaError::Parser(
                        "Too many operators, too few operands".into(),
                    )),
                }
            }
            Expr::Call { name, args } => self.eval_call(name, args),
        }
    }

    fn eval_call(&mut self, name: &str, args: &[Expr]) -> Result<f64, EvaError> {
        match (name, args.len()) {
            ("sin", 1) => {
                let x = self.eval(&args[0])?;
                Ok(self.to_trig_input(x).sin())
            }
            ("cos", 1) => {
                let x = self.eval(&args[0])?;
                Ok(self.to_trig_input(x).cos())
            }
            ("tan", 1) => {
                let x = self.eval(&args[0])?;
                let rad = self.to_trig_input(x);
                if rad.cos().abs() < 1e-15 {
                    Err(EvaError::MathDivideByZero)
                } else {
                    Ok(rad.tan())
                }
            }
            ("csc", 1) => {
                let x = self.eval(&args[0])?;
                let s = self.to_trig_input(x).sin();
                if s.abs() < 1e-15 {
                    Err(EvaError::MathDivideByZero)
                } else {
                    Ok(1.0 / s)
                }
            }
            ("sec", 1) => {
                let x = self.eval(&args[0])?;
                let c = self.to_trig_input(x).cos();
                if c.abs() < 1e-15 {
                    Err(EvaError::MathDivideByZero)
                } else {
                    Ok(1.0 / c)
                }
            }
            ("cot", 1) => {
                let x = self.eval(&args[0])?;
                let rad = self.to_trig_input(x);
                let s = rad.sin();
                if s.abs() < 1e-15 {
                    Err(EvaError::MathDivideByZero)
                } else {
                    Ok(rad.cos() / s)
                }
            }
            ("sinh", 1) => Ok(self.eval(&args[0])?.sinh()),
            ("cosh", 1) => Ok(self.eval(&args[0])?.cosh()),
            ("tanh", 1) => Ok(self.eval(&args[0])?.tanh()),
            ("asin", 1) => {
                let x = self.eval(&args[0])?;
                if x < -1.0 || x > 1.0 {
                    return Err(EvaError::Domain);
                }
                Ok(self.from_trig_output(x.asin()))
            }
            ("acos", 1) => {
                let x = self.eval(&args[0])?;
                if x < -1.0 || x > 1.0 {
                    return Err(EvaError::Domain);
                }
                Ok(self.from_trig_output(x.acos()))
            }
            ("atan", 1) => {
                let x = self.eval(&args[0])?;
                Ok(self.from_trig_output(x.atan()))
            }
            ("acsc", 1) => {
                let x = self.eval(&args[0])?;
                if x.abs() < 1.0 {
                    return Err(EvaError::Domain);
                }
                Ok(self.from_trig_output((1.0 / x).asin()))
            }
            ("asec", 1) => {
                let x = self.eval(&args[0])?;
                if x.abs() < 1.0 {
                    return Err(EvaError::Domain);
                }
                Ok(self.from_trig_output((1.0 / x).acos()))
            }
            ("acot", 1) => {
                let x = self.eval(&args[0])?;
                Ok(self.from_trig_output((1.0 / x).atan()))
            }
            ("ln", 1) => {
                let x = self.eval(&args[0])?;
                if x <= 0.0 {
                    Err(EvaError::Domain)
                } else {
                    Ok(x.ln())
                }
            }
            ("log2", 1) => {
                let x = self.eval(&args[0])?;
                if x <= 0.0 {
                    Err(EvaError::Domain)
                } else {
                    Ok(x.log2())
                }
            }
            ("log10", 1) => {
                let x = self.eval(&args[0])?;
                if x <= 0.0 {
                    Err(EvaError::Domain)
                } else {
                    Ok(x.log10())
                }
            }
            ("sqrt", 1) => {
                let x = self.eval(&args[0])?;
                if x < 0.0 {
                    Err(EvaError::Domain)
                } else {
                    Ok(x.sqrt())
                }
            }
            ("ceil", 1) => Ok(self.eval(&args[0])?.ceil()),
            ("floor", 1) => Ok(self.eval(&args[0])?.floor()),
            ("abs", 1) => Ok(self.eval(&args[0])?.abs()),
            ("deg", 1) => {
                let x = self.eval(&args[0])?;
                Ok(x.to_degrees())
            }
            ("rad", 1) => {
                let x = self.eval(&args[0])?;
                Ok(x.to_radians())
            }
            ("log", 2) => {
                let x = self.eval(&args[0])?;
                let base = self.eval(&args[1])?;
                if x <= 0.0 || base <= 0.0 || base == 1.0 {
                    Err(EvaError::Domain)
                } else {
                    Ok(x.log(base))
                }
            }
            ("nroot", 2) => {
                let x = self.eval(&args[0])?;
                let n = self.eval(&args[1])?;
                if n == 0.0 {
                    Err(EvaError::MathDivideByZero)
                } else if x < 0.0 && n.fract() != 0.0 {
                    Err(EvaError::Domain)
                } else {
                    Ok(x.powf(1.0 / n))
                }
            }
            _ => Err(EvaError::Parser(
                "Too many operators, too few operands".into(),
            )),
        }
    }

    fn to_trig_input(&self, angle: f64) -> f64 {
        // Observed: gradian mode behaves like degree for trig in gold binary
        match self.angle_unit {
            AngleUnit::Radian => angle,
            AngleUnit::Degree | AngleUnit::Gradian => angle.to_radians(),
        }
    }

    fn from_trig_output(&self, rad: f64) -> f64 {
        match self.angle_unit {
            AngleUnit::Radian => rad,
            AngleUnit::Degree | AngleUnit::Gradian => rad.to_degrees(),
        }
    }
}
