//! Lightweight symbolic math utilities used by the verification pipeline.
//!
//! This module provides a self-contained computer algebra system (CAS) that
//! supports the subset of symbolic manipulation required by Paper2Codes. The
//! implementation intentionally focuses on deterministic behaviour and graceful
//! fallbacks rather than exhaustive coverage of all mathematical constructs.
//! Key capabilities:
//!   - Expression parsing with `+`, `-`, `*`, `/`, `^`, parentheses and symbols
//!   - Constant folding and algebraic simplification
//!   - Expansion of products of sums (basic distributive expansion)
//!   - Polynomial differentiation and integration (single variable)
//!   - Solving linear equations in a single variable
//!
//! The module keeps everything in Rust (no external interpreters) so it works
//! in restricted execution environments and during testing.

use crate::error::{Paper2CodesError, Result, VerificationError};
use std::collections::{HashMap, HashSet};

const EPSILON: f64 = 1e-9;

#[derive(Clone)]
pub struct CAS {
    enabled: bool,
}

impl CAS {
    pub fn new() -> Self {
        // Enabled by default because the implementation is self-contained.
        Self { enabled: true }
    }

    /// Check if CAS is available and enabled
    pub fn is_available(&self) -> bool {
        self.enabled
    }

    /// Enable CAS (noop for the pure Rust implementation)
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Simplify a mathematical expression
    pub async fn simplify(&self, expr: &str) -> Result<String> {
        ensure_enabled(self)?;
        let parsed = parse_expression(expr)?;
        let simplified = parsed.simplify();
        Ok(simplified.to_pretty_string())
    }

    /// Solve an equation. Currently supports single-variable linear equations.
    pub async fn solve(&self, equation: &str) -> Result<Vec<String>> {
        ensure_enabled(self)?;
        let (lhs, rhs) = split_equation(equation)?;
        let lhs_expr = parse_expression(lhs)?;
        let rhs_expr = parse_expression(rhs)?;
        let diff = Expr::Add(vec![
            lhs_expr,
            Expr::Mul(vec![Expr::Number(-1.0), rhs_expr]),
        ])
        .simplify();

        let variables = diff.variables();
        if variables.is_empty() {
            if diff.is_zero() {
                return Ok(vec!["All real numbers".to_string()]);
            }
            return Err(cas_error(format!(
                "Equation '{}' has no variables; cannot solve.",
                equation
            )));
        }

        if variables.len() > 1 {
            return Err(cas_error("Only single-variable equations are supported"));
        }

        let var = variables.into_iter().next().unwrap();
        let terms = diff
            .as_polynomial_terms(&var)
            .ok_or_else(|| cas_error("Equation is not a polynomial that the solver can handle"))?;

        // Expect ax + b = 0
        let mut coefficient = 0.0;
        let mut constant = 0.0;
        for (coeff, power) in terms {
            if (power - 1.0).abs() < EPSILON {
                coefficient += coeff;
            } else if power.abs() < EPSILON {
                constant += coeff;
            } else {
                return Err(cas_error(
                    "Only linear equations in a single variable are supported",
                ));
            }
        }

        if coefficient.abs() < EPSILON {
            if constant.abs() < EPSILON {
                return Ok(vec!["All real numbers".to_string()]);
            }
            return Err(cas_error("No solution exists for the provided equation"));
        }

        let solution = -constant / coefficient;
        Ok(vec![format_solution(&var, solution)])
    }

    /// Expand an expression (basic distributive expansion)
    pub async fn expand(&self, expr: &str) -> Result<String> {
        ensure_enabled(self)?;
        let parsed = parse_expression(expr)?;
        let expanded = parsed.expand().simplify();
        Ok(expanded.to_pretty_string())
    }

    /// Differentiate a polynomial expression with respect to a variable
    pub async fn differentiate(&self, expr: &str, variable: &str) -> Result<String> {
        ensure_enabled(self)?;
        let parsed = parse_expression(expr)?;
        let simplified = parsed.simplify();
        let derivative = simplified.differentiate(variable).ok_or_else(|| {
            cas_error("Differentiation currently supports single-variable polynomials only")
        })?;
        Ok(derivative.simplify().to_pretty_string())
    }

    /// Integrate a polynomial expression with respect to a variable
    pub async fn integrate(&self, expr: &str, variable: &str) -> Result<String> {
        ensure_enabled(self)?;
        let parsed = parse_expression(expr)?;
        let simplified = parsed.simplify();
        let integral = simplified.integrate(variable).ok_or_else(|| {
            cas_error("Integration currently supports single-variable polynomials only")
        })?;
        Ok(integral.simplify().to_pretty_string())
    }
}

