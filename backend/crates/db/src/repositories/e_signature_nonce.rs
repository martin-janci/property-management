//! E-signature nonce repository (issue #673 follow-up to PR #542).
//!
//! Persists nonces issued by `LightweightProvider::build_signing_url` so the
//! future `/sign` consumer endpoint can enforce one-shot use of a signed
//! envelope. Without this, an attacker who intercepts a signing URL can
//! replay it for the entire 7-day TTL.
//!
//! The hot operation is `record_nonce` — a single INSERT into
//! `e_signature_nonces`. The unique constraint `(envelope_id, nonce)` does
//! the replay-prevention work: a second INSERT with the same pair raises
//! Postgres error code `23505` (`unique_violation`), which we translate
//! into [`RecordNonceError::Replay`]. The API layer maps that to
//! `409 CONFLICT`.

use sqlx::{Error as SqlxError, PgPool};
use uuid::Uuid;

/// Repository for the `e_signature_nonces` table.
#[derive(Clone)]
pub struct ESignatureNonceRepository {
    pool: PgPool,
}

/// Outcome of attempting to record a freshly-issued or freshly-consumed nonce.
#[derive(Debug, thiserror::Error)]
pub enum RecordNonceError {
    /// The `(envelope_id, nonce)` pair is already on file — this is a replay.
    /// Surface as 409 CONFLICT at the API boundary.
    #[error("nonce already used for envelope")]
    Replay,
    /// Any other database failure (connection, RLS, FK, etc.).
    #[error("database error: {0}")]
    Database(#[from] SqlxError),
}

impl ESignatureNonceRepository {
    /// Construct a new repository against the given pool.
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Record a nonce as consumed for the given envelope.
    ///
    /// Returns `Ok(())` on first use, [`RecordNonceError::Replay`] if the
    /// `(envelope_id, nonce)` pair already exists, and
    /// [`RecordNonceError::Database`] for any other failure.
    pub async fn record_nonce(
        &self,
        envelope_id: Uuid,
        nonce: Uuid,
    ) -> Result<(), RecordNonceError> {
        let result = sqlx::query(
            r#"
            INSERT INTO e_signature_nonces (envelope_id, nonce)
            VALUES ($1, $2)
            "#,
        )
        .bind(envelope_id)
        .bind(nonce)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(SqlxError::Database(db_err)) if db_err.code().as_deref() == Some("23505") => {
                Err(RecordNonceError::Replay)
            }
            Err(e) => Err(RecordNonceError::Database(e)),
        }
    }

    /// Check whether a `(envelope_id, nonce)` pair has been recorded.
    ///
    /// Mostly useful for tests and audit endpoints; the hot path should
    /// rely on `record_nonce` returning [`RecordNonceError::Replay`].
    pub async fn nonce_exists(&self, envelope_id: Uuid, nonce: Uuid) -> Result<bool, SqlxError> {
        let row: Option<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT id FROM e_signature_nonces
            WHERE envelope_id = $1 AND nonce = $2
            LIMIT 1
            "#,
        )
        .bind(envelope_id)
        .bind(nonce)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.is_some())
    }
}

// Postgres SQLSTATE `23505` is `unique_violation`. Checked inline at the
// match arm above to avoid the `Box<dyn DatabaseError>` -> `&dyn DatabaseError`
// coercion dance.
