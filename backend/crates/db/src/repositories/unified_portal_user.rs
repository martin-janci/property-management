//! Unified portal user repository (Phase 2.5 — N1 follow-up to Identity Unification).
//!
//! Phase 2 added `principal_kind` and `portal_origin_id` to the `users` table
//! and ran a one-shot merge migration (00132) populating `users` from
//! `portal_users`. The forward-going invariant is: **every public/portal user
//! has exactly one row in `users` (the source of truth) and at most one row
//! in `portal_users` (back-compat shim that future cleanup will retire).**
//!
//! [`PortalRepository`] still exists and still does its OWN dual-write inside
//! `create_user`, but it does NOT mirror updates: `update_user`,
//! `update_password_hash`, and `upsert_sso_user` only touch `portal_users`.
//! That gap is exactly leak R1 from the Phase 2 deep-dive review.
//!
//! This repository fixes the gap by making `users` authoritative for every
//! write path:
//!
//! | Operation                  | `users`                             | `portal_users`                                    |
//! |----------------------------|-------------------------------------|---------------------------------------------------|
//! | [`create`]                 | INSERT (`principal_kind='public'`)  | INSERT with `pm_user_id` back-pointer             |
//! | [`update_profile`]         | UPDATE first                        | UPDATE if a back-pointer exists                   |
//! | [`update_password_hash`]   | UPDATE first                        | UPDATE if a back-pointer exists                   |
//! | [`sso_upsert`]             | INSERT … ON CONFLICT (email)        | UPSERT mirror                                     |
//!
//! All writes that touch both tables run inside a single transaction so a
//! crash never leaves the rows desynchronized. Email collisions are NEVER
//! silently merged — when [`sso_upsert`] sees an existing `users` row whose
//! `principal_kind != 'public'` (a staff or platform principal already owns
//! that email), the operation refuses with [`UnifiedPortalError::Collision`]
//! and writes a `user_merge_collisions` row for human review (defends leak
//! #7 — same as the migration's contract).
//!
//! Read paths stay on [`PortalRepository`]: it reads what the dual-write
//! writes, so existing handlers don't need to change to pick up the new
//! invariant.

// PortalUser import removed in Phase 6 — portal_users table dropped (migration 00148).
use crate::models::user::{Locale, User};
use crate::DbPool;
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// Sentinel password hash used for SSO-only accounts. The string `"!sso..."`
/// is not a valid Argon2id PHC encoding, so [`argon2::PasswordHash::new`]
/// rejects it and any login attempt with this hash fails closed.
const SSO_ONLY_SENTINEL: &str = "!sso-only-no-password";

/// Errors returned by [`UnifiedPortalUserRepo`].
///
/// Distinct from [`sqlx::Error`] specifically so callers can pattern-match
/// the "merge collision detected; do NOT silently overwrite" case.
#[derive(Debug, thiserror::Error)]
pub enum UnifiedPortalError {
    /// Underlying database error.
    #[error("database error: {0}")]
    Db(#[from] SqlxError),
    /// An existing `users` row was found for the same email but with a
    /// different `principal_kind` (e.g. a staff or platform principal). The
    /// operation refused; a row was written to `user_merge_collisions` for
    /// human review.
    #[error("email collides with existing non-public principal (collision queued)")]
    Collision { existing_user_id: Uuid },
}

/// Profile fields a portal user can update via the API.
#[derive(Debug, Clone, Default)]
pub struct UpdateProfile {
    pub name: Option<String>,
    pub profile_image_url: Option<String>,
    pub locale: Option<String>,
}

/// Phase 2.5 unified write path for public (portal) users.
///
/// Wraps the same [`DbPool`] as [`PortalRepository`]. Uses a transaction for
/// every operation that touches both tables, so a crash mid-write never
/// leaves the rows desynchronized.
#[derive(Clone)]
pub struct UnifiedPortalUserRepo {
    pool: DbPool,
}

impl UnifiedPortalUserRepo {
    /// Create a new unified portal user repository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Create a new public/portal user.
    ///
    /// Phase 6: writes only to `users` (principal_kind='public'). The
    /// `portal_users` dual-write was removed as part of migration 00148 which
    /// dropped that table entirely.
    ///
    /// `password_hash` is `None` for SSO-only accounts; the row gets the
    /// [`SSO_ONLY_SENTINEL`] hash that fails Argon2 verification by construction.
    pub async fn create(
        &self,
        email: &str,
        name: &str,
        password_hash: Option<&str>,
        locale: Locale,
    ) -> Result<User, UnifiedPortalError> {
        // Single-table insert — no transaction needed.
        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (
                email, password_hash, name, locale, status, email_verified_at,
                principal_kind, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, 'active', NULL, 'public', NOW(), NOW())
            RETURNING *
            "#,
        )
        .bind(email)
        .bind(password_hash.unwrap_or(SSO_ONLY_SENTINEL))
        .bind(name)
        .bind(locale.as_str())
        .fetch_one(&self.pool)
        .await?;

        Ok(user)
    }

