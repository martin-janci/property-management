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

use crate::models::portal::PortalUser;
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
    /// Writes the `users` row first (the source of truth), then the
    /// `portal_users` row with `pm_user_id = NULL` and the new `users.id`
    /// back-stamped via `users.portal_origin_id` so future reads can join
    /// either way. Both inserts run in the same transaction.
    ///
    /// `password_hash` is `None` for SSO-only accounts; in that case the
    /// `users` row gets the [`SSO_ONLY_SENTINEL`] hash that fails Argon2
    /// verification by construction.
    pub async fn create(
        &self,
        email: &str,
        name: &str,
        password_hash: Option<&str>,
        locale: Locale,
    ) -> Result<User, UnifiedPortalError> {
        let mut tx = self.pool.begin().await?;

        // 1) Insert the unified users row first — it is the source of truth.
        //    `principal_kind='public'` discriminates this from staff/platform.
        //    `email_verified_at` is left NULL; the registration flow issues a
        //    verification token and flips it later.
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
        .fetch_one(&mut *tx)
        .await?;

        // 2) Insert the portal_users back-compat row. `pm_user_id` is the
        //    legacy back-pointer to `users.id` — we set it to the new users
        //    row so the join works in both directions.
        let portal_user = sqlx::query_as::<_, PortalUser>(
            r#"
            INSERT INTO portal_users (email, name, password_hash, pm_user_id, provider)
            VALUES ($1, $2, $3, $4, 'local')
            RETURNING *
            "#,
        )
        .bind(email)
        .bind(name)
        .bind(password_hash)
        .bind(user.id)
        .fetch_one(&mut *tx)
        .await?;

        // 3) Set users.portal_origin_id back-pointer so a row written via
        //    this path looks identical to one inserted by the merge migration.
        sqlx::query(
            r#"
            UPDATE users SET portal_origin_id = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user.id)
        .bind(portal_user.id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        // Re-fetch so the returned User reflects the back-pointer.
        let refreshed = sqlx::query_as::<_, User>(r#"SELECT * FROM users WHERE id = $1"#)
            .bind(user.id)
            .fetch_one(&self.pool)
            .await?;
        Ok(refreshed)
    }

    /// Update profile fields on `users`, then mirror to `portal_users` if a
    /// back-pointer exists. Transactional.
    ///
    /// Returns `Ok(None)` if no `users` row matches `user_id` (or the user
    /// was soft-deleted) — this is a no-op, not an error, so callers can
    /// distinguish "no such user" from "update failed".
    pub async fn update_profile(
        &self,
        user_id: Uuid,
        data: UpdateProfile,
    ) -> Result<Option<User>, UnifiedPortalError> {
        let mut tx = self.pool.begin().await?;

        // 1) Mirror writable fields onto `users`. `phone` is intentionally
        //    untouched here — portal users don't expose phone in the profile
        //    edit API, and the staff path owns phone updates.
        let updated = sqlx::query_as::<_, User>(
            r#"
            UPDATE users SET
                name   = COALESCE($2, name),
                locale = COALESCE($3, locale),
                updated_at = NOW()
            WHERE id = $1
              AND status != 'deleted'
              AND principal_kind = 'public'
            RETURNING *
            "#,
        )
        .bind(user_id)
        .bind(&data.name)
        .bind(&data.locale)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(updated) = updated else {
            tx.rollback().await?;
            return Ok(None);
        };

        // 2) Mirror to portal_users if and only if a back-pointer was
        //    established. We match BOTH directions:
        //      * users.portal_origin_id  → portal_users.id
        //      * portal_users.pm_user_id → users.id
        //    so rows created by the new repo, by the legacy dual-write in
        //    PortalRepository::create_user, and by the merge migration are
        //    all mirrored correctly.
        sqlx::query(
            r#"
            UPDATE portal_users SET
                name              = COALESCE($2, name),
                profile_image_url = COALESCE($3, profile_image_url),
                locale            = COALESCE($4, locale),
                updated_at = NOW()
            WHERE id = (SELECT portal_origin_id FROM users WHERE id = $1)
               OR pm_user_id = $1
            "#,
        )
        .bind(user_id)
        .bind(&data.name)
        .bind(&data.profile_image_url)
        .bind(&data.locale)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(Some(updated))
    }

    /// Update the password hash on `users`, then mirror to `portal_users` if
    /// a back-pointer exists. Transactional — both rows succeed or neither
    /// does, so a partial write can never leave the two hashes out of sync.
    pub async fn update_password_hash(
        &self,
        user_id: Uuid,
        password_hash: &str,
    ) -> Result<bool, UnifiedPortalError> {
        let mut tx = self.pool.begin().await?;

        let users_rows = sqlx::query(
            r#"
            UPDATE users SET password_hash = $2, updated_at = NOW()
            WHERE id = $1
              AND status != 'deleted'
              AND principal_kind = 'public'
            "#,
        )
        .bind(user_id)
        .bind(password_hash)
        .execute(&mut *tx)
        .await?
        .rows_affected();

        if users_rows == 0 {
            tx.rollback().await?;
            return Ok(false);
        }

        sqlx::query(
            r#"
            UPDATE portal_users SET password_hash = $2, updated_at = NOW()
            WHERE id = (SELECT portal_origin_id FROM users WHERE id = $1)
               OR pm_user_id = $1
            "#,
        )
        .bind(user_id)
        .bind(password_hash)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(true)
    }

    /// Find a public-kind user by email.
    ///
    /// Reads from `users` filtered by `principal_kind='public'`. Falls back
    /// to a `portal_users` lookup ONLY when `users` returns nothing AND a
    /// `portal_users` row exists with no `pm_user_id` back-pointer — that
    /// pattern is the unmerged-collision case (a portal user was queued in
    /// `user_merge_collisions` because some other principal owned the email
    /// in `users`). We log a warning and synthesize a User-shaped projection
    /// from the portal row so the caller can still authenticate the legacy
    /// row, but we do NOT silently merge the two identities.
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

        if user.is_some() {
            return Ok(user);
        }

        // Fallback: an unmerged portal_users row may exist (collision queued).
        // We project it into the User shape so authn keeps working for the
        // legacy row, but we explicitly DO NOT touch users.
        let portal = sqlx::query_as::<_, PortalUser>(
            r#"
            SELECT * FROM portal_users
             WHERE LOWER(email) = LOWER($1)
               AND pm_user_id IS NULL
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(p) = portal {
            tracing::warn!(
                email = %email,
                portal_user_id = %p.id,
                "UnifiedPortalUserRepo::find_by_email fell back to portal_users — \
                 unmerged identity, see user_merge_collisions table"
            );
            // Synthesize a User projection. status='active' because portal
            // user existence implies the user authenticated at some point;
            // we set principal_kind='public' so downstream RequestPrincipal
            // checks behave correctly. The real id stays portal_users.id —
            // callers that look up the row again via UserRepository will
            // see the same shape.
            let synthesized = User {
                id: p.id,
                email: p.email,
                password_hash: p
                    .password_hash
                    .unwrap_or_else(|| SSO_ONLY_SENTINEL.to_string()),
                name: p.name,
                phone: None,
                status: "active".to_string(),
                email_verified_at: if p.email_verified {
                    Some(p.updated_at)
                } else {
                    None
                },
                invited_at: None,
                invited_by: None,
                suspended_at: None,
                suspended_by: None,
                deleted_at: None,
                deleted_by: None,
                locale: p.locale,
                profile_visibility: "visible".to_string(),
                show_contact_info: false,
                scheduled_deletion_at: None,
                principal_kind: "public".to_string(),
                portal_origin_id: Some(p.id),
                created_at: p.created_at,
                updated_at: p.updated_at,
            };
            return Ok(Some(synthesized));
        }

        Ok(None)
    }

    /// Upsert a public user from an SSO provider, dual-writing to both
    /// tables.
    ///
    /// Behavior:
    ///
    /// * No row in either table → INSERT into `users` (kind=`public`) and
    ///   `portal_users` (provider=$provider, `pm_user_id` back-pointer to
    ///   the new users row). Linked via `users.portal_origin_id`.
    /// * Existing `users` row with `principal_kind='public'` → UPDATE name
    ///   (and, if `portal_origin_id` is set, mirror the update to
    ///   `portal_users`). Idempotent SSO sign-in.
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

            // Public principal — idempotent update of the display name and
            // (if linked) mirror to portal_users.
            let updated = sqlx::query_as::<_, User>(
                r#"
                UPDATE users SET
                    name = $2,
                    updated_at = NOW()
                WHERE id = $1
                RETURNING *
                "#,
            )
            .bind(u.id)
            .bind(name)
            .fetch_one(&mut *tx)
            .await?;

            sqlx::query(
                r#"
                UPDATE portal_users SET
                    name     = $2,
                    provider = $3,
                    email_verified = TRUE,
                    updated_at = NOW()
                WHERE id = (SELECT portal_origin_id FROM users WHERE id = $1)
                   OR pm_user_id = $1
                "#,
            )
            .bind(u.id)
            .bind(name)
            .bind(provider)
            .execute(&mut *tx)
            .await?;

            tx.commit().await?;
            return Ok(updated);
        }

        // 2) No users row exists — insert both tables.
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

        let portal_user = sqlx::query_as::<_, PortalUser>(
            r#"
            INSERT INTO portal_users (
                email, name, password_hash, pm_user_id, provider, email_verified
            )
            VALUES ($1, $2, NULL, $3, $4, TRUE)
            RETURNING *
            "#,
        )
        .bind(email)
        .bind(name)
        .bind(user.id)
        .bind(provider)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            UPDATE users SET portal_origin_id = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user.id)
        .bind(portal_user.id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        let refreshed = sqlx::query_as::<_, User>(r#"SELECT * FROM users WHERE id = $1"#)
            .bind(user.id)
            .fetch_one(&self.pool)
            .await?;
        Ok(refreshed)
    }
}
