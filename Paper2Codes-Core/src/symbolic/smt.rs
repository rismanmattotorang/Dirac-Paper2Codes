//! Minimal SMT-like reasoning utilities.
//!
//! The verification pipeline only needs lightweight reasoning about numeric
//! inequalities. Rather than depending on an external SMT solver, we implement
//! a deterministic subset that understands conjunctions of linear constraints
//! in a single theory of reals. The solver supports:
//!   - Parsing expressions such as `x >= 0`, `y < 10`, `x = 5`
//!   - Combining multiple constraints separated by `and` / `&&`
//!   - Checking satisfiability by maintaining interval bounds per variable
//!   - Verifying whether a property is implied by the accumulated bounds
//!   - Providing a simple model (assignment) when one exists

use crate::error::{Paper2CodesError, Result, VerificationError};
use std::collections::HashMap;

const EPSILON: f64 = 1e-9;

#[derive(Clone)]
pub struct SMTSolver {
    enabled: bool,
}

impl SMTSolver {
    pub fn new() -> Self {
        Self { enabled: true }
    }

    /// Check if the solver is available
    pub fn is_available(&self) -> bool {
        self.enabled
    }

    /// Enable the solver (noop for the pure Rust implementation)
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Check satisfiability of a conjunction of constraints
    pub async fn check_sat(&self, formula: &str) -> Result<bool> {
        ensure_enabled(self)?;

        let parsed = parse_constraints(formula)?;
        if parsed.always_false {
            return Ok(false);
        }
        if parsed.constraints.is_empty() {
            return Ok(true);
        }

        let mut system = ConstraintSystem::new();
        for constraint in parsed.constraints {
            if !system.apply(constraint)? {
                return Ok(false);
            }
        }

        Ok(system.is_satisfiable())
    }

    /// Verify whether a property is implied by the given constraints
    pub async fn verify_property(&self, property: &str, constraints: &[&str]) -> Result<bool> {
        ensure_enabled(self)?;

        if property.trim().is_empty() {
            return Err(Paper2CodesError::Validation(
                "Property cannot be empty".to_string(),
            ));
        }

        let mut system = ConstraintSystem::new();
        let mut inconsistent = false;

        for constraint_text in constraints {
            let parsed = parse_constraints(constraint_text)?;
            if parsed.always_false {
                inconsistent = true;
                break;
            }

            for constraint in parsed.constraints {
                if !system.apply(constraint)? {
                    inconsistent = true;
                    break;
                }
            }

            if inconsistent {
                break;
            }
        }

        if inconsistent || !system.is_satisfiable() {
            // Conflicting constraints -> property cannot be established.
            return Ok(false);
        }

        let property = parse_single_constraint(property)?;
        match system.implies(&property) {
            Some(true) => Ok(true),
            _ => Ok(false),
        }
    }

    /// Produce a simple satisfying assignment for a formula if one exists
    pub async fn get_model(&self, formula: &str) -> Result<Option<HashMap<String, String>>> {
        ensure_enabled(self)?;

        let parsed = parse_constraints(formula)?;
        if parsed.always_false {
            return Ok(None);
        }

        let mut system = ConstraintSystem::new();
        for constraint in parsed.constraints {
            if !system.apply(constraint)? {
                return Ok(None);
            }
        }

        if !system.is_satisfiable() {
            return Ok(None);
        }

        Ok(Some(system.build_model()))
    }
}

impl Default for SMTSolver {
    fn default() -> Self {
        Self::new()
    }
}

fn ensure_enabled(solver: &SMTSolver) -> Result<()> {
    if solver.enabled {
        Ok(())
    } else {
        Err(Paper2CodesError::Verification(
            VerificationError::SymbolicVerification(
                "SMT solver disabled. Enable it in configuration to perform symbolic checks."
                    .to_string(),
            ),
        ))
    }
}

// ----- Constraint parsing ---------------------------------------------------

#[derive(Clone, Copy, Debug)]
enum Comparator {
    Eq,
    Ge,
    Le,
    Gt,
    Lt,
}

impl Comparator {
    fn flip(self) -> Self {
        match self {
            Comparator::Eq => Comparator::Eq,
            Comparator::Ge => Comparator::Le,
            Comparator::Le => Comparator::Ge,
            Comparator::Gt => Comparator::Lt,
            Comparator::Lt => Comparator::Gt,
        }
    }
}

#[derive(Clone, Debug)]
struct Constraint {
    variable: String,
    comparator: Comparator,
    value: f64,
}

struct ParsedConstraints {
    constraints: Vec<Constraint>,
    always_false: bool,
}

