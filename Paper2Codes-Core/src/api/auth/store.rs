//! In-memory user and session stores (Phase 1 auth).
//!
//! Provides real, stateful authentication: account creation with duplicate
//! detection, password verification, and refresh-token session lifecycle
//! (create / list / revoke / expire). State lives in memory for now; the
//! `User` and `Session` records are serialisable so the same model maps onto a
//! SurrealDB table for persistence — the production step.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

use crate::api::auth::{Session, User, UserRole};
use crate::error::{Paper2CodesError, Result};

/// Durable backing for the auth stores. Implemented by the storage layer
/// (SurrealDB) so users/sessions survive a restart; left unset for the
/// in-memory default used by tests and single-process runs.
#[async_trait]
pub trait AuthPersistence: Send + Sync {
    async fn load_users(&self) -> Result<Vec<User>>;
    async fn save_user(&self, user: &User) -> Result<()>;
    async fn load_sessions(&self) -> Result<Vec<Session>>;
    async fn save_session(&self, session: &Session) -> Result<()>;
    async fn delete_session(&self, refresh_token: &str) -> Result<()>;
}

/// Stateful user store keyed by user id, with an optional durable backing store.
#[derive(Default)]
pub struct UserStore {
    by_id: RwLock<HashMap<String, User>>,
    persistence: Option<Arc<dyn AuthPersistence>>,
}

impl UserStore {
    pub fn new() -> Self {
        Self {
            by_id: RwLock::new(HashMap::new()),
            persistence: None,
        }
    }

    /// Attach a durable backing store. Call [`UserStore::restore`] afterwards to
    /// load existing accounts into the in-memory cache.
    pub fn with_persistence(mut self, store: Arc<dyn AuthPersistence>) -> Self {
        self.persistence = Some(store);
        self
    }

    /// Load persisted users into the in-memory cache. Returns the count loaded.
    pub async fn restore(&self) -> Result<usize> {
        let Some(store) = &self.persistence else {
            return Ok(0);
        };
        let users = store.load_users().await?;
        let mut cache = self.by_id.write().await;
        for user in users {
            cache.insert(user.id.clone(), user);
        }
        Ok(cache.len())
    }

