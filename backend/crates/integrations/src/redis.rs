//! Redis integration for caching, sessions, and pub/sub (Epic 103).
//!
//! Stories:
//! - 103.2: Redis client integration with health check
//! - 103.3: Redis session storage with TTL
//! - 103.4: Redis pub/sub for real-time cross-instance messaging

use redis::{
    aio::ConnectionManager, AsyncCommands, Client as RedisClientInner, RedisError, RedisResult,
};
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;
use tokio::sync::broadcast;
use tracing::Instrument;
use uuid::Uuid;

// ============================================================================
// Configuration
// ============================================================================

/// Environment variable for Redis URL.
pub const REDIS_URL_ENV: &str = "REDIS_URL";

/// Default Redis URL if not configured.
pub const DEFAULT_REDIS_URL: &str = "redis://127.0.0.1:6379";

/// Default session TTL in seconds (7 days).
pub const DEFAULT_SESSION_TTL_SECS: u64 = 7 * 24 * 60 * 60;

/// Default cache TTL in seconds (15 minutes).
pub const DEFAULT_CACHE_TTL_SECS: u64 = 15 * 60;

/// Key prefix for sessions.
pub const SESSION_KEY_PREFIX: &str = "session:";

/// Key prefix for cache entries.
pub const CACHE_KEY_PREFIX: &str = "cache:";

/// Key prefix for pub/sub channels.
pub const PUBSUB_CHANNEL_PREFIX: &str = "pubsub:";

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during Redis operations.
#[derive(Debug, Error)]
pub enum CacheError {
    /// Connection error.
    #[error("Redis connection error: {0}")]
    Connection(String),

    /// Configuration error.
    #[error("Redis configuration error: {0}")]
    Configuration(String),

    /// Serialization error.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Deserialization error.
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Key not found.
    #[error("Key not found: {0}")]
    NotFound(String),

    /// Operation error.
    #[error("Redis operation error: {0}")]
    Operation(String),

    /// Pub/sub error.
    #[error("Pub/sub error: {0}")]
    PubSub(String),
}

impl From<RedisError> for CacheError {
    fn from(err: RedisError) -> Self {
        CacheError::Operation(err.to_string())
    }
}

// ============================================================================
// Redis Configuration
// ============================================================================

/// Redis configuration.
#[derive(Debug, Clone)]
pub struct RedisConfig {
    /// Redis URL (e.g., redis://127.0.0.1:6379).
    pub url: String,

    /// Connection timeout in milliseconds.
    pub connection_timeout_ms: u64,

    /// Default TTL for cache entries in seconds.
    pub default_cache_ttl_secs: u64,

    /// Default TTL for sessions in seconds.
    pub default_session_ttl_secs: u64,

    /// Key prefix for namespacing (optional).
    pub key_prefix: Option<String>,
}

impl RedisConfig {
    /// Create configuration from environment variables.
    pub fn from_env() -> Result<Self, CacheError> {
        let url = std::env::var(REDIS_URL_ENV).unwrap_or_else(|_| DEFAULT_REDIS_URL.to_string());

        Ok(Self {
            url,
            connection_timeout_ms: 5000,
            default_cache_ttl_secs: DEFAULT_CACHE_TTL_SECS,
            default_session_ttl_secs: DEFAULT_SESSION_TTL_SECS,
            key_prefix: None,
        })
    }

    /// Create configuration with a custom URL.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            connection_timeout_ms: 5000,
            default_cache_ttl_secs: DEFAULT_CACHE_TTL_SECS,
            default_session_ttl_secs: DEFAULT_SESSION_TTL_SECS,
            key_prefix: None,
        }
    }

    /// Set a key prefix for namespacing.
    #[must_use]
    pub fn with_key_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.key_prefix = Some(prefix.into());
        self
    }

    /// Set session TTL.
    #[must_use]
    pub fn with_session_ttl(mut self, ttl_secs: u64) -> Self {
        self.default_session_ttl_secs = ttl_secs;
        self
    }

    /// Set cache TTL.
    #[must_use]
    pub fn with_cache_ttl(mut self, ttl_secs: u64) -> Self {
        self.default_cache_ttl_secs = ttl_secs;
        self
    }
}

