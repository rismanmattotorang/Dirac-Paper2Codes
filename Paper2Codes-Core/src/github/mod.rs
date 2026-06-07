//! GitHub repository integration
//!
//! Automatic detection and integration of GitHub repositories mentioned in papers
//! to enhance code generation context.

pub mod client;
pub mod extractor;

pub use client::{GitHubClient, RepositoryInfo};
pub use extractor::extract_github_urls;
