//! User management and role definitions

use crate::api::auth::password::{hash_password, verify_password};
use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User roles for RBAC
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    /// Administrator with full access
    Admin,
    /// Standard user with normal access
    User,
    /// Read-only viewer
    Viewer,
}

impl UserRole {
    /// Convert role to string
    pub fn as_str(&self) -> &'static str {
        match self {
            UserRole::Admin => "admin",
            UserRole::User => "user",
            UserRole::Viewer => "viewer",
        }
    }

    /// Parse role from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "admin" => Some(UserRole::Admin),
            "user" => Some(UserRole::User),
            "viewer" => Some(UserRole::Viewer),
            _ => None,
        }
    }

    /// Check if role has permission for another role
    pub fn can_access(&self, required_role: &UserRole) -> bool {
        match (self, required_role) {
            (UserRole::Admin, _) => true,
            (UserRole::User, UserRole::User) => true,
            (UserRole::User, UserRole::Viewer) => true,
            (UserRole::Viewer, UserRole::Viewer) => true,
            _ => false,
        }
    }
}

impl From<UserRole> for String {
    fn from(role: UserRole) -> Self {
        role.as_str().to_string()
    }
}

/// User entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique user ID
    pub id: String,
    /// User email (unique)
    pub email: String,
    /// Username (unique)
    pub username: String,
    /// Hashed password
    pub password_hash: String,
    /// User roles
    pub roles: Vec<UserRole>,
    /// Account creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Create a new user
    pub fn new(
        email: String,
        username: String,
        password: &str,
        roles: Vec<UserRole>,
    ) -> Result<Self> {
        let password_hash = hash_password(password)?;
        let now = Utc::now();

        Ok(Self {
            id: Uuid::new_v4().to_string(),
            email,
            username,
            password_hash,
            roles,
            created_at: now,
            updated_at: now,
        })
    }

    /// Verify a password against the user's hash
    pub fn verify_password(&self, password: &str) -> Result<bool> {
        verify_password(password, &self.password_hash)
    }

    /// Update user password
    pub fn update_password(&mut self, new_password: &str) -> Result<()> {
        self.password_hash = hash_password(new_password)?;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Check if user has a specific role
    pub fn has_role(&self, role: UserRole) -> bool {
        self.roles.contains(&role)
    }

    /// Check if user has any of the specified roles
    pub fn has_any_role(&self, roles: &[UserRole]) -> bool {
        roles.iter().any(|role| self.has_role(*role))
    }

    /// Get roles as string vector
    pub fn roles_as_strings(&self) -> Vec<String> {
        self.roles.iter().map(|r| r.as_str().to_string()).collect()
    }
}

/// User service for user management operations
pub struct UserService;

impl UserService {
    /// Validate email format
    pub fn validate_email(email: &str) -> bool {
        // Basic email validation
        email.contains('@') && email.contains('.') && email.len() > 5
    }

    /// Validate username format
    pub fn validate_username(username: &str) -> bool {
        // Username: 3-30 characters, alphanumeric and underscore
        username.len() >= 3
            && username.len() <= 30
            && username.chars().all(|c| c.is_alphanumeric() || c == '_')
    }

    /// Validate password strength
    pub fn validate_password(password: &str, min_length: usize) -> bool {
        password.len() >= min_length
            && password.chars().any(|c| c.is_ascii_lowercase())
            && password.chars().any(|c| c.is_ascii_uppercase())
            && password.chars().any(|c| c.is_ascii_digit())
    }
}

/// Session entity for refresh token management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Session ID
    pub id: String,
    /// User ID
    pub user_id: String,
    /// Refresh token
    pub refresh_token: String,
    /// Expiration timestamp
    pub expires_at: DateTime<Utc>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last used timestamp
    pub last_used_at: DateTime<Utc>,
}

impl Session {
    /// Create a new session
    pub fn new(user_id: String, refresh_token: String, expires_at: DateTime<Utc>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            refresh_token,
            expires_at,
            created_at: now,
            last_used_at: now,
        }
    }

    /// Check if session is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Update last used timestamp
    pub fn update_last_used(&mut self) {
        self.last_used_at = Utc::now();
    }
}