// ============================================================================
// Story 103.2: Redis Client
// ============================================================================

/// Redis client wrapper with connection pooling (Story 103.2).
#[derive(Clone)]
pub struct RedisClient {
    connection_manager: ConnectionManager,
    config: RedisConfig,
}

impl RedisClient {
    /// Create a new Redis client from configuration (Story 103.2).
    pub async fn new(config: RedisConfig) -> Result<Self, CacheError> {
        let client = RedisClientInner::open(config.url.as_str())
            .map_err(|e| CacheError::Connection(format!("Failed to create client: {}", e)))?;

        let connection_manager = ConnectionManager::new(client)
            .await
            .map_err(|e| CacheError::Connection(format!("Failed to create connection: {}", e)))?;

        tracing::info!(url = %config.url, "Connected to Redis");

        Ok(Self {
            connection_manager,
            config,
        })
    }

    /// Create a Redis client from environment variables (Story 103.2).
    pub async fn from_env() -> Result<Self, CacheError> {
        Self::new(RedisConfig::from_env()?).await
    }

    /// Build a full key with prefix.
    fn build_key(&self, key: &str) -> String {
        match &self.config.key_prefix {
            Some(prefix) => format!("{}:{}", prefix, key),
            None => key.to_string(),
        }
    }

    /// Health check - ping Redis (Story 103.2).
    pub async fn health_check(&self) -> Result<bool, CacheError> {
        let mut conn = self.connection_manager.clone();
        let result: RedisResult<String> = redis::cmd("PING").query_async(&mut conn).await;

        match result {
            Ok(response) if response == "PONG" => Ok(true),
            Ok(response) => Err(CacheError::Operation(format!(
                "Unexpected PING response: {}",
                response
            ))),
            Err(e) => Err(CacheError::Connection(format!("PING failed: {}", e))),
        }
    }

    /// Get Redis info for diagnostics (Story 103.2).
    pub async fn info(&self) -> Result<String, CacheError> {
        let mut conn = self.connection_manager.clone();
        let info: String = redis::cmd("INFO").query_async(&mut conn).await?;
        Ok(info)
    }

    // =========================================================================
    // Basic Cache Operations
    // =========================================================================

    /// Set a value with optional TTL.
    pub async fn set<T: Serialize>(
        &self,
        key: &str,
        value: &T,
        ttl_secs: Option<u64>,
    ) -> Result<(), CacheError> {
        let full_key = self.build_key(key);
        let serialized =
            serde_json::to_string(value).map_err(|e| CacheError::Serialization(e.to_string()))?;

        let mut conn = self.connection_manager.clone();

        match ttl_secs {
            Some(ttl) => {
                let _: () = conn.set_ex(&full_key, &serialized, ttl).await?;
            }
            None => {
                let _: () = conn.set(&full_key, &serialized).await?;
            }
        }

        tracing::trace!(key = %full_key, ttl = ?ttl_secs, "Cache SET");
        Ok(())
    }

    /// Get a value.
    pub async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, CacheError> {
        let full_key = self.build_key(key);
        let mut conn = self.connection_manager.clone();

        let result: Option<String> = conn.get(&full_key).await?;

        match result {
            Some(data) => {
                let value = serde_json::from_str(&data)
                    .map_err(|e| CacheError::Deserialization(e.to_string()))?;
                tracing::trace!(key = %full_key, "Cache HIT");
                Ok(Some(value))
            }
            None => {
                tracing::trace!(key = %full_key, "Cache MISS");
                Ok(None)
            }
        }
    }

