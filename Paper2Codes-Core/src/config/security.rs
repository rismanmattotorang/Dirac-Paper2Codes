//! Security validation and secret redaction (Phase 1 hardening).
//!
//! Production deployments must not run with the built-in default secrets. These
//! helpers detect weak/default configuration and either fail closed (strict /
//! production mode) or warn loudly (development).

use crate::config::Config;
use crate::error::{ConfigError, Paper2CodesError, Result};

/// The insecure default JWT secret shipped for local development.
pub const DEFAULT_JWT_SECRET: &str = "change-me-in-production";
/// Minimum acceptable JWT secret length (bytes) in production.
pub const MIN_JWT_SECRET_LEN: usize = 32;

/// Redact a secret for safe display/logging: keep a short prefix/suffix.
pub fn redact(secret: &str) -> String {
    let n = secret.chars().count();
    if n == 0 {
        return String::new();
    }
    if n <= 8 {
        return "•".repeat(n);
    }
    let prefix: String = secret.chars().take(3).collect();
    let suffix: String = {
        let tail: Vec<char> = secret.chars().rev().take(2).collect();
        tail.into_iter().rev().collect()
    };
    format!("{}…{}", prefix, suffix)
}

impl Config {
    /// Return a list of security issues with the current configuration. Empty =
    /// safe to run in production. Pure (no I/O) so it is easily unit-tested.
    pub fn security_issues(&self) -> Vec<String> {
        let mut issues = Vec::new();

        // JWT secret must be non-default and sufficiently long.
        let jwt = &self.api.auth.jwt_secret;
        if jwt == DEFAULT_JWT_SECRET {
            issues.push(
                "api.auth.jwt_secret is the built-in default; set a strong secret (env JWT_SECRET or JWT_SECRET_FILE)".to_string(),
            );
        } else if jwt.len() < MIN_JWT_SECRET_LEN {
            issues.push(format!(
                "api.auth.jwt_secret is too short ({} < {} chars)",
                jwt.len(),
                MIN_JWT_SECRET_LEN
            ));
        }

        // SurrealDB must not use the default root credentials.
        if self.storage.enabled {
            if self.storage.username.as_deref() == Some("root") {
                issues.push("storage.username is the default 'root'".to_string());
            }
            if self.storage.password.as_deref() == Some("root") {
                issues.push("storage.password is the default 'root'".to_string());
            }
        }

        issues
    }

    /// Enforce security configuration. In `strict` mode (production) any issue is
    /// a hard error; otherwise issues are logged as warnings.
    pub fn enforce_security(&self, strict: bool) -> Result<()> {
        let issues = self.security_issues();
        if issues.is_empty() {
            return Ok(());
        }
        if strict {
            return Err(Paper2CodesError::Config(ConfigError::Invalid(format!(
                "insecure configuration for production: {}",
                issues.join("; ")
            ))));
        }
        for issue in &issues {
            tracing::warn!("security: {}", issue);
        }
        Ok(())
    }
}

/// Whether the process is running in production mode (`PAPER2CODES_ENV=production`).
pub fn is_production() -> bool {
    std::env::var("PAPER2CODES_ENV")
        .map(|v| v.eq_ignore_ascii_case("production") || v.eq_ignore_ascii_case("prod"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_secrets() {
        assert_eq!(redact("sk-proj-abcdef1234"), "sk-…34");
        assert_eq!(redact("short"), "•••••");
        assert_eq!(redact(""), "");
    }

    #[test]
    fn default_config_flags_default_jwt_and_root_db() {
        let cfg = Config::default();
        let issues = cfg.security_issues();
        assert!(issues.iter().any(|i| i.contains("jwt_secret")));
        assert!(issues.iter().any(|i| i.contains("'root'")));
        // Non-strict only warns.
        assert!(cfg.enforce_security(false).is_ok());
        // Strict fails closed.
        assert!(cfg.enforce_security(true).is_err());
    }

    #[test]
    fn hardened_config_passes() {
        let mut cfg = Config::default();
        cfg.api.auth.jwt_secret = "a-very-long-and-random-production-secret-value".to_string();
        cfg.storage.username = Some("p2c_app".to_string());
        cfg.storage.password = Some("a-strong-db-password".to_string());
        assert!(cfg.security_issues().is_empty());
        assert!(cfg.enforce_security(true).is_ok());
    }

    #[test]
    fn short_custom_secret_is_flagged() {
        let mut cfg = Config::default();
        cfg.api.auth.jwt_secret = "tooshort".to_string();
        cfg.storage.enabled = false; // isolate the jwt check
        let issues = cfg.security_issues();
        assert_eq!(issues.len(), 1);
        assert!(issues[0].contains("too short"));
    }
}