fn parse_constraints(input: &str) -> Result<ParsedConstraints> {
    let mut constraints = Vec::new();
    let mut always_false = false;

    for clause in split_clauses(input) {
        let trimmed = clause.trim();
        if trimmed.is_empty() {
            continue;
        }

        if trimmed.eq_ignore_ascii_case("true") {
            continue;
        }

        if trimmed.eq_ignore_ascii_case("false") {
            always_false = true;
            break;
        }

        let constraint = parse_single_constraint(trimmed)?;
        constraints.push(constraint);
    }

    Ok(ParsedConstraints {
        constraints,
        always_false,
    })
}

fn parse_single_constraint(input: &str) -> Result<Constraint> {
    let operators = ["==", ">=", "<=", ">", "<", "="];
    let mut found_op = None;

    for op in &operators {
        if let Some(idx) = input.find(op) {
            found_op = Some((idx, *op));
            break;
        }
    }

    let (idx, op) = found_op.ok_or_else(|| {
        Paper2CodesError::Verification(VerificationError::SymbolicVerification(format!(
            "Unable to parse constraint '{}'",
            input
        )))
    })?;

    let (lhs, rhs) = (&input[..idx], &input[idx + op.len()..]);
    let comparator = match op {
        "==" | "=" => Comparator::Eq,
        ">=" => Comparator::Ge,
        "<=" => Comparator::Le,
        ">" => Comparator::Gt,
        "<" => Comparator::Lt,
        _ => unreachable!(),
    };

    normalise_constraint(lhs, rhs, comparator)
}

fn normalise_constraint(lhs: &str, rhs: &str, comparator: Comparator) -> Result<Constraint> {
    let rhs_value = rhs.trim().parse::<f64>().ok();
    let lhs_var = extract_variable(lhs.trim());

    if let (Some(var), Some(value)) = (lhs_var, rhs_value) {
        return Ok(adjust_for_sign(var, comparator, value));
    }

    let lhs_value = lhs.trim().parse::<f64>().ok();
    let rhs_var = extract_variable(rhs.trim());

    if let (Some(value), Some(var)) = (lhs_value, rhs_var) {
        let flipped = comparator.flip();
        return Ok(adjust_for_sign(var, flipped, value));
    }

    Err(Paper2CodesError::Verification(
        VerificationError::SymbolicVerification(format!(
            "Unsupported constraint format '{}'",
            format!("{} {:?} {}", lhs.trim(), comparator, rhs.trim())
        )),
    ))
}

fn adjust_for_sign(
    (variable, sign): (String, f64),
    comparator: Comparator,
    value: f64,
) -> Constraint {
    if sign >= 0.0 {
        Constraint {
            variable,
            comparator,
            value: value * sign,
        }
    } else {
        Constraint {
            variable,
            comparator: comparator.flip(),
            value: -value * sign,
        }
    }
}

fn extract_variable(text: &str) -> Option<(String, f64)> {
    let cleaned = text.trim();
    if cleaned.is_empty() {
        return None;
    }

    if cleaned.starts_with('-') {
        let var = cleaned[1..].trim();
        if is_variable(var) {
            return Some((var.to_string(), -1.0));
        }
    }

    if is_variable(cleaned) {
        return Some((cleaned.to_string(), 1.0));
    }

    None
}

fn is_variable(text: &str) -> bool {
    let mut chars = text.chars();
    match chars.next() {
        Some(ch) if ch.is_ascii_alphabetic() || ch == '_' => {
            chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        }
        _ => false,
    }
}

fn split_clauses(input: &str) -> Vec<String> {
    let mut clauses = vec![input.to_string()];
    for separator in ["&&", " and ", " AND ", " And ", "\n", ";", ","] {
        let mut next = Vec::new();
        for clause in clauses {
            next.extend(clause.split(separator).map(|s| s.to_string()));
        }
        clauses = next;
    }

    clauses
}

// ----- Constraint system ----------------------------------------------------

#[derive(Default)]
struct VariableBounds {
    lower: Option<f64>,
    upper: Option<f64>,
    strict_lower: bool,
    strict_upper: bool,
    equality: Option<f64>,
}

