//! Cache key generation and management
//!
//! Provides standardized cache key generation for different resource types

use sha2::{Digest, Sha256};
use std::fmt;

/// Cache key for different resource types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CacheKey {
    /// Paper resource: `paper:{id}`
    Paper { id: String },
    /// Repository resource: `repo:{id}`
    Repository { id: String },
    /// Task resource: `task:{id}`
    Task { id: String },
    /// Search result: `search:{query_hash}`
    Search { query_hash: String },
    /// List resource with filters: `{resource}:list:{filter_hash}`
    List {
        resource: String,
        filter_hash: String,
    },
    /// Custom key: `custom:{key}`
    Custom { key: String },
}

impl CacheKey {
    /// Generate a cache key for a paper
    pub fn paper(id: &str) -> Self {
        Self::Paper { id: id.to_string() }
    }

    /// Generate a cache key for a repository
    pub fn repository(id: &str) -> Self {
        Self::Repository { id: id.to_string() }
    }

    /// Generate a cache key for a task
    pub fn task(id: &str) -> Self {
        Self::Task { id: id.to_string() }
    }

    /// Generate a cache key for a search query
    pub fn search(query: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(query.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        Self::Search { query_hash: hash }
    }

    /// Generate a cache key for a list with filters
    pub fn list(resource: &str, filters: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(filters.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        Self::List {
            resource: resource.to_string(),
            filter_hash: hash,
        }
    }

    /// Generate a custom cache key
    pub fn custom(key: &str) -> Self {
        Self::Custom {
            key: key.to_string(),
        }
    }

    /// Convert cache key to string representation
    pub fn to_string(&self) -> String {
        match self {
            Self::Paper { id } => format!("paper:{}", id),
            Self::Repository { id } => format!("repo:{}", id),
            Self::Task { id } => format!("task:{}", id),
            Self::Search { query_hash } => format!("search:{}", query_hash),
            Self::List {
                resource,
                filter_hash,
            } => {
                format!("{}:list:{}", resource, filter_hash)
            }
            Self::Custom { key } => format!("custom:{}", key),
        }
    }
}

impl fmt::Display for CacheKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_generation() {
        let paper_key = CacheKey::paper("123");
        assert_eq!(paper_key.to_string(), "paper:123");

        let repo_key = CacheKey::repository("456");
        assert_eq!(repo_key.to_string(), "repo:456");

        let task_key = CacheKey::task("789");
        assert_eq!(task_key.to_string(), "task:789");

        let search_key1 = CacheKey::search("test query");
        let search_key2 = CacheKey::search("test query");
        assert_eq!(search_key1.to_string(), search_key2.to_string());

        let list_key = CacheKey::list("papers", "page=1&limit=10");
        assert!(list_key.to_string().starts_with("papers:list:"));
    }
}
