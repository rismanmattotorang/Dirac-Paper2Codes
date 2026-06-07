//! JWT token generation and validation

use crate::config::AuthConfig;
use crate::error::{Paper2CodesError, Result};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

/// JWT claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (user ID)
    pub sub: String,
    /// User email
    pub email: String,
    /// User roles
    pub roles: Vec<String>,
    /// Issued at timestamp
    pub iat: i64,
    /// Expiration timestamp
    pub exp: i64,
    /// Token type (access or refresh)
    #[serde(default = "default_token_type")]
    pub token_type: String,
}

fn default_token_type() -> String {
    "access".to_string()
}

impl Claims {
    /// Create new access token claims
    pub fn new_access(
        user_id: String,
        email: String,
        roles: Vec<String>,
        expiration: Duration,
    ) -> Self {
        let now = Utc::now();
        Self {
            sub: user_id,
            email,
            roles,
            iat: now.timestamp(),
            exp: (now + expiration).timestamp(),
            token_type: "access".to_string(),
        }
    }

    /// Create new refresh token claims
    pub fn new_refresh(
        user_id: String,
        email: String,
        roles: Vec<String>,
        expiration: Duration,
    ) -> Self {
        let now = Utc::now();
        Self {
            sub: user_id,
            email,
            roles,
            iat: now.timestamp(),
            exp: (now + expiration).timestamp(),
            token_type: "refresh".to_string(),
        }
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() > self.exp
    }

    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if user has any of the specified roles
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|role| self.has_role(role))
    }
}

/// JWT service for token generation and validation
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
    access_expiration: Duration,
    refresh_expiration: Duration,
}

impl JwtService {
    /// Create a new JWT service
    pub fn new(config: &AuthConfig) -> Result<Self> {
        let secret = config.jwt_secret.as_bytes();

        let encoding_key = EncodingKey::from_secret(secret);
        let decoding_key = DecodingKey::from_secret(secret);

        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;
        validation.validate_nbf = false;

        let access_expiration = Duration::seconds(config.jwt_expiration as i64);
        let refresh_expiration = Duration::seconds(config.refresh_expiration as i64);

        Ok(Self {
            encoding_key,
            decoding_key,
            validation,
            access_expiration,
            refresh_expiration,
        })
    }

    /// Generate an access token
    pub fn generate_access_token(
        &self,
        user_id: String,
        email: String,
        roles: Vec<String>,
    ) -> Result<String> {
        let claims = Claims::new_access(user_id, email, roles, self.access_expiration);

        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding_key).map_err(|e| {
            Paper2CodesError::Config(crate::error::ConfigError::Invalid(format!(
                "Failed to generate access token: {}",
                e
            )))
        })
    }

    /// Generate a refresh token
    pub fn generate_refresh_token(
        &self,
        user_id: String,
        email: String,
        roles: Vec<String>,
    ) -> Result<String> {
        let claims = Claims::new_refresh(user_id, email, roles, self.refresh_expiration);

        encode(&Header::new(Algorithm::HS256), &claims, &self.encoding_key).map_err(|e| {
            Paper2CodesError::Config(crate::error::ConfigError::Invalid(format!(
                "Failed to generate refresh token: {}",
                e
            )))
        })
    }

    /// Validate and decode a token
    pub fn validate_token(&self, token: &str) -> Result<Claims> {
        let token_data =
            decode::<Claims>(token, &self.decoding_key, &self.validation).map_err(|e| {
                Paper2CodesError::Config(crate::error::ConfigError::Invalid(format!(
                    "Invalid token: {}",
                    e
                )))
            })?;

        let claims = token_data.claims;

        // Check expiration manually as well
        if claims.is_expired() {
            return Err(Paper2CodesError::Config(
                crate::error::ConfigError::Invalid("Token expired".to_string()),
            ));
        }

        Ok(claims)
    }

    /// Validate access token
    pub fn validate_access_token(&self, token: &str) -> Result<Claims> {
        let claims = self.validate_token(token)?;

        if claims.token_type != "access" {
            return Err(Paper2CodesError::Config(
                crate::error::ConfigError::Invalid("Invalid token type".to_string()),
            ));
        }

        Ok(claims)
    }

    /// Validate refresh token
    pub fn validate_refresh_token(&self, token: &str) -> Result<Claims> {
        let claims = self.validate_token(token)?;

        if claims.token_type != "refresh" {
            return Err(Paper2CodesError::Config(
                crate::error::ConfigError::Invalid("Invalid token type".to_string()),
            ));
        }

        Ok(claims)
    }

    /// Get access token expiration duration
    pub fn access_expiration(&self) -> Duration {
        self.access_expiration
    }

    /// Get refresh token expiration duration
    pub fn refresh_expiration(&self) -> Duration {
        self.refresh_expiration
    }
}
