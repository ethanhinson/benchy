use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LParen,
    RParen,
    Comma,
}

#[derive(Debug)]
pub enum LexError {
    UnexpectedChar(char),
    InvalidNumber,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LexError::UnexpectedChar(c) => write!(f, "unexpected character '{c}'"),
            LexError::InvalidNumber => write!(f, "invalid number"),
        }
    }
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, LexError> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }

        match c {
            '+' => tokens.push(Token::Plus),
            '-' => tokens.push(Token::Minus),
            '*' => {
                if i + 1 < chars.len() && chars[i + 1] == '*' {
                    tokens.push(Token::Caret);
                    i += 1;
                } else {
                    tokens.push(Token::Star);
                }
            }
            '/' => tokens.push(Token::Slash),
            '^' => tokens.push(Token::Caret),
            '(' => tokens.push(Token::LParen),
            ')' => tokens.push(Token::RParen),
            ',' => tokens.push(Token::Comma),
            c if c.is_ascii_digit() || c == '.' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let slice: String = chars[start..i].iter().collect();
                let value: f64 = slice.parse().map_err(|_| LexError::InvalidNumber)?;
                tokens.push(Token::Number(value));
                continue;
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len()
                    && (chars[i].is_ascii_alphanumeric() || chars[i] == '_')
                {
                    i += 1;
                }
                let ident: String = chars[start..i].iter().collect();
                tokens.push(Token::Ident(ident));
                continue;
            }
            other => return Err(LexError::UnexpectedChar(other)),
        }
        i += 1;
    }

    Ok(tokens)
}

pub fn insert_implicit_multiplication(tokens: &[Token]) -> Vec<Token> {
    let mut out = Vec::with_capacity(tokens.len() * 2);
    for (idx, token) in tokens.iter().enumerate() {
        if idx > 0 {
            let prev = &tokens[idx - 1];
            if needs_implicit_mul(prev, token) {
                out.push(Token::Star);
            }
        }
        out.push(token.clone());
    }
    out
}

