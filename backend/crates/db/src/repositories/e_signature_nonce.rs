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

/// Read-only lifecycle state of a `(envelope_id, nonce)` pair, used by the
/// `/sign` GET render-context handler to decide whether to show a signing
/// form. Consumption itself goes through [`ESignatureNonceRepository::consume_nonce`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NonceState {
    /// No row for this pair — the nonce was never issued for this envelope
    /// (forged or for a different envelope). Map to 404/401.
    NotIssued,
    /// Issued (row exists, `consumed_at IS NULL`) and not yet spent — a
    /// `/sign` POST may consume it.
    Pending,
    /// Already consumed (`consumed_at` set) — a replay. Map to 409 CONFLICT.
    Consumed,
}

/// Result of an atomic single-use consume attempt via
/// [`ESignatureNonceRepository::consume_nonce`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsumeOutcome {
    /// This call flipped `consumed_at` NULL -> NOW(); the signer may proceed.
    Consumed,
    /// The nonce was issued but a prior call already consumed it — replay.
    /// Map to 409 CONFLICT.
    AlreadyConsumed,
    /// No row for this `(envelope_id, nonce)` pair — never issued. Map to
    /// 401/404 (an unsigned/forged nonce that nonetheless passed HMAC would
    /// be extraordinary, but we fail closed).
    NotIssued,
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

    /// Read the lifecycle state of a `(envelope_id, nonce)` pair WITHOUT
    /// mutating it. Used by the `/sign` GET render-context handler so it can
    /// distinguish "show the signing form" (Pending) from "already signed"
    /// (Consumed) and "unknown link" (NotIssued).
    pub async fn nonce_state(
        &self,
        envelope_id: Uuid,
        nonce: Uuid,
    ) -> Result<NonceState, SqlxError> {
        // `consumed_at` is nullable; the outer Option is "row present?", the
        // inner is "consumed?". A `::text` cast is unnecessary here — the
        // column is a plain TIMESTAMPTZ, decoded directly.
        let row: Option<(Option<chrono::DateTime<chrono::Utc>>,)> = sqlx::query_as(
            r#"
            SELECT consumed_at FROM e_signature_nonces
            WHERE envelope_id = $1 AND nonce = $2
            LIMIT 1
            "#,
        )
        .bind(envelope_id)
        .bind(nonce)
        .fetch_optional(&self.pool)
        .await?;

        Ok(match row {
            None => NonceState::NotIssued,
            Some((None,)) => NonceState::Pending,
            Some((Some(_),)) => NonceState::Consumed,
        })
    }

    /// Atomically consume an issued nonce (single-use).
    ///
    /// Flips `consumed_at` from `NULL` to `NOW()` in a single statement, so
    /// two concurrent `/sign` POSTs racing on the same link cannot both win:
    /// exactly one `UPDATE ... WHERE consumed_at IS NULL` matches a row, the
    /// other matches zero. The loser, and every later replay, is reported as
    /// [`ConsumeOutcome::AlreadyConsumed`] (→ 409). A pair that was never
    /// issued is [`ConsumeOutcome::NotIssued`] (→ 401/404).
    pub async fn consume_nonce(
        &self,
        envelope_id: Uuid,
        nonce: Uuid,
    ) -> Result<ConsumeOutcome, SqlxError> {
        let consumed: Option<(Uuid,)> = sqlx::query_as(
            r#"
            UPDATE e_signature_nonces
            SET consumed_at = NOW()
            WHERE envelope_id = $1 AND nonce = $2 AND consumed_at IS NULL
            RETURNING id
            "#,
        )
        .bind(envelope_id)
        .bind(nonce)
        .fetch_optional(&self.pool)
        .await?;

        if consumed.is_some() {
            return Ok(ConsumeOutcome::Consumed);
        }

        // Zero rows updated: either the pair was never issued, or it was
        // already consumed by a prior call. Distinguish for the caller.
        if self.nonce_exists(envelope_id, nonce).await? {
            Ok(ConsumeOutcome::AlreadyConsumed)
        } else {
            Ok(ConsumeOutcome::NotIssued)
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
