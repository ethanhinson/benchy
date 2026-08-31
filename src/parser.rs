use crate::error::EvaError;
use crate::eval::Evaluator;

const FUNCTIONS_1: &[&str] = &[
    "sin", "cos", "tan", "csc", "sec", "cot", "sinh", "cosh", "tanh", "asin", "acos", "atan",
    "acsc", "asec", "acot", "ln", "log2", "log10", "sqrt", "ceil", "floor", "abs", "deg", "rad",
];

const FUNCTIONS_2: &[&str] = &["log", "nroot"];

fn is_function_name(text: &str) -> bool {
    FUNCTIONS_1.contains(&text) || FUNCTIONS_2.contains(&text)
}

fn read_name_token(chars: &[char], mut i: usize) -> (String, usize) {
    let start = i;
    while i < chars.len() && is_ident_char(chars[i]) {
        i += 1;
    }
    if i < chars.len() && chars[i].is_ascii_digit() {
        let digit_start = i;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
        let candidate: String = chars[start..i].iter().collect();
        if is_function_name(&candidate) {
            return (candidate, i);
        }
        i = digit_start;
    }
    (chars[start..i].iter().collect(), i)
}

pub fn evaluate_expression(input: &str, evaluator: &mut Evaluator) -> Result<f64, EvaError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(0.0);
    }

    let preprocessed = preprocess(trimmed)?;
    let mut parser = Parser::new(&preprocessed, evaluator);
    let value = parser.parse_expression()?;
    parser.skip_whitespace();
    if !parser.is_at_end() {
        return Err(EvaError::parser("Too many operators, too few operands"));
    }
    Ok(value)
}

fn preprocess(input: &str) -> Result<String, EvaError> {
    let balanced = balance_parentheses(input)?;
    Ok(insert_implicit_multiplication(&balanced))
}

fn balance_parentheses(input: &str) -> Result<String, EvaError> {
    let open = input.chars().filter(|&c| c == '(').count();
    let close = input.chars().filter(|&c| c == ')').count();
    if close > open {
        return Err(EvaError::syntax("Mismatched parentheses!"));
    }
    let mut out = input.to_string();
    for _ in 0..open.saturating_sub(close) {
        out.push(')');
    }
    Ok(out)
}

fn insert_implicit_multiplication(input: &str) -> String {
    let tokens = tokenize_for_implicit_mul(input);
    let mut out = String::new();

    for (i, token) in tokens.iter().enumerate() {
        if i > 0 && needs_implicit_mul_between(&tokens[i - 1], token) {
            out.push('*');
        }
        out.push_str(token);
    }

    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ImplicitToken {
    Number,
    Ident,
    Function,
    LParen,
    RParen,
    Other,
}

fn tokenize_for_implicit_mul(input: &str) -> Vec<String> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }

        if c.is_ascii_digit() || (c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit()) {
            let start = i;
            if c == '.' {
                i += 1;
            }
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            tokens.push(chars[start..i].iter().collect());
            continue;
        }

        if is_ident_char(c) {
            let (ident, next) = read_name_token(&chars, i);
            i = next;
            tokens.push(ident);
            continue;
        }

        if c == '(' {
            tokens.push("(".to_string());
            i += 1;
            continue;
        }
        if c == ')' {
            tokens.push(")".to_string());
            i += 1;
            continue;
        }

        if c == '*' && i + 1 < chars.len() && chars[i + 1] == '*' {
            tokens.push("**".to_string());
            i += 2;
            continue;
        }

        tokens.push(c.to_string());
        i += 1;
    }

    tokens
}

fn token_kind(token: &str) -> ImplicitToken {
    match token {
        "(" => ImplicitToken::LParen,
        ")" => ImplicitToken::RParen,
        _ if token.chars().next().is_some_and(|c| c.is_ascii_digit() || c == '.') => {
            ImplicitToken::Number
        }
        _ if is_function_name(token) => ImplicitToken::Function,
        _ if token.chars().all(is_ident_char) => ImplicitToken::Ident,
        _ => ImplicitToken::Other,
    }
}

