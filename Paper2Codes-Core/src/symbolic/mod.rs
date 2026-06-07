pub mod cas;
pub mod smt;

pub use cas::CAS;
pub use smt::SMTSolver;

use crate::error::Result;
use crate::types::CodeModule;

#[derive(Clone)]
pub struct SymbolicSolver {
    smt: Option<SMTSolver>,
    cas: Option<CAS>,
}

impl SymbolicSolver {
    pub fn new() -> Self {
        Self::from_enabled(true)
    }

    pub fn from_enabled(enable_symbolic: bool) -> Self {
        if enable_symbolic {
            Self {
                smt: Some(SMTSolver::new()),
                cas: Some(CAS::new()),
            }
        } else {
            Self {
                smt: None,
                cas: None,
            }
        }
    }

    pub fn disabled() -> Self {
        Self {
            smt: None,
            cas: None,
        }
    }

    /// Create a new SymbolicSolver with SMT solver enabled
    pub fn with_smt(mut self, smt: SMTSolver) -> Self {
        self.smt = Some(smt);
        self
    }

    /// Create a new SymbolicSolver with CAS enabled
    pub fn with_cas(mut self, cas: CAS) -> Self {
        self.cas = Some(cas);
        self
    }

    /// Check if SMT solver is available
    pub fn has_smt(&self) -> bool {
        self.smt.as_ref().map_or(false, |s| s.is_available())
    }

    pub fn is_enabled(&self) -> bool {
        self.smt.is_some() || self.cas.is_some()
    }

    /// Check if CAS is available
    pub fn has_cas(&self) -> bool {
        self.cas.as_ref().map_or(false, |c| c.is_available())
    }

    pub async fn verify_property(
        &self,
        code: &CodeModule,
        property: &str,
    ) -> Result<VerificationResult> {
        // Try to use SMT solver if available
        if let Some(ref smt) = self.smt {
            if smt.is_available() {
                // Attempt symbolic verification using SMT solver
                // For now, return unverified with clear message
                tracing::debug!("SMT solver available but verification not fully implemented for code: {}, property: {}", 
                    code.id, property);
                return Ok(VerificationResult {
                    verified: false,
                    counterexample: None,
                    proof: None,
                });
            }
        }

        // Graceful degradation: return unverified result with informative message
        tracing::warn!(
            "Symbolic verification requested but SMT solver not available. Code: {}, Property: {}",
            code.id,
            property
        );
        Ok(VerificationResult {
            verified: false,
            counterexample: None,
            proof: None,
        })
    }

    pub async fn simplify_expression(&self, expr: &str) -> Result<String> {
        // Validate input expression
        if expr.trim().is_empty() {
            return Err(crate::error::Paper2CodesError::Validation(
                "Expression cannot be empty".to_string(),
            ));
        }

        // Try to use CAS if available
        if let Some(ref cas) = self.cas {
            if cas.is_available() {
                return cas.simplify(expr).await;
            }
        }

        // Graceful degradation: return original expression with warning
        tracing::debug!(
            "CAS not available for simplification, returning original expression: {}",
            expr
        );
        Ok(expr.to_string())
    }

    pub async fn solve_equation(&self, equation: &str) -> Result<Vec<String>> {
        // Validate input equation
        if equation.trim().is_empty() {
            return Err(crate::error::Paper2CodesError::Validation(
                "Equation cannot be empty".to_string(),
            ));
        }

        // Try to use CAS if available
        if let Some(ref cas) = self.cas {
            if cas.is_available() {
                return cas.solve(equation).await;
            }
        }

        // Graceful degradation: return empty solutions with warning
        tracing::warn!("CAS not available for equation solving: {}", equation);
        Ok(Vec::new())
    }

    /// Expand a mathematical expression
    pub async fn expand_expression(&self, expr: &str) -> Result<String> {
        if expr.trim().is_empty() {
            return Err(crate::error::Paper2CodesError::Validation(
                "Expression cannot be empty".to_string(),
            ));
        }

        if let Some(ref cas) = self.cas {
            if cas.is_available() {
                return cas.expand(expr).await;
            }
        }

        tracing::debug!(
            "CAS not available for expansion, returning original expression: {}",
            expr
        );
        Ok(expr.to_string())
    }

    /// Differentiate an expression with respect to a variable
    pub async fn differentiate(&self, expr: &str, variable: &str) -> Result<String> {
        if expr.trim().is_empty() {
            return Err(crate::error::Paper2CodesError::Validation(
                "Expression cannot be empty".to_string(),
            ));
        }

        if variable.trim().is_empty() {
            return Err(crate::error::Paper2CodesError::Validation(
                "Variable cannot be empty".to_string(),
            ));
        }

        if let Some(ref cas) = self.cas {
            if cas.is_available() {
                return cas.differentiate(expr, variable).await;
            }
        }

        tracing::warn!(
            "CAS not available for differentiation: {} with respect to {}",
            expr,
            variable
        );
        Ok(expr.to_string())
    }

    /// Integrate an expression with respect to a variable
    pub async fn integrate(&self, expr: &str, variable: &str) -> Result<String> {
        if expr.trim().is_empty() {
            return Err(crate::error::Paper2CodesError::Validation(
                "Expression cannot be empty".to_string(),
            ));
        }

        if variable.trim().is_empty() {
            return Err(crate::error::Paper2CodesError::Validation(
                "Variable cannot be empty".to_string(),
            ));
        }

        if let Some(ref cas) = self.cas {
            if cas.is_available() {
                return cas.integrate(expr, variable).await;
            }
        }

        tracing::warn!(
            "CAS not available for integration: {} with respect to {}",
            expr,
            variable
        );
        Ok(expr.to_string())
    }
}

impl Default for SymbolicSolver {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub verified: bool,
    pub counterexample: Option<String>,
    pub proof: Option<String>,
}