impl Default for CAS {
    fn default() -> Self {
        Self::new()
    }
}

fn ensure_enabled(cas: &CAS) -> Result<()> {
    if cas.enabled {
        Ok(())
    } else {
        Err(Paper2CodesError::Verification(
            VerificationError::SymbolicVerification(
                "CAS integration disabled. Enable it via configuration to use symbolic features."
                    .to_string(),
            ),
        ))
    }
}

fn cas_error(message: impl Into<String>) -> Paper2CodesError {
    Paper2CodesError::Verification(VerificationError::SymbolicVerification(message.into()))
}

fn format_solution(variable: &str, value: f64) -> String {
    format!("{} = {}", variable, format_number(value))
}

fn format_number(value: f64) -> String {
    if value.abs() < EPSILON {
        "0".to_string()
    } else if (value - value.round()).abs() < EPSILON {
        format!("{}", value.round() as i64)
    } else {
        let mut s = format!("{:.12}", value);
        while s.contains('.') && s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
        s
    }
}

fn split_equation(equation: &str) -> Result<(&str, &str)> {
    if let Some(idx) = equation.find('=') {
        let lhs = equation[..idx].trim();
        let rhs = equation[idx + 1..].trim();
        if lhs.is_empty() || rhs.is_empty() {
            Err(cas_error("Equation must contain expressions on both sides"))
        } else {
            Ok((lhs, rhs))
        }
    } else {
        Err(cas_error("Equation must contain '='"))
    }
}

fn parse_expression(input: &str) -> Result<Expr> {
    let tokens = Lexer::new(input).tokenise()?;
    let mut parser = Parser::new(tokens);
    parser.parse_expression().map_err(|e| cas_error(e))
}

// ----- Expression parsing and manipulation ---------------------------------

#[derive(Debug, Clone)]
enum Token {
    Number(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LParen,
    RParen,
}

struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
        }
    }

    fn tokenise(mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        while let Some(&ch) = self.chars.peek() {
            match ch {
                c if c.is_whitespace() => {
                    self.chars.next();
                }
                c if c.is_ascii_digit() || c == '.' => {
                    tokens.push(self.consume_number()?);
                }
                c if is_identifier_start(c) => {
                    tokens.push(self.consume_identifier());
                }
                '+' => {
                    self.chars.next();
                    tokens.push(Token::Plus);
                }
                '-' => {
                    self.chars.next();
                    tokens.push(Token::Minus);
                }
                '*' => {
                    self.chars.next();
                    tokens.push(Token::Star);
                }
                '/' => {
                    self.chars.next();
                    tokens.push(Token::Slash);
                }
                '^' => {
                    self.chars.next();
                    tokens.push(Token::Caret);
                }
                '(' => {
                    self.chars.next();
                    tokens.push(Token::LParen);
                }
                ')' => {
                    self.chars.next();
                    tokens.push(Token::RParen);
                }
                _ => {
                    return Err(cas_error(format!(
                        "Unsupported character '{}' in expression",
                        ch
                    )));
                }
            }
        }
        Ok(tokens)
    }

    fn consume_number(&mut self) -> Result<Token> {
        let mut number = String::new();
        let mut has_decimal = false;
        while let Some(&ch) = self.chars.peek() {
            if ch.is_ascii_digit() {
                number.push(ch);
                self.chars.next();
            } else if ch == '.' && !has_decimal {
                has_decimal = true;
                number.push(ch);
                self.chars.next();
            } else {
                break;
            }
        }

        if number == "." {
            return Err(cas_error("Invalid number '.'"));
        }

        let value: f64 = number.parse().map_err(|_| cas_error("Invalid number"))?;
        Ok(Token::Number(value))
    }

    fn consume_identifier(&mut self) -> Token {
        let mut ident = String::new();
        while let Some(&ch) = self.chars.peek() {
            if is_identifier_part(ch) {
                ident.push(ch);
                self.chars.next();
            } else {
                break;
            }
        }
        Token::Ident(ident)
    }
}

fn is_identifier_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_identifier_part(ch: char) -> bool {
    is_identifier_start(ch) || ch.is_ascii_digit()
}

#[derive(Debug, Clone)]
enum Expr {
    Number(f64),
    Var(String),
    Add(Vec<Expr>),
    Mul(Vec<Expr>),
    Pow(Box<Expr>, Box<Expr>),
}

