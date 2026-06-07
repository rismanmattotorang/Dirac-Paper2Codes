//! GitHub API client for repository information extraction
use crate::error::{Paper2CodesError, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use reqwest::header;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

/// GitHub repository information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RepositoryInfo {
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub html_url: String,
    pub language: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub readme: Option<String>,
    pub requirements: Option<Vec<String>>,
}

/// Repository content structure
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct RepositoryContent {
    name: String,
    content: Option<String>,
    download_url: Option<String>,
    r#type: String,
}

/// GitHub API client
/// Thread-safe implementation using Arc<Mutex<>> for rate limit tracking
pub struct GitHubClient {
    client: reqwest::Client,
    rate_limit_remaining: std::sync::Arc<std::sync::Mutex<u32>>,
}

impl GitHubClient {
    /// Create a new GitHub client
    pub fn new(token: Option<String>) -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/vnd.github.v3+json"),
        );
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static("paper2codes"),
        );

        let has_token = token.is_some();
        if let Some(ref token_val) = token {
            headers.insert(
                header::AUTHORIZATION,
                header::HeaderValue::from_str(&format!("token {}", token_val)).map_err(|e| {
                    Paper2CodesError::Validation(format!("Invalid GitHub token: {}", e))
                })?,
            );
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .default_headers(headers)
            .build()
            .map_err(|e| {
                Paper2CodesError::Validation(format!("Failed to build HTTP client: {}", e))
            })?;

        Ok(Self {
            client,
            rate_limit_remaining: std::sync::Arc::new(std::sync::Mutex::new(if has_token {
                5000
            } else {
                60
            })),
        })
    }

    /// Extract repository owner and name from a GitHub URL
    pub fn extract_repo_info_from_url(&self, url: &str) -> Option<(String, String)> {
        let github_regex = regex::Regex::new(r"github\.com/([^/]+)/([^/]+)").ok()?;

        if let Some(captures) = github_regex.captures(url) {
            let owner = captures.get(1)?.as_str().to_string();
            let mut repo = captures.get(2)?.as_str().to_string();

            // Remove .git extension if present
            if repo.ends_with(".git") {
                repo = repo[..repo.len() - 4].to_string();
            }

            // Remove query parameters or fragment identifiers
            if let Some(index) = repo.find(|c| c == '?' || c == '#') {
                repo = repo[..index].to_string();
            }

            return Some((owner, repo));
        }

        None
    }

    /// Get repository information
    /// This method is now thread-safe and can be called concurrently
    pub async fn get_repository_info(&self, owner: &str, repo: &str) -> Result<RepositoryInfo> {
        self.check_rate_limit().await?;

        let url = format!("https://api.github.com/repos/{}/{}", owner, repo);
        let response = self.client.get(&url).send().await.map_err(|e| {
            Paper2CodesError::Validation(format!("GitHub API request failed: {}", e))
        })?;

        self.update_rate_limit(&response);

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(Paper2CodesError::Validation(format!(
                "GitHub API error ({}): {}",
                status, error_text
            )));
        }

        #[derive(Deserialize)]
        struct GitHubRepo {
            name: String,
            full_name: String,
            description: Option<String>,
            html_url: String,
            language: Option<String>,
            created_at: String,
            updated_at: String,
        }

        let repo: GitHubRepo = response.json().await.map_err(|e| {
            Paper2CodesError::Validation(format!("Failed to parse GitHub response: {}", e))
        })?;

        // Get README
        let readme = self.get_readme(owner, repo.name.as_str()).await.ok();

        // Get requirements
        let requirements = self.get_requirements(owner, repo.name.as_str()).await.ok();

        Ok(RepositoryInfo {
            name: repo.name,
            full_name: repo.full_name,
            description: repo.description,
            html_url: repo.html_url,
            language: repo.language,
            created_at: repo.created_at,
            updated_at: repo.updated_at,
            readme,
            requirements,
        })
    }

    /// Get repository README
    async fn get_readme(&self, owner: &str, repo: &str) -> Result<String> {
        self.check_rate_limit().await?;

        let url = format!("https://api.github.com/repos/{}/{}/readme", owner, repo);
        let response = self.client.get(&url).send().await.map_err(|e| {
            Paper2CodesError::Validation(format!("GitHub API request failed: {}", e))
        })?;

        self.update_rate_limit(&response);

        if !response.status().is_success() {
            return Err(Paper2CodesError::Validation(format!(
                "Failed to get README: {}",
                response.status()
            )));
        }

        let content: RepositoryContent = response.json().await.map_err(|e| {
            Paper2CodesError::Validation(format!("Failed to parse response: {}", e))
        })?;

        if let Some(content_base64) = content.content {
            let cleaned = content_base64.replace("\n", "");
            let decoded = BASE64.decode(&cleaned).map_err(|e| {
                Paper2CodesError::Validation(format!("Failed to decode base64: {}", e))
            })?;
            String::from_utf8(decoded).map_err(|e| {
                Paper2CodesError::Validation(format!("Failed to convert to UTF-8: {}", e))
            })
        } else {
            Err(Paper2CodesError::Validation(
                "README content is empty".to_string(),
            ))
        }
    }

    /// Get repository requirements files
    async fn get_requirements(&self, owner: &str, repo: &str) -> Result<Vec<String>> {
        let mut requirements = Vec::new();
        let req_files = vec![
            "requirements.txt",
            "Cargo.toml",
            "package.json",
            "go.mod",
            "pom.xml",
        ];

        for file in req_files {
            if let Ok(content) = self.get_file_content(owner, repo, file).await {
                requirements.push(format!("{}:\n{}", file, content));
            }
        }

        Ok(requirements)
    }

    /// Get content of a specific file
    async fn get_file_content(&self, owner: &str, repo: &str, path: &str) -> Result<String> {
        self.check_rate_limit().await?;

        let url = format!(
            "https://api.github.com/repos/{}/{}/contents/{}",
            owner, repo, path
        );
        let response = self.client.get(&url).send().await.map_err(|e| {
            Paper2CodesError::Validation(format!("GitHub API request failed: {}", e))
        })?;

        self.update_rate_limit(&response);

        if !response.status().is_success() {
            return Err(Paper2CodesError::Validation(format!(
                "Failed to get file: {}",
                response.status()
            )));
        }

        let content: RepositoryContent = response.json().await.map_err(|e| {
            Paper2CodesError::Validation(format!("Failed to parse response: {}", e))
        })?;

        if let Some(content_base64) = content.content {
            let cleaned = content_base64.replace("\n", "");
            let decoded = BASE64.decode(&cleaned).map_err(|e| {
                Paper2CodesError::Validation(format!("Failed to decode base64: {}", e))
            })?;
            String::from_utf8(decoded).map_err(|e| {
                Paper2CodesError::Validation(format!("Failed to convert to UTF-8: {}", e))
            })
        } else {
            Err(Paper2CodesError::Validation(
                "File content is empty".to_string(),
            ))
        }
    }

    fn update_rate_limit(&self, response: &reqwest::Response) {
        if let Some(remaining) = response.headers().get("X-RateLimit-Remaining") {
            if let Ok(remaining_str) = remaining.to_str() {
                if let Ok(remaining_val) = remaining_str.parse::<u32>() {
                    if let Ok(mut limit) = self.rate_limit_remaining.lock() {
                        *limit = remaining_val;
                        debug!("GitHub API rate limit remaining: {}", remaining_val);
                    }
                }
            }
        }
    }

    async fn check_rate_limit(&self) -> Result<()> {
        let current_limit = {
            self.rate_limit_remaining
                .lock()
                .map_err(|e| {
                    Paper2CodesError::Validation(format!(
                        "Failed to acquire rate limit lock: {}",
                        e
                    ))
                })?
                .clone()
        };

        if current_limit < 5 {
            warn!(
                "GitHub API rate limit is low: {}. Waiting...",
                current_limit
            );
            sleep(Duration::from_secs(10)).await;
        }
        Ok(())
    }
}