    async fn persist(&self, user: &User) {
        if let Some(store) = &self.persistence {
            if let Err(e) = store.save_user(user).await {
                tracing::warn!(user = %user.id, error = %e, "failed to persist user");
            }
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
        let user = {
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
            user
        };
        self.persist(&user).await;
        Ok(user)
    }

    /// Insert or replace a user (used to seed an initial admin).
    pub async fn upsert(&self, user: User) {
        self.by_id.write().await.insert(user.id.clone(), user.clone());
        self.persist(&user).await;
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

/// Stateful refresh-token session store keyed by session id, with an optional
/// durable backing store.
#[derive(Default)]
pub struct SessionStore {
    by_id: RwLock<HashMap<String, Session>>,
    persistence: Option<Arc<dyn AuthPersistence>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            by_id: RwLock::new(HashMap::new()),
            persistence: None,
        }
    }

    /// Attach a durable backing store. Call [`SessionStore::restore`] afterwards
    /// to load active sessions into the in-memory cache.
    pub fn with_persistence(mut self, store: Arc<dyn AuthPersistence>) -> Self {
        self.persistence = Some(store);
        self
    }

    /// Load persisted, non-expired sessions into the cache. Returns the count.
    pub async fn restore(&self) -> Result<usize> {
        let Some(store) = &self.persistence else {
            return Ok(0);
        };
        let sessions = store.load_sessions().await?;
        let mut cache = self.by_id.write().await;
        for session in sessions {
            if !session.is_expired() {
                cache.insert(session.id.clone(), session);
            }
        }
        Ok(cache.len())
    }

    async fn persist(&self, session: &Session) {
        if let Some(store) = &self.persistence {
            if let Err(e) = store.save_session(session).await {
                tracing::warn!(session = %session.id, error = %e, "failed to persist session");
            }
        }
    }

    async fn persist_delete(&self, refresh_token: &str) {
        if let Some(store) = &self.persistence {
            if let Err(e) = store.delete_session(refresh_token).await {
                tracing::warn!(error = %e, "failed to delete persisted session");
            }
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
        self.persist(&session).await;
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
        let removed = self.by_id.write().await.remove(session_id);
        match removed {
            Some(session) => {
                self.persist_delete(&session.refresh_token).await;
                true
            }
            None => false,
        }
    }

    /// Revoke the session holding a given refresh token (logout).
    pub async fn revoke_by_refresh(&self, refresh_token: &str) -> bool {
        let removed = {
            let mut sessions = self.by_id.write().await;
            let id = sessions
                .values()
                .find(|s| s.refresh_token == refresh_token)
                .map(|s| s.id.clone());
            id.and_then(|id| sessions.remove(&id))
        };
        match removed {
            Some(_) => {
                self.persist_delete(refresh_token).await;
                true
            }
            None => false,
        }
    }

    /// Drop expired sessions; returns the number removed.
    pub async fn cleanup_expired(&self) -> usize {
        let expired: Vec<Session> = {
            let mut sessions = self.by_id.write().await;
            let expired: Vec<Session> = sessions
                .values()
                .filter(|s| s.is_expired())
                .cloned()
                .collect();
            for s in &expired {
                sessions.remove(&s.id);
            }
            expired
        };
        for s in &expired {
            self.persist_delete(&s.refresh_token).await;
        }
        expired.len()
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

    /// In-memory mock of the durable backing store, to exercise write-through
    /// and restore without a real database.
    #[derive(Default)]
    struct MockAuthStore {
        users: RwLock<HashMap<String, User>>,
        sessions: RwLock<HashMap<String, Session>>,
    }
    #[async_trait]
    impl AuthPersistence for MockAuthStore {
        async fn load_users(&self) -> Result<Vec<User>> {
            Ok(self.users.read().await.values().cloned().collect())
        }
        async fn save_user(&self, user: &User) -> Result<()> {
            self.users.write().await.insert(user.id.clone(), user.clone());
            Ok(())
        }
        async fn load_sessions(&self) -> Result<Vec<Session>> {
            Ok(self.sessions.read().await.values().cloned().collect())
        }
        async fn save_session(&self, session: &Session) -> Result<()> {
            self.sessions
                .write()
                .await
                .insert(session.refresh_token.clone(), session.clone());
            Ok(())
        }
        async fn delete_session(&self, refresh_token: &str) -> Result<()> {
            self.sessions.write().await.remove(refresh_token);
            Ok(())
        }
    }

    #[tokio::test]
    async fn users_write_through_and_restore() {
        let backing = Arc::new(MockAuthStore::default());

        // First process: register a user (write-through to the backing store).
        {
            let store = UserStore::new().with_persistence(backing.clone());
            store
                .register("d@dirac.id".into(), "dave".into(), "password123", vec![UserRole::User])
                .await
                .unwrap();
        }
        assert_eq!(backing.users.read().await.len(), 1);

        // Second process: a fresh store rehydrates from the backing store.
        let store2 = UserStore::new().with_persistence(backing.clone());
        assert_eq!(store2.restore().await.unwrap(), 1);
        assert!(store2.find_by_username("dave").await.is_some());
    }

    #[tokio::test]
    async fn sessions_write_through_and_revoke_deletes() {
        let backing = Arc::new(MockAuthStore::default());
        let future = Utc::now() + chrono::Duration::hours(1);

        let store = SessionStore::new().with_persistence(backing.clone());
        store.create("u1".into(), "refresh-xyz".into(), future).await;
        assert_eq!(backing.sessions.read().await.len(), 1);

        // Logout removes it from the durable store too.
        assert!(store.revoke_by_refresh("refresh-xyz").await);
        assert!(backing.sessions.read().await.is_empty());
    }

    #[tokio::test]
    async fn sessions_restore_skips_expired() {
        let backing = Arc::new(MockAuthStore::default());
        // Seed one active + one expired directly into the backing store.
        backing
            .save_session(&Session::new(
                "u1".into(),
                "active".into(),
                Utc::now() + chrono::Duration::hours(1),
            ))
            .await
            .unwrap();
        backing
            .save_session(&Session::new(
                "u1".into(),
                "expired".into(),
                Utc::now() - chrono::Duration::hours(1),
            ))
            .await
            .unwrap();

        let store = SessionStore::new().with_persistence(backing.clone());
        assert_eq!(store.restore().await.unwrap(), 1); // only the active one
        assert!(store.get_by_refresh("active").await.is_some());
    }
}