impl Expr {
    fn simplify(self) -> Expr {
        match self {
            Expr::Number(n) => Expr::Number(trim_small(n)),
            Expr::Var(name) => Expr::Var(name),
            Expr::Add(terms) => {
                let mut flattened = Vec::new();
                let mut constant = 0.0;
                for term in terms.into_iter().map(|t| t.simplify()) {
                    match term {
                        Expr::Number(n) => constant += n,
                        Expr::Add(inner) => flattened.extend(inner),
                        other => flattened.push(other),
                    }
                }

                if constant.abs() > EPSILON {
                    flattened.push(Expr::Number(constant));
                }

                match flattened.len() {
                    0 => Expr::Number(0.0),
                    1 => flattened.into_iter().next().unwrap(),
                    _ => Expr::Add(flattened),
                }
            }
            Expr::Mul(factors) => {
                let mut flattened = Vec::new();
                let mut constant = 1.0;

                for factor in factors.into_iter().map(|f| f.simplify()) {
                    match factor {
                        Expr::Number(n) => constant *= n,
                        Expr::Mul(inner) => flattened.extend(inner),
                        other => flattened.push(other),
                    }
                }

                if constant.abs() < EPSILON {
                    return Expr::Number(0.0);
                }

                if (constant - 1.0).abs() > EPSILON {
                    flattened.insert(0, Expr::Number(constant));
                }

                if flattened.is_empty() {
                    Expr::Number(constant)
                } else if flattened.len() == 1 {
                    flattened.into_iter().next().unwrap()
                } else {
                    Expr::Mul(flattened)
                }
            }
            Expr::Pow(base, exponent) => {
                let base = base.simplify();
                let exponent = exponent.simplify();
                match (&base, &exponent) {
                    (_, Expr::Number(exp)) if exp.abs() < EPSILON => Expr::Number(1.0),
                    (_, Expr::Number(exp)) if (exp - 1.0).abs() < EPSILON => base,
                    (Expr::Number(base_num), Expr::Number(exp_num)) => {
                        Expr::Number(base_num.powf(*exp_num))
                    }
                    _ => Expr::Pow(Box::new(base), Box::new(exponent)),
                }
            }
        }
    }

    fn expand(self) -> Expr {
        match self {
            Expr::Add(terms) => {
                Expr::Add(terms.into_iter().map(|t| t.expand()).collect()).simplify()
            }
            Expr::Mul(mut factors) => {
                if factors.is_empty() {
                    return Expr::Number(1.0);
                }
                let first = factors.remove(0).expand();
                let rest = Expr::Mul(factors).expand();
                distribute(first, rest).simplify()
            }
            Expr::Pow(base, exponent) => {
                let base = base.expand();
                let exponent = exponent.expand();
                if let Expr::Number(exp) = exponent {
                    if exp >= 0.0 && exp.fract().abs() < EPSILON && exp <= 8.0 {
                        let mut result = Expr::Number(1.0);
                        for _ in 0..(exp as usize) {
                            result = distribute(result, base.clone()).simplify();
                        }
                        result
                    } else {
                        Expr::Pow(Box::new(base), Box::new(Expr::Number(exp)))
                    }
                } else {
                    Expr::Pow(Box::new(base), Box::new(exponent))
                }
            }
            other => other,
        }
    }

    fn variables(&self) -> HashSet<String> {
        let mut vars = HashSet::new();
        self.collect_variables(&mut vars);
        vars
    }

    fn collect_variables(&self, vars: &mut HashSet<String>) {
        match self {
            Expr::Number(_) => {}
            Expr::Var(name) => {
                vars.insert(name.clone());
            }
            Expr::Add(terms) | Expr::Mul(terms) => {
                for term in terms {
                    term.collect_variables(vars);
                }
            }
            Expr::Pow(base, exponent) => {
                base.collect_variables(vars);
                exponent.collect_variables(vars);
            }
        }
    }

    fn is_zero(&self) -> bool {
        matches!(self, Expr::Number(value) if value.abs() < EPSILON)
    }

