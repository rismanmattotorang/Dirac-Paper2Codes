//! Docker-based reproduction system
//!
//! Isolated execution environments using Docker for safe code reproduction and testing.

#[cfg(feature = "docker")]
pub mod docker;

#[cfg(feature = "docker")]
pub use docker::{ReproductionResult, ReproductionSystem};

#[cfg(not(feature = "docker"))]
pub struct ReproductionSystem;

#[cfg(not(feature = "docker"))]
impl ReproductionSystem {
    pub fn new() -> Self {
        Self
    }
}
