use crate::error::EvaError;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number(f64),
    Const(String),
    Unary { op: char, expr: Box<Expr> },
    Binary {
        op: String,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call { name: String, args: Vec<Expr> },
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Ident(String),
    Op(String),
    LParen,
    RParen,
    Comma,
}

#[derive(Clone, Copy, PartialEq)]
enum TokenKind {
    Number,
    Ident,
    RParen,
    LParen,
    Op,
    Comma,
}

pub fn balance_parens(input: &str) -> String {
    let mut depth = 0i32;
    for c in input.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    let mut out = input.to_string();
    for _ in 0..depth {
        out.push(')');
    }
    out
}

fn token_kind(tok: &Token) -> TokenKind {
    match tok {
        Token::Number(_) => TokenKind::Number,
        Token::Ident(_) => TokenKind::Ident,
        Token::LParen => TokenKind::LParen,
        Token::RParen => TokenKind::RParen,
        Token::Op(_) => TokenKind::Op,
        Token::Comma => TokenKind::Comma,
    }
}

fn needs_mul(prev: &TokenKind, next: &TokenKind) -> bool {
    matches!(
        (prev, next),
        (TokenKind::Number, TokenKind::Ident)
            | (TokenKind::Number, TokenKind::LParen)
            | (TokenKind::RParen, TokenKind::Ident)
            | (TokenKind::RParen, TokenKind::LParen)
            | (TokenKind::RParen, TokenKind::Number)
    )
}

fn apply_implicit_mul(tokens: Vec<Token>) -> Vec<Token> {
    let mut out = Vec::new();
    let mut prev: Option<TokenKind> = None;

    for tok in tokens {
        let kind = token_kind(&tok);
        if let Some(p) = &prev {
            if needs_mul(p, &kind) {
                out.push(Token::Op("*".into()));
            }
        }
        out.push(tok);
        prev = Some(kind);
    }
    out
}

fn tokenize(input: &str) -> Result<Vec<Token>, EvaError> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }

        if c.is_ascii_digit() || (c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit())
        {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let num: f64 = input[start..i]
                .parse()
                .map_err(|_| EvaError::Parser("Too many operators, too few operands".into()))?;
            tokens.push(Token::Number(num));
            continue;
        }

        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            tokens.push(Token::Ident(input[start..i].to_string()));
            continue;
        }

        match c {
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            ',' => {
                tokens.push(Token::Comma);
                i += 1;
            }
            '+' | '-' | '*' | '/' | '^' => {
                tokens.push(Token::Op(c.to_string()));
                i += 1;
            }
            _ => {
                return Err(EvaError::Parser(
                    "Too many operators, too few operands".into(),
                ));
            }
        }
    }

    let mut merged = Vec::new();
    let mut idx = 0;
    while idx < tokens.len() {
        if let Token::Op(op) = &tokens[idx] {
            if op == "*" && idx + 1 < tokens.len() {
                if let Token::Op(op2) = &tokens[idx + 1] {
                    if op2 == "*" {
                        merged.push(Token::Op("**".into()));
                        idx += 2;
                        continue;
                    }
                }
            }
        }
        merged.push(tokens[idx].clone());
        idx += 1;
    }

    Ok(merged)
}

pub fn parse(input: &str) -> Result<Expr, EvaError> {
    let tokens = apply_implicit_mul(tokenize(input)?);
    let mut parser = Parser::new(tokens);
    let expr = parser.parse_expr()?;
    if parser.peek().is_some() {
        return Err(EvaError::Parser(
            "Too many operators, too few operands".into(),
        ));
    }
    Ok(expr)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        let tok = self.tokens.get(self.pos).cloned();
        if tok.is_some() {
            self.pos += 1;
        }
        tok
    }

    fn parse_expr(&mut self) -> Result<Expr, EvaError> {
        self.parse_add_sub()
    }

    fn parse_add_sub(&mut self) -> Result<Expr, EvaError> {
        let mut left = self.parse_mul_div()?;
        loop {
            let op = match self.peek() {
                Some(Token::Op(op)) if op == "+" || op == "-" => op.clone(),
                _ => break,
            };
            self.advance();
            let right = self.parse_mul_div()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_mul_div(&mut self) -> Result<Expr, EvaError> {
        let mut left = self.parse_power()?;
        loop {
            let op = match self.peek() {
                Some(Token::Op(op)) if op == "*" || op == "/" => op.clone(),
                _ => break,
            };
            self.advance();
            let right = self.parse_power()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_power(&mut self) -> Result<Expr, EvaError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Some(Token::Op(op)) if op == "^" || op == "**" => op.clone(),
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, EvaError> {
        let unary_op = match self.peek() {
            Some(Token::Op(op)) if op == "+" || op == "-" => Some(op.clone()),
            _ => None,
        };
        if let Some(op) = unary_op {
            let op_char = op.chars().next().unwrap();
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary {
                op: op_char,
                expr: Box::new(expr),
            });
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, EvaError> {
        match self.peek() {
            Some(Token::Number(n)) => {
                let n = *n;
                self.advance();
                Ok(Expr::Number(n))
            }
            Some(Token::Ident(name)) => {
                let name = name.clone();
                self.advance();
                if name == "pi" || name == "e" || name == "_" {
                    return Ok(Expr::Const(name));
                }
                if let Some(Token::LParen) = self.peek() {
                    self.advance();
                    let mut args = vec![self.parse_expr()?];
                    while let Some(Token::Comma) = self.peek() {
                        self.advance();
                        args.push(self.parse_expr()?);
                    }
                    if !matches!(self.advance(), Some(Token::RParen)) {
                        return Err(EvaError::Parser(
                            "Too many operators, too few operands".into(),
                        ));
                    }
                    Ok(Expr::Call { name, args })
                } else {
                    Err(EvaError::Syntax(format!(
                        "Function '{name}' expected parentheses"
                    )))
                }
            }
            Some(Token::LParen) => {
                self.advance();
                let expr = self.parse_expr()?;
                if !matches!(self.advance(), Some(Token::RParen)) {
                    return Err(EvaError::Parser(
                        "Too many operators, too few operands".into(),
                    ));
                }
                Ok(expr)
            }
            _ => Err(EvaError::Parser(
                "Too many operators, too few operands".into(),
            )),
        }
    }
}

pub fn prepare_input(raw: &str, repl: bool) -> String {
    let trimmed = raw.trim();
    if repl {
        balance_parens(trimmed)
    } else {
        trimmed.to_string()
    }
}