fn needs_implicit_mul(prev: &Token, next: &Token) -> bool {
    if matches!(next, Token::LParen) {
        if let Token::Ident(name) = prev {
            if is_function_name(name) {
                return false;
            }
        }
    }

    matches!(
        prev,
        Token::Number(_) | Token::RParen | Token::Ident(_)
    ) && matches!(
        next,
        Token::LParen | Token::Ident(_) | Token::Number(_)
    )
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    Ident(String),
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        name: String,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Plus,
    Minus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

#[derive(Debug)]
pub enum ParseError {
    UnexpectedEof,
    UnexpectedToken,
    TooManyOperators,
    TooFewArgs { need: usize },
    TrailingTokens,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedEof | ParseError::UnexpectedToken => {
                write!(f, "Too many operators, too few operands")
            }
            ParseError::TooManyOperators => {
                write!(f, "Too many operators, too few operands")
            }
            ParseError::TooFewArgs { need } => {
                write!(f, "To few arguments for function, need {need}")
            }
            ParseError::TrailingTokens => {
                write!(f, "Too many operators, too few operands")
            }
        }
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_additive()?;
        if self.pos < self.tokens.len() {
            return Err(ParseError::TrailingTokens);
        }
        Ok(expr)
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_multiplicative()?;
        while matches!(self.peek(), Some(Token::Plus | Token::Minus)) {
            let op = match self.next().unwrap() {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Sub,
                _ => unreachable!(),
            };
            let right = self.parse_multiplicative()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_power()?;
        while matches!(self.peek(), Some(Token::Star | Token::Slash)) {
            let op = match self.next().unwrap() {
                Token::Star => BinaryOp::Mul,
                Token::Slash => BinaryOp::Div,
                _ => unreachable!(),
            };
            let right = self.parse_power()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_power(&mut self) -> Result<Expr, ParseError> {
        let left = self.parse_unary()?;
        if matches!(self.peek(), Some(Token::Caret)) {
            self.next();
            let right = self.parse_power()?;
            return Ok(Expr::Binary {
                op: BinaryOp::Pow,
                left: Box::new(left),
                right: Box::new(right),
            });
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        match self.peek().cloned() {
            Some(Token::Plus) => {
                self.next();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Plus,
                    expr: Box::new(expr),
                })
            }
            Some(Token::Minus) => {
                self.next();
                let expr = self.parse_unary()?;
                Ok(Expr::Unary {
                    op: UnaryOp::Minus,
                    expr: Box::new(expr),
                })
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.next() {
            Some(Token::Number(n)) => Ok(Expr::Number(n)),
            Some(Token::Ident(name)) => {
                if matches!(self.peek(), Some(Token::LParen)) {
                    self.next();
                    let args = self.parse_call_args(&name)?;
                    if !matches!(self.next(), Some(Token::RParen)) {
                        return Err(ParseError::UnexpectedEof);
                    }
                    Ok(Expr::Call { name, args })
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            Some(Token::LParen) => {
                let expr = self.parse_additive()?;
                if !matches!(self.next(), Some(Token::RParen)) {
                    return Err(ParseError::UnexpectedEof);
                }
                Ok(expr)
            }
            None => Err(ParseError::UnexpectedEof),
            Some(_) => Err(ParseError::UnexpectedToken),
        }
    }

    fn parse_call_args(&mut self, name: &str) -> Result<Vec<Expr>, ParseError> {
        if matches!(self.peek(), Some(Token::RParen)) {
            let need = expected_arg_count(name);
            return Err(ParseError::TooFewArgs { need });
        }

        let mut args = vec![self.parse_additive()?];
        while matches!(self.peek(), Some(Token::Comma)) {
            self.next();
            args.push(self.parse_additive()?);
        }

        let need = expected_arg_count(name);
        if args.len() < need {
            return Err(ParseError::TooFewArgs { need });
        }
        Ok(args)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let token = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(token)
        } else {
            None
        }
    }
}

pub fn is_function_name(name: &str) -> bool {
    matches!(
        name,
        "sin"
            | "cos"
            | "tan"
            | "csc"
            | "sec"
            | "cot"
            | "sinh"
            | "cosh"
            | "tanh"
            | "asin"
            | "acos"
            | "atan"
            | "acsc"
            | "asec"
            | "acot"
            | "ln"
            | "log2"
            | "log10"
            | "sqrt"
            | "ceil"
            | "floor"
            | "abs"
            | "log"
            | "nroot"
            | "deg"
            | "rad"
    )
}

fn expected_arg_count(name: &str) -> usize {
    match name {
        "log" | "nroot" => 2,
        _ => 1,
    }
}

pub fn balance_parentheses(input: &str) -> String {
    let mut balanced = input.to_string();
    let open = balanced.chars().filter(|&c| c == '(').count();
    let close = balanced.chars().filter(|&c| c == ')').count();
    if open > close {
        balanced.extend(std::iter::repeat(')').take(open - close));
    }
    balanced
}

pub fn parse(input: &str) -> Result<Expr, ParseError> {
    let tokens = tokenize(input).map_err(|_| ParseError::UnexpectedToken)?;
    if tokens.is_empty() {
        return Err(ParseError::UnexpectedEof);
    }
    let tokens = insert_implicit_multiplication(&tokens);
    let mut parser = Parser::new(tokens);
    parser.parse_expression()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implicit_multiplication() {
        let expr = parse("12sin(45(2))").unwrap();
        assert!(matches!(expr, Expr::Binary { op: BinaryOp::Mul, .. }));
    }

    #[test]
    fn power_right_associative() {
        let expr = parse("2^3^2").unwrap();
        if let Expr::Binary { right, .. } = expr {
            assert!(matches!(*right, Expr::Binary { op: BinaryOp::Pow, .. }));
        } else {
            panic!("expected power expression");
        }
    }
}
