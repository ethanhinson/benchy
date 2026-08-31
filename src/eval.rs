use std::f64::consts::{E, PI};

use crate::error::EvaError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AngleUnit {
    Degree,
    Radian,
    Gradian,
}

impl AngleUnit {
    pub fn to_radians(self, angle: f64) -> f64 {
        match self {
            AngleUnit::Degree | AngleUnit::Gradian => angle.to_radians(),
            AngleUnit::Radian => angle,
        }
    }
}

pub struct Evaluator {
    pub angle_unit: AngleUnit,
    pub last_answer: f64,
}

impl Evaluator {
    pub fn new(angle_unit: AngleUnit) -> Self {
        Self {
            angle_unit,
            last_answer: 0.0,
        }
    }

    pub fn eval_function(&self, name: &str, args: &[f64]) -> Result<f64, EvaError> {
        match name {
            "sin" => Ok(args[0].sin()),
            "cos" => Ok(args[0].cos()),
            "tan" => Ok(args[0].tan()),
            "csc" => Ok(1.0 / args[0].sin()),
            "sec" => Ok(1.0 / args[0].cos()),
            "cot" => Ok(1.0 / args[0].tan()),
            "sinh" => Ok(args[0].sinh()),
            "cosh" => Ok(args[0].cosh()),
            "tanh" => Ok(args[0].tanh()),
            "asin" => Ok(args[0].asin()),
            "acos" => Ok(args[0].acos()),
            "atan" => Ok(args[0].atan()),
            "acsc" => Ok((1.0 / args[0]).asin()),
            "asec" => Ok((1.0 / args[0]).acos()),
            "acot" => Ok((1.0 / args[0]).atan()),
            "ln" => Ok(args[0].ln()),
            "log2" => Ok(args[0].log2()),
            "log10" => Ok(args[0].log10()),
            "sqrt" => Ok(args[0].sqrt()),
            "ceil" => Ok(args[0].ceil()),
            "floor" => Ok(args[0].floor()),
            "abs" => Ok(args[0].abs()),
            "deg" => Ok(args[0].to_degrees()),
            "rad" => Ok(args[0].to_radians()),
            "log" => {
                if args[1] == 0.0 && args[0] > 0.0 {
                    Ok(-0.0)
                } else {
                    Ok(args[0].log(args[1]))
                }
            }
            "nroot" => Ok(args[0].powf(1.0 / args[1])),
            _ => Err(EvaError::parser(format!("Unknown function {name}"))),
        }
    }

    pub fn apply_trig_input(&self, name: &str, x: f64) -> f64 {
        match name {
            "sin" | "cos" | "tan" | "csc" | "sec" | "cot" => self.angle_unit.to_radians(x),
            _ => x,
        }
    }

    pub fn constant(name: &str, last_answer: f64) -> Option<f64> {
        match name {
            "pi" => Some(PI),
            "e" => Some(E),
            "_" => Some(last_answer),
            _ => None,
        }
    }

    pub fn validate_function_result(name: &str, args: &[f64], result: f64) -> Result<f64, EvaError> {
        if !result.is_finite() {
            return Err(EvaError::Domain);
        }

        match name {
            "ln" | "log2" | "log10" | "sqrt" if args[0] < 0.0 => Err(EvaError::Domain),
            "log" if args[0] <= 0.0 => Err(EvaError::Domain),
            "log" if args[1] == 0.0 && args[0] > 0.0 => Ok(-0.0),
            "asin" | "acos" if args[0] < -1.0 || args[0] > 1.0 => Err(EvaError::Domain),
            "acsc" | "asec" | "acot" if args[0] == 0.0 => Err(EvaError::Domain),
            "csc" | "sec" | "cot" | "tan" if !result.is_finite() => Ok(result),
            _ => Ok(result),
        }
    }

    pub fn eval_binary(op: char, left: f64, right: f64) -> Result<f64, EvaError> {
        let value = match op {
            '+' => left + right,
            '-' => left - right,
            '*' => left * right,
            '/' => {
                if right == 0.0 {
                    return Err(EvaError::Math("Divide by zero error!".into()));
                }
                left / right
            }
            '^' => left.powf(right),
            _ => return Err(EvaError::parser("Unknown operator")),
        };
        Ok(value)
    }
}
