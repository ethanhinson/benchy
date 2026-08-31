use std::fmt;

#[derive(Debug)]
pub enum EvaError {
    Domain,
    MathDivideByZero,
    Parser(String),
    Syntax(String),
}

impl fmt::Display for EvaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvaError::Domain => write!(f, "Domain Error: Out of bounds!"),
            EvaError::MathDivideByZero => write!(f, "Math Error: Divide by zero error!"),
            EvaError::Parser(msg) => write!(f, "Parser Error: {msg}"),
            EvaError::Syntax(msg) => write!(f, "Syntax Error: {msg}"),
        }
    }
}

impl std::error::Error for EvaError {}
