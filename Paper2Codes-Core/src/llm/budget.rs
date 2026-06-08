//! Per-run token budget (Phase 2 cost control).
//!
//! Bounds the number of LLM tokens a single run (e.g. one paper) may consume so
//! a pathological input cannot rack up unbounded spend. Thread-safe; intended to
//! be shared across the agents/router for a run and checked before each call.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{LLMError, Result};

/// A shared, atomic token budget for one run.
#[derive(Debug)]
pub struct TokenBudget {
    max: u64,
    used: AtomicU64,
}

impl TokenBudget {
    /// Create a budget with a maximum token count. `0` means unlimited.
    pub fn new(max_tokens: u64) -> Self {
        Self {
            max: max_tokens,
            used: AtomicU64::new(0),
        }
    }

    /// Reserve `tokens` against the budget. Returns an error (without consuming)
    /// if the reservation would exceed the limit. A budget of `0` is unlimited.
    pub fn try_consume(&self, tokens: u64) -> Result<()> {
        if self.max == 0 {
            self.used.fetch_add(tokens, Ordering::Relaxed);
            return Ok(());
        }
        // Atomic compare-and-swap loop to avoid overshoot under concurrency.
        let mut current = self.used.load(Ordering::Relaxed);
        loop {
            let next = current.saturating_add(tokens);
            if next > self.max {
                return Err(LLMError::InvalidRequest(format!(
                    "token budget exceeded: {} used + {} requested > {} limit",
                    current, tokens, self.max
                ))
                .into());
            }
            match self.used.compare_exchange_weak(
                current,
                next,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Ok(()),
                Err(observed) => current = observed,
            }
        }
    }

    pub fn consumed(&self) -> u64 {
        self.used.load(Ordering::Relaxed)
    }

    /// Remaining tokens (`u64::MAX` when unlimited).
    pub fn remaining(&self) -> u64 {
        if self.max == 0 {
            u64::MAX
        } else {
            self.max.saturating_sub(self.consumed())
        }
    }

    pub fn reset(&self) {
        self.used.store(0, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consumes_within_budget() {
        let b = TokenBudget::new(100);
        assert!(b.try_consume(40).is_ok());
        assert!(b.try_consume(60).is_ok());
        assert_eq!(b.consumed(), 100);
        assert_eq!(b.remaining(), 0);
    }

    #[test]
    fn rejects_overspend_without_consuming() {
        let b = TokenBudget::new(100);
        assert!(b.try_consume(80).is_ok());
        assert!(b.try_consume(30).is_err()); // would exceed
        assert_eq!(b.consumed(), 80); // unchanged by the failed attempt
        assert!(b.try_consume(20).is_ok()); // exactly to the limit
    }

    #[test]
    fn zero_means_unlimited() {
        let b = TokenBudget::new(0);
        assert!(b.try_consume(1_000_000).is_ok());
        assert_eq!(b.remaining(), u64::MAX);
    }

    #[test]
    fn reset_clears_usage() {
        let b = TokenBudget::new(10);
        b.try_consume(10).unwrap();
        b.reset();
        assert_eq!(b.consumed(), 0);
        assert!(b.try_consume(10).is_ok());
    }
}
