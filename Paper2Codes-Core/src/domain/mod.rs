//! Domain detection and domain-aware code generation
//!
//! This module provides automatic detection of computational domains in research papers
//! and domain-specific code generation capabilities.

pub mod detector;
pub mod templates;

pub use detector::{ComputationalDomain, DomainDetector};
pub use templates::DomainTemplates;
