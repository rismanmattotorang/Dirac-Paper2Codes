//! Cache invalidation strategies
//!
//! Provides intelligent cache invalidation based on resource relationships

use super::keys::CacheKey;
use super::InMemoryCache;
use std::sync::Arc;
use tracing::{debug, info};

/// Cache invalidator with support for pattern-based invalidation
pub struct CacheInvalidator {
    cache: Arc<InMemoryCache>,
}

impl CacheInvalidator {
    /// Create a new cache invalidator
    pub fn new(cache: Arc<InMemoryCache>) -> Self {
        Self { cache }
    }

    /// Invalidate a specific resource
    pub async fn invalidate_resource(&self, key: &CacheKey) {
        debug!(key = %key, "Invalidating resource");
        self.cache.invalidate(key).await;
    }

    /// Invalidate all papers
    pub async fn invalidate_all_papers(&self) {
        info!("Invalidating all papers");
        self.cache.invalidate_pattern("paper:").await;
    }

    /// Invalidate all repositories
    pub async fn invalidate_all_repositories(&self) {
        info!("Invalidating all repositories");
        self.cache.invalidate_pattern("repo:").await;
    }

    /// Invalidate all tasks
    pub async fn invalidate_all_tasks(&self) {
        info!("Invalidating all tasks");
        self.cache.invalidate_pattern("task:").await;
    }

    /// Invalidate all search results
    pub async fn invalidate_all_searches(&self) {
        info!("Invalidating all search results");
        self.cache.invalidate_pattern("search:").await;
    }

    /// Invalidate paper-related caches (paper, lists, searches)
    pub async fn invalidate_paper_related(&self, paper_id: &str) {
        debug!(paper_id = paper_id, "Invalidating paper-related caches");

        // Invalidate the paper itself
        self.cache.invalidate(&CacheKey::paper(paper_id)).await;

        // Invalidate paper lists (they might include this paper)
        self.cache.invalidate_pattern("papers:list:").await;

        // Invalidate search results (they might include this paper)
        self.cache.invalidate_pattern("search:").await;
    }

    /// Invalidate repository-related caches
    pub async fn invalidate_repository_related(&self, repo_id: &str) {
        debug!(repo_id = repo_id, "Invalidating repository-related caches");

        // Invalidate the repository itself
        self.cache.invalidate(&CacheKey::repository(repo_id)).await;

        // Invalidate repository lists
        self.cache.invalidate_pattern("repositories:list:").await;
    }

    /// Invalidate task-related caches
    pub async fn invalidate_task_related(&self, task_id: &str) {
        debug!(task_id = task_id, "Invalidating task-related caches");

        // Invalidate the task itself
        self.cache.invalidate(&CacheKey::task(task_id)).await;

        // Invalidate task lists
        self.cache.invalidate_pattern("tasks:list:").await;
    }

    /// Clear all caches
    pub async fn clear_all(&self) {
        info!("Clearing all caches");
        self.cache.clear().await;
    }
}