    fn as_polynomial_terms(&self, var: &str) -> Option<Vec<(f64, f64)>> {
        let mut terms = Vec::new();
        match self {
            Expr::Add(items) => {
                for item in items {
                    let part = item.as_polynomial_terms(var)?;
                    terms.extend(part);
                }
            }
            _ => {
                let (coeff, power) = term_to_coeff_power(self, var)?;
                terms.push((coeff, power));
            }
        }

        // Merge like terms (same power)
        let mut merged: HashMap<i64, f64> = HashMap::new();
        for (coeff, power) in terms {
            let key = ((power * 1_000_000.0).round() as i64).clamp(-1_000_000_000, 1_000_000_000);
            *merged.entry(key).or_insert(0.0) += coeff;
        }

        let mut result = Vec::new();
        for (key, coeff) in merged {
            let power = (key as f64) / 1_000_000.0;
            if coeff.abs() > EPSILON {
                result.push((coeff, power));
            }
        }

        Some(result)
    }

    fn differentiate(&self, var: &str) -> Option<Expr> {
        let terms = self.as_polynomial_terms(var)?;
        let mut derived_terms = Vec::new();
        for (coeff, power) in terms {
            if power.abs() < EPSILON {
                continue; // derivative of constant is 0
            }
            derived_terms.push((coeff * power, power - 1.0));
        }
        Some(Expr::from_polynomial_terms(&derived_terms, var))
    }

    fn integrate(&self, var: &str) -> Option<Expr> {
        let terms = self.as_polynomial_terms(var)?;
        let mut integrated_terms = Vec::new();
        for (coeff, power) in terms {
            let new_power = power + 1.0;
            if new_power.abs() < EPSILON {
                // Division by zero (integral of x^-1) is not supported in polynomial mode
                return None;
            }
            integrated_terms.push((coeff / new_power, new_power));
        }
        Some(Expr::from_polynomial_terms(&integrated_terms, var))
    }

    fn to_pretty_string(&self) -> String {
        match self {
            Expr::Number(n) => format_number(*n),
            Expr::Var(name) => name.clone(),
            Expr::Add(terms) => {
                let mut parts = Vec::new();
                for term in terms {
                    let s = term.to_pretty_string();
                    if s.starts_with('-') {
                        parts.push(format!("({})", s));
                    } else {
                        parts.push(s);
                    }
                }
                parts.join(" + ")
            }
            Expr::Mul(factors) => {
                let mut parts = Vec::new();
                for factor in factors {
                    match factor {
                        Expr::Add(_) => parts.push(format!("({})", factor.to_pretty_string())),
                        Expr::Mul(_) => parts.push(format!("({})", factor.to_pretty_string())),
                        other => parts.push(other.to_pretty_string()),
                    }
                }
                parts.join(" * ")
            }
            Expr::Pow(base, exponent) => {
                let base_str = match &**base {
                    Expr::Number(_) | Expr::Var(_) => base.to_pretty_string(),
                    _ => format!("({})", base.to_pretty_string()),
                };
                let exponent_str = match &**exponent {
                    Expr::Number(_) | Expr::Var(_) => exponent.to_pretty_string(),
                    _ => format!("({})", exponent.to_pretty_string()),
                };
                format!("{}^{}", base_str, exponent_str)
            }
        }
    }

    fn from_polynomial_terms(terms: &[(f64, f64)], var: &str) -> Expr {
        if terms.is_empty() {
            return Expr::Number(0.0);
        }

        let mut exprs = Vec::new();
        for (coeff, power) in terms {
            if coeff.abs() < EPSILON {
                continue;
            }

            let base = Expr::Var(var.to_string());
            let term = if power.abs() < EPSILON {
                Expr::Number(*coeff)
            } else if (power - 1.0).abs() < EPSILON {
                if (*coeff - 1.0).abs() < EPSILON {
                    base.clone()
                } else if (*coeff + 1.0).abs() < EPSILON {
                    Expr::Mul(vec![Expr::Number(-1.0), base.clone()])
                } else {
                    Expr::Mul(vec![Expr::Number(*coeff), base.clone()])
                }
            } else {
                let power_expr = Expr::Pow(Box::new(base.clone()), Box::new(Expr::Number(*power)));
                if (*coeff - 1.0).abs() < EPSILON {
                    power_expr
                } else if (*coeff + 1.0).abs() < EPSILON {
                    Expr::Mul(vec![Expr::Number(-1.0), power_expr])
                } else {
                    Expr::Mul(vec![Expr::Number(*coeff), power_expr])
                }
            };
            exprs.push(term);
        }

        if exprs.is_empty() {
            Expr::Number(0.0)
        } else if exprs.len() == 1 {
            exprs.into_iter().next().unwrap()
        } else {
            Expr::Add(exprs).simplify()
        }
    }
}

