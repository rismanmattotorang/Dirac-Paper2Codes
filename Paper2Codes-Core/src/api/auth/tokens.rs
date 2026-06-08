//! Personal API tokens (Phase 1 auth, programmatic access).
//!
//! Lets a user mint long-lived bearer tokens for scripts/CI without exposing
//! their password or short-lived JWTs. Only a hash of each token is stored; the
//! plaintext is shown exactly once at creation. State lives in memory with an
//! optional durable backing store (same write-through + restore pattern as the
//! user/session stores), so tokens survive a restart when SurrealDB is wired.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine;
use chrono::{DateTime, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::error::Result;

/// Prefix marking a Dirac Paper2Codes personal access token.
pub const TOKEN_PREFIX: &str = "p2c_";

/// Stored representation of an API token. The plaintext is never persisted —
/// only `token_hash` (a SHA-256 digest) and a short display `prefix`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiToken {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub token_hash: String,
    /// First few characters of the plaintext, for display (e.g. `p2c_AbCd`).
    pub prefix: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl ApiToken {
    pub fn is_expired(&self) -> bool {
        self.expires_at.map(|t| t < Utc::now()).unwrap_or(false)
    }
}

/// SHA-256 hash of a token's plaintext, base64-encoded. Tokens are high-entropy
/// so a fast hash is appropriate (unlike passwords, which use argon2).
fn hash_token(plaintext: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(plaintext.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(hasher.finalize())
}

/// Generate a fresh, high-entropy token plaintext (`p2c_<32 random bytes>`).
fn generate_plaintext() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let body = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
    format!("{TOKEN_PREFIX}{body}")
}

/// Durable backing for the API token store.
#[async_trait]
pub trait ApiTokenPersistence: Send + Sync {
    async fn load_tokens(&self) -> Result<Vec<ApiToken>>;
    async fn save_token(&self, token: &ApiToken) -> Result<()>;
    async fn delete_token(&self, id: &str) -> Result<()>;
}

/// Stateful API token store keyed by token id.
#[derive(Default)]
pub struct ApiTokenStore {
    by_id: RwLock<HashMap<String, ApiToken>>,
    persistence: Option<Arc<dyn ApiTokenPersistence>>,
}

impl ApiTokenStore {
    pub fn new() -> Self {
        Self {
            by_id: RwLock::new(HashMap::new()),
            persistence: None,
        }
    }

    pub fn with_persistence(mut self, store: Arc<dyn ApiTokenPersistence>) -> Self {
        self.persistence = Some(store);
        self
    }

    /// Load persisted, non-expired tokens into the cache. Returns the count.
    pub async fn restore(&self) -> Result<usize> {
        let Some(store) = &self.persistence else {
            return Ok(0);
        };
        let tokens = store.load_tokens().await?;
        let mut cache = self.by_id.write().await;
        for token in tokens {
            if !token.is_expired() {
                cache.insert(token.id.clone(), token);
            }
        }
        Ok(cache.len())
    }

    async fn persist(&self, token: &ApiToken) {
        if let Some(store) = &self.persistence {
            if let Err(e) = store.save_token(token).await {
                tracing::warn!(token = %token.id, error = %e, "failed to persist api token");
            }
        }
    }

    async fn persist_delete(&self, id: &str) {
        if let Some(store) = &self.persistence {
            if let Err(e) = store.delete_token(id).await {
                tracing::warn!(token = %id, error = %e, "failed to delete persisted api token");
            }
        }
    }

    /// Mint a new token for `user_id`. Returns the stored record and the
    /// plaintext (shown to the caller exactly once — it is not recoverable).
    pub async fn create(
        &self,
        user_id: String,
        name: String,
        expires_at: Option<DateTime<Utc>>,
    ) -> (ApiToken, String) {
        let plaintext = generate_plaintext();
        let token = ApiToken {
            id: Uuid::new_v4().to_string(),
            user_id,
            name,
            token_hash: hash_token(&plaintext),
            prefix: plaintext.chars().take(12).collect(),
            created_at: Utc::now(),
            last_used_at: None,
            expires_at,
        };
        self.by_id
            .write()
            .await
            .insert(token.id.clone(), token.clone());
        self.persist(&token).await;
        (token, plaintext)
    }

    /// Verify a presented plaintext token. On a match for a non-expired token,
    /// stamps `last_used_at` and returns the owning record.
    pub async fn verify(&self, plaintext: &str) -> Option<ApiToken> {
        if !plaintext.starts_with(TOKEN_PREFIX) {
            return None;
        }
        let hash = hash_token(plaintext);
        let updated = {
            let mut tokens = self.by_id.write().await;
            let id = tokens
                .values()
                .find(|t| t.token_hash == hash && !t.is_expired())
                .map(|t| t.id.clone());
            id.and_then(|id| {
                tokens.get_mut(&id).map(|t| {
                    t.last_used_at = Some(Utc::now());
                    t.clone()
                })
            })
        };
        if let Some(ref token) = updated {
            self.persist(token).await;
        }
        updated
    }

    pub async fn list_for_user(&self, user_id: &str) -> Vec<ApiToken> {
        self.by_id
            .read()
            .await
            .values()
            .filter(|t| t.user_id == user_id && !t.is_expired())
            .cloned()
            .collect()
    }

    /// Revoke a token by id, but only if it belongs to `user_id`. Returns true
    /// if a token was removed.
    pub async fn revoke(&self, id: &str, user_id: &str) -> bool {
        let removed = {
            let mut tokens = self.by_id.write().await;
            match tokens.get(id) {
                Some(t) if t.user_id == user_id => tokens.remove(id),
                _ => None,
            }
        };
        match removed {
            Some(_) => {
                self.persist_delete(id).await;
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_then_verify_roundtrip() {
        let store = ApiTokenStore::new();
        let (token, plaintext) = store.create("u1".into(), "ci".into(), None).await;
        assert!(plaintext.starts_with(TOKEN_PREFIX));
        assert!(token.prefix.starts_with(TOKEN_PREFIX));
        // The plaintext is not stored; only its hash.
        assert_ne!(token.token_hash, plaintext);

        let verified = store.verify(&plaintext).await.expect("token should verify");
        assert_eq!(verified.user_id, "u1");
        assert!(verified.last_used_at.is_some());

        // A wrong token does not verify.
        assert!(store.verify("p2c_not-a-real-token").await.is_none());
        // A non-prefixed token is rejected outright.
        assert!(store.verify("garbage").await.is_none());
    }

    #[tokio::test]
    async fn list_and_revoke_are_scoped_to_owner() {
        let store = ApiTokenStore::new();
        let (t1, _) = store.create("u1".into(), "a".into(), None).await;
        store.create("u1".into(), "b".into(), None).await;
        store.create("u2".into(), "c".into(), None).await;

        assert_eq!(store.list_for_user("u1").await.len(), 2);
        // u2 cannot revoke u1's token.
        assert!(!store.revoke(&t1.id, "u2").await);
        assert_eq!(store.list_for_user("u1").await.len(), 2);
        // The owner can.
        assert!(store.revoke(&t1.id, "u1").await);
        assert_eq!(store.list_for_user("u1").await.len(), 1);
    }

    #[tokio::test]
    async fn expired_tokens_do_not_verify_or_list() {
        let store = ApiTokenStore::new();
        let past = Utc::now() - chrono::Duration::hours(1);
        let (_, plaintext) = store.create("u1".into(), "old".into(), Some(past)).await;
        assert!(store.verify(&plaintext).await.is_none());
        assert_eq!(store.list_for_user("u1").await.len(), 0);
    }

    #[derive(Default)]
    struct MockTokenStore {
        tokens: RwLock<HashMap<String, ApiToken>>,
    }
    #[async_trait]
    impl ApiTokenPersistence for MockTokenStore {
        async fn load_tokens(&self) -> Result<Vec<ApiToken>> {
            Ok(self.tokens.read().await.values().cloned().collect())
        }
        async fn save_token(&self, token: &ApiToken) -> Result<()> {
            self.tokens.write().await.insert(token.id.clone(), token.clone());
            Ok(())
        }
        async fn delete_token(&self, id: &str) -> Result<()> {
            self.tokens.write().await.remove(id);
            Ok(())
        }
    }

    #[tokio::test]
    async fn write_through_and_restore() {
        let backing = Arc::new(MockTokenStore::default());
        let plaintext = {
            let store = ApiTokenStore::new().with_persistence(backing.clone());
            let (_, plaintext) = store.create("u1".into(), "ci".into(), None).await;
            plaintext
        };
        assert_eq!(backing.tokens.read().await.len(), 1);

        // A fresh store rehydrates and the persisted token still verifies.
        let store2 = ApiTokenStore::new().with_persistence(backing.clone());
        assert_eq!(store2.restore().await.unwrap(), 1);
        assert!(store2.verify(&plaintext).await.is_some());
    }
}
