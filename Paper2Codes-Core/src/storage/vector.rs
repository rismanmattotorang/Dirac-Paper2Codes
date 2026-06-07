// Vector storage utilities
// This module provides helper functions for vector operations

use crate::storage::errors::StorageResult;

/// Calculate cosine similarity between two vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> StorageResult<f32> {
    if a.len() != b.len() {
        return Err(crate::storage::errors::StorageError::VectorSearch(
            "Vector dimensions do not match".to_string(),
        ));
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return Ok(0.0);
    }

    Ok(dot_product / (norm_a * norm_b))
}

/// Normalize a vector to unit length
pub fn normalize_vector(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

/// Validate embedding dimension
pub fn validate_embedding_dimension(embedding: &[f32], expected_dim: usize) -> StorageResult<()> {
    if embedding.len() != expected_dim {
        return Err(crate::storage::errors::StorageError::VectorSearch(format!(
            "Expected embedding dimension {}, got {}",
            expected_dim,
            embedding.len()
        )));
    }
    Ok(())
}
