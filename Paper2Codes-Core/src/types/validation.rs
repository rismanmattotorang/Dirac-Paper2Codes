use crate::error::Result;

/// Trait for types that can be validated
pub trait Validatable {
    /// Validate the type and return an error if invalid
    fn validate(&self) -> Result<()>;
}

/// Helper macro for implementing validation
#[macro_export]
macro_rules! impl_validatable {
    ($type:ty, $validate_fn:expr) => {
        impl $crate::types::validation::Validatable for $type {
            fn validate(&self) -> $crate::error::Result<()> {
                $validate_fn(self)
            }
        }
    };
}
