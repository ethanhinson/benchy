use std::fmt;

#[derive(Debug)]
pub enum EvaError {
    Parser(String),
    Syntax(String),
    Domain,
    Math(String),
}

impl EvaError {
    pub fn parser(msg: impl Into<String>) -> Self {
        EvaError::Parser(msg.into())
    }

    pub fn syntax(msg: impl Into<String>) -> Self {
        EvaError::Syntax(msg.into())
    }
}

impl fmt::Display for EvaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvaError::Parser(msg) => write!(f, "Parser Error: {msg}"),
            EvaError::Syntax(msg) => write!(f, "Syntax Error: {msg}"),
            EvaError::Domain => write!(f, "Domain Error: Out of bounds!"),
            EvaError::Math(msg) => write!(f, "Math Error: {msg}"),
        }
    }
}