fn term_to_coeff_power(expr: &Expr, var: &str) -> Option<(f64, f64)> {
    match expr {
        Expr::Number(n) => Some((*n, 0.0)),
        Expr::Var(name) => {
            if name == var {
                Some((1.0, 1.0))
            } else {
                None
            }
        }
        Expr::Mul(factors) => {
            let mut coeff = 1.0;
            let mut power = 0.0;
            for factor in factors {
                match factor {
                    Expr::Number(n) => coeff *= n,
                    Expr::Var(name) if name == var => power += 1.0,
                    Expr::Pow(base, exponent) => match (&**base, &**exponent) {
                        (Expr::Var(name), Expr::Number(exp)) if name == var => power += *exp,
                        (Expr::Number(n), Expr::Number(exp)) => {
                            coeff *= n.powf(*exp);
                        }
                        _ => return None,
                    },
                    other => {
                        let (inner_coeff, inner_power) = term_to_coeff_power(other, var)?;
                        coeff *= inner_coeff;
                        power += inner_power;
                    }
                }
            }
            Some((coeff, power))
        }
        Expr::Pow(base, exponent) => match (&**base, &**exponent) {
            (Expr::Var(name), Expr::Number(exp)) if name == var => Some((1.0, *exp)),
            (Expr::Number(n), Expr::Number(exp)) => Some((n.powf(*exp), 0.0)),
            _ => None,
        },
        Expr::Add(_) => None,
    }
}

fn distribute(a: Expr, b: Expr) -> Expr {
    let terms_a = match a {
        Expr::Add(items) => items,
        other => vec![other],
    };
    let terms_b = match b {
        Expr::Add(items) => items,
        other => vec![other],
    };

    let mut products = Vec::new();
    for term_a in &terms_a {
        for term_b in &terms_b {
            products.push(Expr::Mul(vec![term_a.clone(), term_b.clone()]).simplify());
        }
    }

    Expr::Add(products).simplify()
}

fn trim_small(value: f64) -> f64 {
    if value.abs() < EPSILON {
        0.0
    } else {
        value
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn parse_expression(&mut self) -> std::result::Result<Expr, String> {
        self.parse_add_sub()
    }

    fn parse_add_sub(&mut self) -> std::result::Result<Expr, String> {
        let mut expr = self.parse_mul_div()?;
        while let Some(token) = self.peek() {
            match token {
                Token::Plus => {
                    self.advance();
                    let rhs = self.parse_mul_div()?;
                    expr = Expr::Add(vec![expr, rhs]).simplify();
                }
                Token::Minus => {
                    self.advance();
                    let rhs = self.parse_mul_div()?;
                    expr =
                        Expr::Add(vec![expr, Expr::Mul(vec![Expr::Number(-1.0), rhs])]).simplify();
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_mul_div(&mut self) -> std::result::Result<Expr, String> {
        let mut expr = self.parse_power()?;
        while let Some(token) = self.peek() {
            match token {
                Token::Star => {
                    self.advance();
                    let rhs = self.parse_power()?;
                    expr = Expr::Mul(vec![expr, rhs]).simplify();
                }
                Token::Slash => {
                    self.advance();
                    let rhs = self.parse_power()?;
                    expr = Expr::Mul(vec![
                        expr,
                        Expr::Pow(Box::new(rhs), Box::new(Expr::Number(-1.0))),
                    ])
                    .simplify();
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_power(&mut self) -> std::result::Result<Expr, String> {
        let mut expr = self.parse_unary()?;
        while let Some(Token::Caret) = self.peek() {
            self.advance();
            let exponent = self.parse_unary()?;
            expr = Expr::Pow(Box::new(expr), Box::new(exponent)).simplify();
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> std::result::Result<Expr, String> {
        if let Some(Token::Minus) = self.peek() {
            self.advance();
            let inner = self.parse_unary()?;
            Ok(Expr::Mul(vec![Expr::Number(-1.0), inner]).simplify())
        } else {
            self.parse_primary()
        }
    }

    fn parse_primary(&mut self) -> std::result::Result<Expr, String> {
        match self.next() {
            Some(Token::Number(n)) => Ok(Expr::Number(n)),
            Some(Token::Ident(name)) => Ok(Expr::Var(name)),
            Some(Token::LParen) => {
                let expr = self.parse_expression()?;
                match self.next() {
                    Some(Token::RParen) => Ok(expr),
                    _ => Err("Expected ')'".to_string()),
                }
            }
            Some(token) => Err(format!("Unexpected token {:?}", token)),
            None => Err("Unexpected end of expression".to_string()),
        }
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

    fn advance(&mut self) {
        self.pos += 1;
    }
}
