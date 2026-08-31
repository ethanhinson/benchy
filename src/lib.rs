pub mod angle;
pub mod eval;
pub mod format;
pub mod parser;

pub use angle::AngleUnit;
pub use eval::{eval, Context, EvalError};
pub use format::format_value;
pub use parser::{balance_parentheses, parse, Expr, ParseError};

const VALID_CONSTANTS: &[&str] = &["e", "pi", "_"];

pub fn is_valid_ident(name: &str) -> bool {
    parser::is_function_name(name) || VALID_CONSTANTS.contains(&name)
}

mod parser_ext {
    use super::parser::{is_function_name, Expr, ParseError};

    pub fn validate_expr(expr: &Expr) -> Result<(), ParseError> {
        match expr {
            Expr::Ident(name)
                if !super::VALID_CONSTANTS.contains(&name.as_str())
                    && !is_function_name(name) =>
            {
                Err(ParseError::TooManyOperators)
            }
            Expr::Unary { expr, .. } => validate_expr(expr),
            Expr::Binary { left, right, .. } => {
                validate_expr(left)?;
                validate_expr(right)
            }
            Expr::Call { args, .. } => {
                for arg in args {
                    validate_expr(arg)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

pub fn parse_and_validate(input: &str) -> Result<parser::Expr, ParseError> {
    let expr = parse(input)?;
    parser_ext::validate_expr(&expr)?;
    Ok(expr)
}