    /// Delete a key.
    pub async fn delete(&self, key: &str) -> Result<bool, CacheError> {
        let full_key = self.build_key(key);
        let mut conn = self.connection_manager.clone();

        let deleted: i64 = conn.del(&full_key).await?;
        tracing::trace!(key = %full_key, deleted = deleted > 0, "Cache DEL");
        Ok(deleted > 0)
    }

    /// Check if a key exists.
    pub async fn exists(&self, key: &str) -> Result<bool, CacheError> {
        let full_key = self.build_key(key);
        let mut conn = self.connection_manager.clone();

        let exists: bool = conn.exists(&full_key).await?;
        Ok(exists)
    }

    /// Set TTL on an existing key.
    pub async fn expire(&self, key: &str, ttl_secs: u64) -> Result<bool, CacheError> {
        let full_key = self.build_key(key);
        let mut conn = self.connection_manager.clone();

        let updated: bool = conn.expire(&full_key, ttl_secs as i64).await?;
        Ok(updated)
    }

    /// Get TTL of a key.
    pub async fn ttl(&self, key: &str) -> Result<Option<i64>, CacheError> {
        let full_key = self.build_key(key);
        let mut conn = self.connection_manager.clone();

        let ttl: i64 = conn.ttl(&full_key).await?;
        if ttl < 0 {
            Ok(None)
        } else {
            Ok(Some(ttl))
        }
    }

    /// Delete multiple keys matching a pattern.
    pub async fn delete_pattern(&self, pattern: &str) -> Result<u64, CacheError> {
        let full_pattern = self.build_key(pattern);
        let mut conn = self.connection_manager.clone();

        let keys: Vec<String> = conn.keys(&full_pattern).await?;
        if keys.is_empty() {
            return Ok(0);
        }

        let deleted: u64 = conn.del(&keys).await?;
        tracing::debug!(pattern = %full_pattern, deleted = deleted, "Cache DEL pattern");
        Ok(deleted)
    }

    // =========================================================================
    // List / Queue Operations (used by background fan-out workers)
    // =========================================================================

    /// Append a raw string value to the tail of a Redis list (`RPUSH`).
    ///
    /// Used as a simple FIFO job queue: producers `RPUSH` and consumers
    /// `BLPOP` from the head. Returns the new length of the list.
    pub async fn queue_push(&self, key: &str, value: &str) -> Result<u64, CacheError> {
        let full_key = self.build_key(key);
        let mut conn = self.connection_manager.clone();

        let len: u64 = conn.rpush(&full_key, value).await?;
        tracing::trace!(key = %full_key, len = len, "Queue RPUSH");
        Ok(len)
    }

