//! In-memory user and session stores (Phase 1 auth).
//!
//! Provides real, stateful authentication: account creation with duplicate
//! detection, password verification, and refresh-token session lifecycle
//! (create / list / revoke / expire). State lives in memory for now; the
//! `User` and `Session` records are serialisable so the same model maps onto a
//! SurrealDB table for persistence — the production step.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use crate::api::auth::{Session, User, UserRole};
use crate::error::{Paper2CodesError, Result};

/// Stateful user store keyed by user id.
#[derive(Default)]
pub struct UserStore {
    by_id: RwLock<HashMap<String, User>>,
}

impl UserStore {
    pub fn new() -> Self {
        Self {
            by_id: RwLock::new(HashMap::new()),
        }
    }

    /// Register a new user, rejecting duplicate username/email (case-insensitive).
    pub async fn register(
        &self,
        email: String,
        username: String,
        password: &str,
        roles: Vec<UserRole>,
    ) -> Result<User> {
        let mut users = self.by_id.write().await;
        if users.values().any(|u| {
            u.username.eq_ignore_ascii_case(&username) || u.email.eq_ignore_ascii_case(&email)
        }) {
            return Err(Paper2CodesError::Validation(
                "a user with that username or email already exists".to_string(),
            ));
        }
        let user = User::new(email, username, password, roles)?;
        users.insert(user.id.clone(), user.clone());
        Ok(user)
    }

    /// Insert or replace a user (used to seed an initial admin).
    pub async fn upsert(&self, user: User) {
        self.by_id.write().await.insert(user.id.clone(), user);
    }

    pub async fn find_by_username(&self, username: &str) -> Option<User> {
        self.by_id
            .read()
            .await
            .values()
            .find(|u| u.username.eq_ignore_ascii_case(username))
            .cloned()
    }

    pub async fn find_by_id(&self, id: &str) -> Option<User> {
        self.by_id.read().await.get(id).cloned()
    }

    /// Find by username or email (case-insensitive) — the login identifier.
    pub async fn find_by_login(&self, identifier: &str) -> Option<User> {
        self.by_id
            .read()
            .await
            .values()
            .find(|u| {
                u.username.eq_ignore_ascii_case(identifier)
                    || u.email.eq_ignore_ascii_case(identifier)
            })
            .cloned()
    }

    pub async fn count(&self) -> usize {
        self.by_id.read().await.len()
    }
}

/// Stateful refresh-token session store keyed by session id.
#[derive(Default)]
pub struct SessionStore {
    by_id: RwLock<HashMap<String, Session>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            by_id: RwLock::new(HashMap::new()),
        }
    }

    pub async fn create(
        &self,
        user_id: String,
        refresh_token: String,
        expires_at: DateTime<Utc>,
    ) -> Session {
        let session = Session::new(user_id, refresh_token, expires_at);
        self.by_id
            .write()
            .await
            .insert(session.id.clone(), session.clone());
        session
    }

    /// Find a non-expired session by its refresh token.
    pub async fn get_by_refresh(&self, refresh_token: &str) -> Option<Session> {
        self.by_id
            .read()
            .await
            .values()
            .find(|s| s.refresh_token == refresh_token && !s.is_expired())
            .cloned()
    }

    pub async fn list_for_user(&self, user_id: &str) -> Vec<Session> {
        self.by_id
            .read()
            .await
            .values()
            .filter(|s| s.user_id == user_id && !s.is_expired())
            .cloned()
            .collect()
    }

    pub async fn all_active(&self) -> Vec<Session> {
        self.by_id
            .read()
            .await
            .values()
            .filter(|s| !s.is_expired())
            .cloned()
            .collect()
    }

    /// Revoke a session by id. Returns true if it existed.
    pub async fn revoke(&self, session_id: &str) -> bool {
        self.by_id.write().await.remove(session_id).is_some()
    }

    /// Revoke the session holding a given refresh token (logout).
    pub async fn revoke_by_refresh(&self, refresh_token: &str) -> bool {
        let mut sessions = self.by_id.write().await;
        let id = sessions
            .values()
            .find(|s| s.refresh_token == refresh_token)
            .map(|s| s.id.clone());
        match id {
            Some(id) => sessions.remove(&id).is_some(),
            None => false,
        }
    }

    /// Drop expired sessions; returns the number removed.
    pub async fn cleanup_expired(&self) -> usize {
        let mut sessions = self.by_id.write().await;
        let before = sessions.len();
        sessions.retain(|_, s| !s.is_expired());
        before - sessions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn register_rejects_duplicates() {
        let store = UserStore::new();
        let u = store
            .register("a@dirac.id".into(), "alice".into(), "password123", vec![UserRole::User])
            .await
            .unwrap();
        assert_eq!(store.count().await, 1);
        assert!(store.find_by_username("ALICE").await.is_some()); // case-insensitive
        assert!(store.find_by_id(&u.id).await.is_some());

        // Duplicate username (different case) rejected.
        assert!(store
            .register("b@dirac.id".into(), "Alice".into(), "password123", vec![UserRole::User])
            .await
            .is_err());
        // Duplicate email rejected.
        assert!(store
            .register("A@dirac.id".into(), "bob".into(), "password123", vec![UserRole::User])
            .await
            .is_err());
    }

    #[tokio::test]
    async fn verifies_password() {
        let store = UserStore::new();
        store
            .register("c@dirac.id".into(), "carol".into(), "s3cret-password", vec![UserRole::User])
            .await
            .unwrap();
        let user = store.find_by_username("carol").await.unwrap();
        assert!(user.verify_password("s3cret-password").unwrap());
        assert!(!user.verify_password("wrong").unwrap());
    }

    #[tokio::test]
    async fn session_lifecycle() {
        let store = SessionStore::new();
        let future = Utc::now() + chrono::Duration::hours(1);
        let s = store.create("u1".into(), "refresh-abc".into(), future).await;

        assert!(store.get_by_refresh("refresh-abc").await.is_some());
        assert_eq!(store.list_for_user("u1").await.len(), 1);

        // Logout by refresh token.
        assert!(store.revoke_by_refresh("refresh-abc").await);
        assert!(store.get_by_refresh("refresh-abc").await.is_none());
        assert!(!store.revoke(&s.id).await); // already gone
    }

    #[tokio::test]
    async fn expired_sessions_are_hidden_and_cleaned() {
        let store = SessionStore::new();
        let past = Utc::now() - chrono::Duration::hours(1);
        store.create("u1".into(), "old".into(), past).await;
        assert!(store.get_by_refresh("old").await.is_none()); // expired -> not returned
        assert_eq!(store.all_active().await.len(), 0);
        assert_eq!(store.cleanup_expired().await, 1);
    }
}
