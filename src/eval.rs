use std::f64::consts::{E, PI};

use crate::angle::AngleUnit;
use crate::parser::Expr;

#[derive(Debug, Clone, PartialEq)]
pub enum EvalError {
    DomainError,
    MathError,
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvalError::DomainError => write!(f, "Domain Error: Out of bounds!"),
            EvalError::MathError => write!(f, "Math Error: Divide by zero error!"),
        }
    }
}

pub struct Context {
    pub angle_unit: AngleUnit,
    pub previous: f64,
}

impl Context {
    pub fn new(angle_unit: AngleUnit) -> Self {
        Self {
            angle_unit,
            previous: 0.0,
        }
    }
}

pub fn eval(expr: &Expr, ctx: &Context) -> Result<f64, EvalError> {
    match expr {
        Expr::Number(n) => Ok(*n),
        Expr::Ident(name) => match name.as_str() {
            "e" => Ok(E),
            "pi" => Ok(PI),
            "_" => Ok(ctx.previous),
            other if crate::parser::is_function_name(other) => Err(EvalError::DomainError),
            _ => Err(EvalError::DomainError),
        },
        Expr::Unary { op, expr } => {
            let value = eval(expr, ctx)?;
            Ok(match op {
                crate::parser::UnaryOp::Plus => value,
                crate::parser::UnaryOp::Minus => -value,
            })
        }
        Expr::Binary { op, left, right } => {
            let l = eval(left, ctx)?;
            let r = eval(right, ctx)?;
            match op {
                crate::parser::BinaryOp::Add => Ok(l + r),
                crate::parser::BinaryOp::Sub => Ok(l - r),
                crate::parser::BinaryOp::Mul => Ok(l * r),
                crate::parser::BinaryOp::Div => {
                    if r.abs() < f64::EPSILON {
                        Err(EvalError::MathError)
                    } else {
                        Ok(l / r)
                    }
                }
                crate::parser::BinaryOp::Pow => Ok(l.powf(r)),
            }
        }
        Expr::Call { name, args } => eval_call(name, args, ctx),
    }
}

fn eval_call(name: &str, args: &[Expr], ctx: &Context) -> Result<f64, EvalError> {
    let values: Result<Vec<f64>, EvalError> = args.iter().map(|a| eval(a, ctx)).collect();
    let values = values?;

    match name {
        "sin" => trig(values[0], ctx.angle_unit, f64::sin),
        "cos" => trig(values[0], ctx.angle_unit, f64::cos),
        "tan" => trig(values[0], ctx.angle_unit, f64::tan),
        "csc" => {
            let v = trig(values[0], ctx.angle_unit, f64::sin)?;
            if v.abs() < f64::EPSILON {
                Err(EvalError::MathError)
            } else {
                Ok(1.0 / v)
            }
        }
        "sec" => {
            let v = trig(values[0], ctx.angle_unit, f64::cos)?;
            if v.abs() < f64::EPSILON {
                Err(EvalError::MathError)
            } else {
                Ok(1.0 / v)
            }
        }
        "cot" => {
            let s = trig(values[0], ctx.angle_unit, f64::sin)?;
            let c = trig(values[0], ctx.angle_unit, f64::cos)?;
            if s.abs() < f64::EPSILON {
                Err(EvalError::MathError)
            } else {
                Ok(c / s)
            }
        }
        "sinh" => Ok(values[0].sinh()),
        "cosh" => Ok(values[0].cosh()),
        "tanh" => Ok(values[0].tanh()),
        "asin" => domain(values[0].abs() <= 1.0, values[0].asin()),
        "acos" => domain(values[0].abs() <= 1.0, values[0].acos()),
        "atan" => Ok(values[0].atan()),
        "acsc" => {
            if values[0].abs() < 1.0 {
                Err(EvalError::DomainError)
            } else {
                Ok((1.0 / values[0]).asin())
            }
        }
        "asec" => {
            if values[0].abs() < 1.0 {
                Err(EvalError::DomainError)
            } else {
                Ok((1.0 / values[0]).acos())
            }
        }
        "acot" => Ok((1.0 / values[0]).atan()),
        "ln" => domain(values[0] > 0.0, values[0].ln()),
        "log2" => domain(values[0] > 0.0, values[0].log2()),
        "log10" => domain(values[0] > 0.0, values[0].log10()),
        "sqrt" => domain(values[0] >= 0.0, values[0].sqrt()),
        "ceil" => Ok(values[0].ceil()),
        "floor" => Ok(values[0].floor()),
        "abs" => Ok(values[0].abs()),
        "log" => {
            let base = values[1];
            let value = values[0];
            domain(value > 0.0 && base > 0.0 && base != 1.0, value.log(base))
        }
        "nroot" => {
            let value = values[0];
            let n = values[1];
            if n.abs() < f64::EPSILON {
                Err(EvalError::MathError)
            } else if value < 0.0 && n.fract().abs() > f64::EPSILON {
                Err(EvalError::DomainError)
            } else {
                Ok(value.powf(1.0 / n))
            }
        }
        "deg" => Ok(values[0].to_degrees()),
        "rad" => Ok(values[0].to_radians()),
        _ => Err(EvalError::DomainError),
    }
}

fn trig(value: f64, unit: AngleUnit, f: fn(f64) -> f64) -> Result<f64, EvalError> {
    Ok(f(unit.to_radians(value)))
}

fn domain(ok: bool, value: f64) -> Result<f64, EvalError> {
    if ok {
        Ok(value)
    } else {
        Err(EvalError::DomainError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn basic_arithmetic() {
        let ctx = Context::new(AngleUnit::Degree);
        let expr = parse("1+1").unwrap();
        assert!((eval(&expr, &ctx).unwrap() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn sin_thirty_degrees() {
        let ctx = Context::new(AngleUnit::Degree);
        let expr = parse("sin(30)").unwrap();
        assert!((eval(&expr, &ctx).unwrap() - 0.5).abs() < 1e-9);
    }
}