    /// Block until an element is available at the head of a Redis list, or the
    /// timeout elapses (`BLPOP`).
    ///
    /// `timeout_secs == 0` blocks indefinitely (standard Redis semantics).
    /// Returns `Ok(Some(value))` when an element was popped, or `Ok(None)` when
    /// the timeout elapsed with no element available.
    ///
    /// NOTE: `BLPOP` holds the underlying connection for up to `timeout_secs`.
    /// `ConnectionManager` multiplexes a single connection, so callers should
    /// use a modest timeout (a few seconds) rather than blocking forever, to
    /// avoid starving other commands that share the manager.
    pub async fn queue_pop_blocking(
        &self,
        key: &str,
        timeout_secs: f64,
    ) -> Result<Option<String>, CacheError> {
        let full_key = self.build_key(key);
        let mut conn = self.connection_manager.clone();

        // BLPOP returns `[key, value]` on success or nil (→ None) on timeout.
        let popped: Option<(String, String)> = conn.blpop(&full_key, timeout_secs).await?;
        match popped {
            Some((_popped_key, value)) => {
                tracing::trace!(key = %full_key, "Queue BLPOP hit");
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Return the current length of a Redis list (`LLEN`).
    ///
    /// Useful for queue-depth metrics / alerting.
    pub async fn queue_len(&self, key: &str) -> Result<u64, CacheError> {
        let full_key = self.build_key(key);
        let mut conn = self.connection_manager.clone();

        let len: u64 = conn.llen(&full_key).await?;
        Ok(len)
    }
}

// ============================================================================
// Story 103.3: Session Storage
// ============================================================================

/// Session data stored in Redis.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct SessionData {
    /// User ID.
    pub user_id: Uuid,

    /// Organization ID (if any).
    pub organization_id: Option<Uuid>,

    /// Session creation timestamp.
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last activity timestamp.
    pub last_activity: chrono::DateTime<chrono::Utc>,

    /// User agent string.
    pub user_agent: Option<String>,

    /// IP address.
    pub ip_address: Option<String>,

    /// Additional metadata.
    pub metadata: serde_json::Value,
}

impl SessionData {
    /// Create a new session.
    pub fn new(user_id: Uuid, organization_id: Option<Uuid>) -> Self {
        let now = chrono::Utc::now();
        Self {
            user_id,
            organization_id,
            created_at: now,
            last_activity: now,
            user_agent: None,
            ip_address: None,
            metadata: serde_json::json!({}),
        }
    }

    /// Set user agent.
    #[must_use]
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// Set IP address.
    #[must_use]
    pub fn with_ip_address(mut self, ip: impl Into<String>) -> Self {
        self.ip_address = Some(ip.into());
        self
    }
}

/// Redis session storage (Story 103.3).
#[derive(Clone)]
pub struct SessionStore {
    client: RedisClient,
    ttl_secs: u64,
}

impl SessionStore {
    /// Create a new session store.
    pub fn new(client: RedisClient) -> Self {
        let ttl_secs = client.config.default_session_ttl_secs;
        Self { client, ttl_secs }
    }

    /// Create a new session store with custom TTL.
    pub fn with_ttl(client: RedisClient, ttl_secs: u64) -> Self {
        Self { client, ttl_secs }
    }

    /// Build session key.
    fn session_key(session_id: &str) -> String {
        format!("{}{}", SESSION_KEY_PREFIX, session_id)
    }

    /// Build user sessions index key.
    fn user_sessions_key(user_id: Uuid) -> String {
        format!("{}user:{}", SESSION_KEY_PREFIX, user_id)
    }

    /// Create a new session (Story 103.3).
    pub async fn create(&self, session_id: &str, data: &SessionData) -> Result<(), CacheError> {
        let key = Self::session_key(session_id);

        // Store session data
        self.client.set(&key, data, Some(self.ttl_secs)).await?;

        // Add to user's session index
        let user_key = Self::user_sessions_key(data.user_id);
        let mut conn = self.client.connection_manager.clone();
        let _: () = conn.sadd(&user_key, session_id).await?;
        let _: () = conn.expire(&user_key, self.ttl_secs as i64).await?;

        tracing::debug!(
            session_id = %session_id,
            user_id = %data.user_id,
            ttl_secs = %self.ttl_secs,
            "Created session"
        );

        Ok(())
    }

    /// Get session data (Story 103.3).
    pub async fn get(&self, session_id: &str) -> Result<Option<SessionData>, CacheError> {
        let key = Self::session_key(session_id);
        self.client.get(&key).await
    }

    /// Update session last activity and extend TTL (Story 103.3).
    pub async fn touch(&self, session_id: &str) -> Result<bool, CacheError> {
        let key = Self::session_key(session_id);

        // Get current session
        let session: Option<SessionData> = self.client.get(&key).await?;

        if let Some(mut data) = session {
            data.last_activity = chrono::Utc::now();
            self.client.set(&key, &data, Some(self.ttl_secs)).await?;
            tracing::trace!(session_id = %session_id, "Session touched");
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Delete a session (Story 103.3).
    pub async fn delete(&self, session_id: &str) -> Result<bool, CacheError> {
        let key = Self::session_key(session_id);

        // Get user ID before deleting
        if let Some(data) = self.get(session_id).await? {
            // Remove from user's session index
            let user_key = Self::user_sessions_key(data.user_id);
            let mut conn = self.client.connection_manager.clone();
            let _: () = conn.srem(&user_key, session_id).await?;
        }

        let deleted = self.client.delete(&key).await?;
        tracing::debug!(session_id = %session_id, deleted = deleted, "Deleted session");
        Ok(deleted)
    }

    /// Delete all sessions for a user (Story 103.3).
    pub async fn delete_user_sessions(&self, user_id: Uuid) -> Result<u64, CacheError> {
        let user_key = Self::user_sessions_key(user_id);
        let mut conn = self.client.connection_manager.clone();

        // Get all session IDs for user
        let session_ids: Vec<String> = conn.smembers(&user_key).await?;

        let mut deleted = 0u64;
        for session_id in &session_ids {
            let key = Self::session_key(session_id);
            if self.client.delete(&key).await? {
                deleted += 1;
            }
        }

        // Delete the index
        let _: () = conn.del(&user_key).await?;

        tracing::info!(
            user_id = %user_id,
            deleted = deleted,
            "Deleted all user sessions"
        );

        Ok(deleted)
    }

    /// List all session IDs for a user (Story 103.3).
    pub async fn list_user_sessions(&self, user_id: Uuid) -> Result<Vec<String>, CacheError> {
        let user_key = Self::user_sessions_key(user_id);
        let mut conn = self.client.connection_manager.clone();

        let session_ids: Vec<String> = conn.smembers(&user_key).await?;
        Ok(session_ids)
    }
}

// ============================================================================
// Story 103.4: Pub/Sub Service
// ============================================================================

/// Pub/sub message.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct PubSubMessage {
    /// Message ID.
    pub id: Uuid,

    /// Channel name.
    pub channel: String,

    /// Event type.
    pub event_type: String,

    /// Message payload.
    pub payload: serde_json::Value,

    /// Timestamp.
    pub timestamp: chrono::DateTime<chrono::Utc>,

    /// Source instance ID (for filtering own messages).
    pub source_instance: Option<String>,
}

impl PubSubMessage {
    /// Create a new message.
    pub fn new(channel: &str, event_type: &str, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            channel: channel.to_string(),
            event_type: event_type.to_string(),
            payload,
            timestamp: chrono::Utc::now(),
            source_instance: None,
        }
    }

    /// Set source instance.
    #[must_use]
    pub fn with_source(mut self, instance_id: impl Into<String>) -> Self {
        self.source_instance = Some(instance_id.into());
        self
    }
}

/// Common event types for pub/sub.
pub mod event_types {
    /// User logged in.
    pub const USER_LOGGED_IN: &str = "user.logged_in";
    /// User logged out.
    pub const USER_LOGGED_OUT: &str = "user.logged_out";
    /// Session invalidated.
    pub const SESSION_INVALIDATED: &str = "session.invalidated";
    /// Cache invalidation.
    pub const CACHE_INVALIDATE: &str = "cache.invalidate";
    /// Real-time notification.
    pub const NOTIFICATION: &str = "notification";
    /// Fault status changed.
    pub const FAULT_STATUS_CHANGED: &str = "fault.status_changed";
    /// Vote cast.
    pub const VOTE_CAST: &str = "vote.cast";
    /// Document uploaded.
    pub const DOCUMENT_UPLOADED: &str = "document.uploaded";
}

/// Common channel names for pub/sub.
pub mod channels {
    /// Global broadcast channel.
    pub const GLOBAL: &str = "global";
    /// Session events channel.
    pub const SESSIONS: &str = "sessions";
    /// Cache invalidation channel.
    pub const CACHE: &str = "cache";
    /// Real-time notifications channel.
    pub const NOTIFICATIONS: &str = "notifications";

    /// Build organization-specific channel.
    pub fn organization(org_id: uuid::Uuid) -> String {
        format!("org:{}", org_id)
    }

    /// Build user-specific channel.
    pub fn user(user_id: uuid::Uuid) -> String {
        format!("user:{}", user_id)
    }

    /// Build building-specific channel.
    pub fn building(building_id: uuid::Uuid) -> String {
        format!("building:{}", building_id)
    }
}

/// Redis pub/sub service (Story 103.4).
#[derive(Clone)]
pub struct PubSubService {
    client: RedisClient,
    instance_id: String,
}

impl PubSubService {
    /// Create a new pub/sub service.
    pub fn new(client: RedisClient) -> Self {
        let instance_id = Uuid::new_v4().to_string();
        Self {
            client,
            instance_id,
        }
    }

    /// Create with a custom instance ID.
    pub fn with_instance_id(client: RedisClient, instance_id: impl Into<String>) -> Self {
        Self {
            client,
            instance_id: instance_id.into(),
        }
    }

    /// Build full channel name.
    fn build_channel(&self, channel: &str) -> String {
        format!("{}{}", PUBSUB_CHANNEL_PREFIX, channel)
    }

    /// Publish a message to a channel (Story 103.4).
    pub async fn publish(&self, channel: &str, message: PubSubMessage) -> Result<(), CacheError> {
        let full_channel = self.build_channel(channel);
        let message_with_source = PubSubMessage {
            source_instance: Some(self.instance_id.clone()),
            ..message
        };

        let serialized = serde_json::to_string(&message_with_source)
            .map_err(|e| CacheError::Serialization(e.to_string()))?;

        let mut conn = self.client.connection_manager.clone();
        let _: () = conn.publish(&full_channel, &serialized).await?;

        tracing::debug!(
            channel = %full_channel,
            event_type = %message_with_source.event_type,
            message_id = %message_with_source.id,
            "Published message"
        );

        Ok(())
    }

    /// Publish a simple event (Story 103.4).
    pub async fn publish_event(
        &self,
        channel: &str,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<(), CacheError> {
        let message = PubSubMessage::new(channel, event_type, payload);
        self.publish(channel, message).await
    }

    /// Subscribe to a channel and get a broadcast receiver (Story 103.4).
    ///
    /// Note: This creates a new connection for the subscription.
    /// Messages from the same instance are filtered out.
    ///
    /// The subscriber uses Redis SUBSCRIBE command with a dedicated connection.
    pub async fn subscribe(
        &self,
        channel: &str,
    ) -> Result<broadcast::Receiver<PubSubMessage>, CacheError> {
        let full_channel = self.build_channel(channel);
        let instance_id = self.instance_id.clone();
        let url = self.client.config.url.clone();

        // Create a broadcast channel for distributing messages
        let (tx, rx) = broadcast::channel::<PubSubMessage>(100);

        // Create a new client for subscription (pub/sub requires dedicated connection)
        let client = RedisClientInner::open(url.as_str())
            .map_err(|e| CacheError::Connection(format!("Failed to create client: {}", e)))?;

        let mut pubsub = client
            .get_async_pubsub()
            .await
            .map_err(|e| CacheError::Connection(format!("Failed to get connection: {}", e)))?;

        pubsub
            .subscribe(&full_channel)
            .await
            .map_err(|e| CacheError::PubSub(format!("Subscribe failed: {}", e)))?;

        tracing::info!(channel = %full_channel, "Subscribed to channel");

        // Spawn a task to forward messages
        let span_channel = full_channel.clone();
        tokio::spawn(
            async move {
                let mut pubsub_stream = pubsub.into_on_message();

                while let Some(msg) = futures_lite::StreamExt::next(&mut pubsub_stream).await {
                    let payload: Result<String, RedisError> = msg.get_payload();
                    if let Ok(payload) = payload {
                        if let Ok(message) = serde_json::from_str::<PubSubMessage>(&payload) {
                            // Filter out messages from same instance
                            if message.source_instance.as_ref() != Some(&instance_id) {
                                let _ = tx.send(message);
                            }
                        }
                    }
                }
            }
            .instrument(tracing::info_span!(
                "bg.redis_pubsub",
                channel = %span_channel,
            )),
        );

        Ok(rx)
    }

    /// Broadcast cache invalidation (Story 103.4).
    pub async fn invalidate_cache(&self, keys: Vec<String>) -> Result<(), CacheError> {
        self.publish_event(
            channels::CACHE,
            event_types::CACHE_INVALIDATE,
            serde_json::json!({ "keys": keys }),
        )
        .await
    }

    /// Broadcast session invalidation (Story 103.4).
    pub async fn invalidate_session(&self, session_id: &str) -> Result<(), CacheError> {
        self.publish_event(
            channels::SESSIONS,
            event_types::SESSION_INVALIDATED,
            serde_json::json!({ "session_id": session_id }),
        )
        .await
    }

    /// Send notification to a user (Story 103.4).
    pub async fn notify_user(
        &self,
        user_id: Uuid,
        notification: serde_json::Value,
    ) -> Result<(), CacheError> {
        let channel = channels::user(user_id);
        self.publish_event(&channel, event_types::NOTIFICATION, notification)
            .await
    }

    /// Send notification to an organization (Story 103.4).
    pub async fn notify_organization(
        &self,
        org_id: Uuid,
        notification: serde_json::Value,
    ) -> Result<(), CacheError> {
        let channel = channels::organization(org_id);
        self.publish_event(&channel, event_types::NOTIFICATION, notification)
            .await
    }

    /// Get the instance ID.
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    /// Borrow the underlying [`RedisClient`].
    ///
    /// Exposes the raw client so callers that hold a `PubSubService` (the only
    /// Redis handle threaded into the background workers) can run list/queue
    /// commands (`RPUSH` / `BLPOP` / `LLEN`) for fan-out job queues without a
    /// second connection handle.
    pub fn client(&self) -> &RedisClient {
        &self.client
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redis_config_from_env_defaults() {
        // Clear env var to test defaults
        std::env::remove_var(REDIS_URL_ENV);
        let config = RedisConfig::from_env().unwrap();
        assert_eq!(config.url, DEFAULT_REDIS_URL);
    }

    #[test]
    fn test_redis_config_new() {
        let config = RedisConfig::new("redis://localhost:6380")
            .with_key_prefix("test")
            .with_session_ttl(3600)
            .with_cache_ttl(300);

        assert_eq!(config.url, "redis://localhost:6380");
        assert_eq!(config.key_prefix, Some("test".to_string()));
        assert_eq!(config.default_session_ttl_secs, 3600);
        assert_eq!(config.default_cache_ttl_secs, 300);
    }

    #[test]
    fn test_session_data_new() {
        let user_id = Uuid::new_v4();
        let org_id = Uuid::new_v4();

        let session = SessionData::new(user_id, Some(org_id))
            .with_user_agent("TestAgent/1.0")
            .with_ip_address("127.0.0.1");

        assert_eq!(session.user_id, user_id);
        assert_eq!(session.organization_id, Some(org_id));
        assert_eq!(session.user_agent, Some("TestAgent/1.0".to_string()));
        assert_eq!(session.ip_address, Some("127.0.0.1".to_string()));
    }

    #[test]
    fn test_pubsub_message_new() {
        let msg = PubSubMessage::new(
            "test-channel",
            "test.event",
            serde_json::json!({"key": "value"}),
        )
        .with_source("instance-1");

        assert_eq!(msg.channel, "test-channel");
        assert_eq!(msg.event_type, "test.event");
        assert_eq!(msg.source_instance, Some("instance-1".to_string()));
    }

    #[test]
    fn test_channel_builders() {
        let org_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();
        let building_id = Uuid::new_v4();

        assert!(channels::organization(org_id).starts_with("org:"));
        assert!(channels::user(user_id).starts_with("user:"));
        assert!(channels::building(building_id).starts_with("building:"));
    }
}