impl VariableBounds {
    fn apply(&mut self, constraint: &Constraint) -> Result<bool> {
        let value = constraint.value;
        match constraint.comparator {
            Comparator::Eq => {
                if let Some(existing) = self.equality {
                    if (existing - value).abs() > EPSILON {
                        return Ok(false);
                    }
                }
                self.equality = Some(value);
                self.lower = Some(value);
                self.upper = Some(value);
                self.strict_lower = false;
                self.strict_upper = false;
            }
            Comparator::Ge => match self.lower {
                Some(current) if value_greater(value, current) => {
                    self.lower = Some(value);
                    self.strict_lower = false;
                }
                None => {
                    self.lower = Some(value);
                    self.strict_lower = false;
                }
                _ => {}
            },
            Comparator::Gt => match self.lower {
                Some(current)
                    if value_greater(value, current)
                        || (value_close(value, current) && !self.strict_lower) =>
                {
                    self.lower = Some(value);
                    self.strict_lower = true;
                }
                None => {
                    self.lower = Some(value);
                    self.strict_lower = true;
                }
                _ => {}
            },
            Comparator::Le => match self.upper {
                Some(current) if value_less(value, current) => {
                    self.upper = Some(value);
                    self.strict_upper = false;
                }
                None => {
                    self.upper = Some(value);
                    self.strict_upper = false;
                }
                _ => {}
            },
            Comparator::Lt => match self.upper {
                Some(current)
                    if value_less(value, current)
                        || (value_close(value, current) && !self.strict_upper) =>
                {
                    self.upper = Some(value);
                    self.strict_upper = true;
                }
                None => {
                    self.upper = Some(value);
                    self.strict_upper = true;
                }
                _ => {}
            },
        }

        Ok(self.is_consistent())
    }

    fn is_consistent(&self) -> bool {
        if let Some(eq) = self.equality {
            if let Some(lower) = self.lower {
                if value_less(eq, lower) {
                    return false;
                }
                if value_close(eq, lower) && self.strict_lower {
                    return false;
                }
            }
            if let Some(upper) = self.upper {
                if value_greater(eq, upper) {
                    return false;
                }
                if value_close(eq, upper) && self.strict_upper {
                    return false;
                }
            }
        }

        match (self.lower, self.upper) {
            (Some(lower), Some(upper)) => {
                if value_greater(lower, upper) {
                    return false;
                }
                if value_close(lower, upper) && (self.strict_lower || self.strict_upper) {
                    return false;
                }
                true
            }
            _ => true,
        }
    }

    fn implies(&self, constraint: &Constraint) -> Option<bool> {
        match constraint.comparator {
            Comparator::Eq => {
                if let Some(eq) = self.equality {
                    Some(value_close(eq, constraint.value))
                } else {
                    None
                }
            }
            Comparator::Ge => self.lower.map(|lower| !value_less(lower, constraint.value)),
            Comparator::Gt => self.lower.map(|lower| {
                value_greater(lower, constraint.value)
                    || (value_close(lower, constraint.value) && !self.strict_lower)
            }),
            Comparator::Le => self
                .upper
                .map(|upper| !value_greater(upper, constraint.value)),
            Comparator::Lt => self.upper.map(|upper| {
                value_less(upper, constraint.value)
                    || (value_close(upper, constraint.value) && !self.strict_upper)
            }),
        }
    }

    fn choose_value(&self) -> f64 {
        if let Some(eq) = self.equality {
            return eq;
        }

        match (self.lower, self.upper) {
            (Some(lower), Some(upper)) => {
                if self.strict_lower && self.strict_upper && value_close(lower, upper) {
                    lower
                } else if self.strict_lower && !self.strict_upper {
                    lower + EPSILON
                } else if !self.strict_lower && self.strict_upper {
                    upper - EPSILON
                } else {
                    (lower + upper) / 2.0
                }
            }
            (Some(lower), None) => {
                if self.strict_lower {
                    lower + EPSILON
                } else {
                    lower
                }
            }
            (None, Some(upper)) => {
                if self.strict_upper {
                    upper - EPSILON
                } else {
                    upper
                }
            }
            (None, None) => 0.0,
        }
    }
}

struct ConstraintSystem {
    variables: HashMap<String, VariableBounds>,
}

impl ConstraintSystem {
    fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    fn apply(&mut self, constraint: Constraint) -> Result<bool> {
        let entry = self
            .variables
            .entry(constraint.variable.clone())
            .or_insert_with(VariableBounds::default);
        entry.apply(&constraint)
    }

    fn is_satisfiable(&self) -> bool {
        self.variables.values().all(|bounds| bounds.is_consistent())
    }

    fn implies(&self, constraint: &Constraint) -> Option<bool> {
        self.variables
            .get(&constraint.variable)
            .and_then(|bounds| bounds.implies(constraint))
    }

    fn build_model(&self) -> HashMap<String, String> {
        let mut model = HashMap::new();
        for (var, bounds) in &self.variables {
            let value = bounds.choose_value();
            model.insert(var.clone(), format_number(value));
        }
        model
    }
}

fn value_close(a: f64, b: f64) -> bool {
    (a - b).abs() < EPSILON
}

fn value_greater(a: f64, b: f64) -> bool {
    a - b > EPSILON
}

fn value_less(a: f64, b: f64) -> bool {
    b - a > EPSILON
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