fn needs_implicit_mul_between(prev: &str, curr: &str) -> bool {
    let prev_kind = token_kind(prev);
    let curr_kind = token_kind(curr);

    matches!(prev_kind, ImplicitToken::Number | ImplicitToken::Ident | ImplicitToken::RParen)
        && matches!(
            curr_kind,
            ImplicitToken::Number
                | ImplicitToken::Ident
                | ImplicitToken::Function
                | ImplicitToken::LParen
        )
        && !(matches!(prev_kind, ImplicitToken::Ident | ImplicitToken::Function)
            && curr_kind == ImplicitToken::LParen)
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

struct Parser<'a> {
    chars: Vec<char>,
    pos: usize,
    evaluator: &'a mut Evaluator,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str, evaluator: &'a mut Evaluator) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
            evaluator,
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.advance();
        }
    }

    fn parse_expression(&mut self) -> Result<f64, EvaError> {
        let mut value = self.parse_term()?;
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some('+') => {
                    self.advance();
                    value += self.parse_term()?;
                }
                Some('-') => {
                    self.advance();
                    value -= self.parse_term()?;
                }
                _ => break,
            }
        }
        Ok(value)
    }

    fn parse_term(&mut self) -> Result<f64, EvaError> {
        let mut value = self.parse_power()?;
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some('*') => {
                    self.advance();
                    value *= self.parse_power()?;
                }
                Some('/') => {
                    self.advance();
                    value = Evaluator::eval_binary('/', value, self.parse_power()?)?;
                }
                _ => break,
            }
        }
        Ok(value)
    }

    fn parse_power(&mut self) -> Result<f64, EvaError> {
        let mut value = self.parse_unary()?;
        self.skip_whitespace();
        if matches!(self.peek(), Some('^')) {
            self.advance();
            if self.peek() == Some('*') && self.chars.get(self.pos + 1) == Some(&'*') {
                self.advance();
            }
            let rhs = self.parse_power()?;
            value = value.powf(rhs);
        }
        Ok(value)
    }

    fn parse_unary(&mut self) -> Result<f64, EvaError> {
        self.skip_whitespace();
        match self.peek() {
            Some('+') => {
                self.advance();
                Ok(self.parse_unary()?)
            }
            Some('-') => {
                self.advance();
                Ok(-self.parse_unary()?)
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<f64, EvaError> {
        let value = self.parse_primary()?;
        loop {
            self.skip_whitespace();
            if self.peek() == Some('(') {
                return Err(EvaError::parser("Too many operators, too few operands"));
            }
            break;
        }
        Ok(value)
    }

    fn parse_primary(&mut self) -> Result<f64, EvaError> {
        self.skip_whitespace();

        if self.peek() == Some('(') {
            self.advance();
            let value = self.parse_expression()?;
            self.skip_whitespace();
            if self.peek() != Some(')') {
                return Err(EvaError::syntax("Mismatched parentheses!"));
            }
            self.advance();
            return Ok(value);
        }

        if let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '.' {
                return self.parse_number();
            }
            if is_ident_char(c) {
                return self.parse_identifier_or_function();
            }
        }

        Err(EvaError::parser("Too many operators, too few operands"))
    }

    fn parse_number(&mut self) -> Result<f64, EvaError> {
        let start = self.pos;
        if self.peek() == Some('.') {
            self.advance();
        }
        while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '.') {
            self.advance();
        }
        let text: String = self.chars[start..self.pos].iter().collect();
        text.parse::<f64>()
            .map_err(|_| EvaError::parser("Invalid number"))
    }

    fn parse_identifier_or_function(&mut self) -> Result<f64, EvaError> {
        let start = self.pos;
        let (ident, next) = read_name_token(&self.chars, self.pos);
        self.pos = next;
        let _ = start;
        self.skip_whitespace();

        if self.peek() == Some('(') {
            return self.parse_function_call(&ident);
        }

        Evaluator::constant(&ident, self.evaluator.last_answer)
            .ok_or_else(|| EvaError::parser("Too many operators, too few operands"))
    }

    fn parse_function_call(&mut self, name: &str) -> Result<f64, EvaError> {
        if !is_function_name(name) {
            return Err(EvaError::parser("Too many operators, too few operands"));
        }

        self.advance(); // '('
        self.skip_whitespace();

        let arity = if FUNCTIONS_2.contains(&name) { 2 } else { 1 };
        let mut args = Vec::with_capacity(arity);

        if self.peek() == Some(')') {
            return Err(EvaError::parser(format!(
                "To few arguments for function, need {arity}"
            )));
        }

        args.push(self.parse_expression()?);
        self.skip_whitespace();

        if arity == 2 {
            if self.peek() != Some(',') {
                return Err(EvaError::parser("To few arguments for function, need 2"));
            }
            self.advance();
            self.skip_whitespace();
            args.push(self.parse_expression()?);
            self.skip_whitespace();
        }

        if self.peek() != Some(')') {
            return Err(EvaError::parser(format!(
                "To few arguments for function, need {arity}"
            )));
        }
        self.advance();

        if FUNCTIONS_1.contains(&name)
            && matches!(name, "sin" | "cos" | "tan" | "csc" | "sec" | "cot")
        {
            let rad = self.evaluator.apply_trig_input(name, args[0]);
            let result = match name {
                "sin" => rad.sin(),
                "cos" => rad.cos(),
                "tan" => rad.tan(),
                "csc" => 1.0 / rad.sin(),
                "sec" => 1.0 / rad.cos(),
                "cot" => 1.0 / rad.tan(),
                _ => unreachable!(),
            };
            return Evaluator::validate_function_result(name, &args, result);
        }

        let result = self.evaluator.eval_function(name, &args)?;
        Evaluator::validate_function_result(name, &args, result)
    }
}
