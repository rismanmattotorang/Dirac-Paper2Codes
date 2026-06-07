//! Integration tests for API endpoints
//!
//! This module contains comprehensive integration tests for the API server endpoints,
//! including health checks, CRUD operations, security tests, and performance tests.

mod common;

#[cfg(feature = "api")]
mod health_test;
#[cfg(feature = "api")]
mod papers_test;
#[cfg(feature = "api")]
mod repositories_test;
#[cfg(feature = "api")]
mod tasks_test;
#[cfg(feature = "api")]
mod security_test;
#[cfg(feature = "api")]
mod performance_test;
#[cfg(feature = "api")]
mod load_test;
#[cfg(feature = "api")]
mod e2e_test;
#[cfg(feature = "api")]
mod contract_test;