    /// Update profile fields on `users`.
    ///
    /// Phase 6: `portal_users` has been dropped (migration 00148). Updates
    /// `users` directly — no mirror step needed.
    ///
    /// Returns `Ok(None)` if no `users` row matches `user_id` (or the user
    /// was soft-deleted) — this is a no-op, not an error, so callers can
    /// distinguish "no such user" from "update failed".
    pub async fn update_profile(
        &self,
        user_id: Uuid,
        data: UpdateProfile,
    ) -> Result<Option<User>, UnifiedPortalError> {
        let updated = sqlx::query_as::<_, User>(
            r#"
            UPDATE users SET
                name              = COALESCE($2, name),
                profile_image_url = COALESCE($3, profile_image_url),
                locale            = COALESCE($4, locale),
                updated_at        = NOW()
            WHERE id = $1
              AND status != 'deleted'
              AND principal_kind = 'public'
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(&data.name)
        .bind(&data.profile_image_url)
        .bind(&data.locale)
        .fetch_optional(&self.pool)
        .await?;

        Ok(updated)
    }

    /// Update the password hash on `users`.
    ///
    /// Phase 6: `portal_users` has been dropped (migration 00148). Updates
    /// `users` directly — no mirror step needed.
    pub async fn update_password_hash(
        &self,
        user_id: Uuid,
        password_hash: &str,
    ) -> Result<bool, UnifiedPortalError> {
        let rows = sqlx::query(
            r#"
            UPDATE users SET password_hash = $2, updated_at = NOW()
            WHERE id = $1
              AND status != 'deleted'
              AND principal_kind = 'public'
            "#,
        )
        .bind(user_id)
        .bind(password_hash)
        .execute(&self.pool)
        .await?
        .rows_affected();

        Ok(rows > 0)
    }

    /// Find a public-kind user by email.
    ///
    /// Phase 6: reads from `users` only (`portal_users` dropped in 00148).
    /// The legacy fallback to `portal_users` for unmerged-collision rows is
    /// removed — any such row was merged into `users` by the 00148 migration's
    /// back-fill step.
    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, UnifiedPortalError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users
             WHERE LOWER(email) = LOWER($1)
               AND principal_kind = 'public'
               AND status != 'deleted'
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(user)
    }

    /// Upsert a public user from an SSO provider.
    ///
    /// Phase 6: writes to `users` only (`portal_users` dropped in 00148).
    ///
    /// Behavior:
    ///
    /// * No existing row → INSERT into `users` (kind=`public`).
    /// * Existing `users` row with `principal_kind='public'` → UPDATE name.
    ///   Idempotent SSO sign-in.
    /// * Existing `users` row with `principal_kind` != `'public'` → REFUSE.
    ///   Writes a `user_merge_collisions` row, returns
    ///   [`UnifiedPortalError::Collision`]. NEVER silently overwrites a
    ///   staff or platform principal's account just because their email
    ///   shows up at an SSO IdP. (Defends leak #7 the same way the migration
    ///   does.)
    pub async fn sso_upsert(
        &self,
        provider: &str,
        provider_user_id: Option<Uuid>,
        email: &str,
        name: &str,
    ) -> Result<User, UnifiedPortalError> {
        let mut tx = self.pool.begin().await?;

        // 1) Look up an existing users row by email (case-insensitive).
        let existing = sqlx::query_as::<_, User>(
            r#"
            SELECT * FROM users
             WHERE LOWER(email) = LOWER($1)
               AND status != 'deleted'
             FOR UPDATE
            "#,
        )
        .bind(email)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(u) = existing {
            if u.principal_kind != "public" {
                // Staff or platform principal owns this email. Queue a
                // collision row and refuse — the same contract the merge
                // migration enforces.
                sqlx::query(
                    r#"
                    INSERT INTO user_merge_collisions (
                        source_table, source_id, target_email, payload, status
                    )
                    SELECT 'users', $1, $2,
                           jsonb_build_object(
                               'reason',          'sso_upsert_collision_with_non_public_principal',
                               'sso_provider',    $3::text,
                               'sso_provider_uid', $4::uuid,
                               'sso_email',       $2::text,
                               'sso_name',        $5::text,
                               'existing_kind',   $6::text
                           ),
                           'pending'
                    WHERE NOT EXISTS (
                        SELECT 1 FROM user_merge_collisions c
                         WHERE c.source_table = 'users'
                           AND c.source_id    = $1
                           AND c.status       = 'pending'
                    )
                    "#,
                )
                .bind(u.id)
                .bind(email)
                .bind(provider)
                .bind(provider_user_id)
                .bind(name)
                .bind(&u.principal_kind)
                .execute(&mut *tx)
                .await?;

                tx.commit().await?;
                return Err(UnifiedPortalError::Collision {
                    existing_user_id: u.id,
                });
            }

            // Public principal — idempotent update of the display name.
            // Phase 6: portal_users mirror removed (table dropped in 00148).
            let updated = sqlx::query_as::<_, User>(
                r#"
                UPDATE users SET
                    name       = $2,
                    updated_at = NOW()
                WHERE id = $1
                RETURNING *
                "#,
            )
            .bind(u.id)
            .bind(name)
            .fetch_one(&mut *tx)
            .await?;

            tx.commit().await?;
            return Ok(updated);
        }

        // 2) No users row exists — insert into users only.
        // Phase 6: portal_users dual-write removed (table dropped in 00148).
        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (
                email, password_hash, name, locale, status, email_verified_at,
                principal_kind, created_at, updated_at
            )
            VALUES ($1, $2, $3, 'en', 'active', NOW(), 'public', NOW(), NOW())
            RETURNING *
            "#,
        )
        .bind(email)
        .bind(SSO_ONLY_SENTINEL)
        .bind(name)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(user)
    }
}
