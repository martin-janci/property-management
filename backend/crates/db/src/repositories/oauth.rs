//! OAuth 2.0 Provider repository (Epic 10A).
//!
//! This module provides database operations for OAuth 2.0 implementation
//! including clients, authorization codes, access tokens, and refresh tokens.

use crate::models::oauth::{
    CreateAccessToken, CreateAuthorizationCode, CreateOAuthClient, CreateRefreshToken,
    CreateUserOAuthGrant, OAuthAccessToken, OAuthAuthorizationCode, OAuthClient, OAuthRefreshToken,
    UpdateOAuthClient, UserGrantWithClientRow, UserOAuthGrant,
};
use crate::DbPool;
use sqlx::Error as SqlxError;
use uuid::Uuid;

/// Repository for OAuth 2.0 operations.
#[derive(Clone)]
pub struct OAuthRepository {
    pool: DbPool,
}

impl OAuthRepository {
    /// Create a new OAuthRepository.
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    // ==================== OAuth Clients ====================

    /// Create a new OAuth client.
    pub async fn create_client(&self, data: CreateOAuthClient) -> Result<OAuthClient, SqlxError> {
        let client = sqlx::query_as::<_, OAuthClient>(
            r#"
            INSERT INTO oauth_clients (client_id, client_secret_hash, name, description, redirect_uris, scopes, is_confidential, rotate_refresh_tokens)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(&data.client_id)
        .bind(&data.client_secret_hash)
        .bind(&data.name)
        .bind(&data.description)
        .bind(sqlx::types::Json(&data.redirect_uris))
        .bind(sqlx::types::Json(&data.scopes))
        .bind(data.is_confidential)
        .bind(data.rotate_refresh_tokens)
        .fetch_one(&self.pool)
        .await?;

        Ok(client)
    }

    /// Find OAuth client by client_id.
    pub async fn find_client_by_client_id(
        &self,
        client_id: &str,
    ) -> Result<Option<OAuthClient>, SqlxError> {
        let client = sqlx::query_as::<_, OAuthClient>(
            r#"
            SELECT * FROM oauth_clients WHERE client_id = $1
            "#,
        )
        .bind(client_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(client)
    }

    /// Find active OAuth client by client_id.
    pub async fn find_active_client_by_client_id(
        &self,
        client_id: &str,
    ) -> Result<Option<OAuthClient>, SqlxError> {
        let client = sqlx::query_as::<_, OAuthClient>(
            r#"
            SELECT * FROM oauth_clients WHERE client_id = $1 AND is_active = true
            "#,
        )
        .bind(client_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(client)
    }

    /// Find OAuth client by UUID.
    pub async fn find_client_by_id(&self, id: Uuid) -> Result<Option<OAuthClient>, SqlxError> {
        let client = sqlx::query_as::<_, OAuthClient>(
            r#"
            SELECT * FROM oauth_clients WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(client)
    }

    /// List all OAuth clients.
    pub async fn list_clients(&self) -> Result<Vec<OAuthClient>, SqlxError> {
        let clients = sqlx::query_as::<_, OAuthClient>(
            r#"
            SELECT * FROM oauth_clients ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(clients)
    }

    /// Update an OAuth client.
    pub async fn update_client(
        &self,
        id: Uuid,
        data: UpdateOAuthClient,
    ) -> Result<Option<OAuthClient>, SqlxError> {
        let client = sqlx::query_as::<_, OAuthClient>(
            r#"
            UPDATE oauth_clients
            SET name = COALESCE($2, name),
                description = COALESCE($3, description),
                redirect_uris = COALESCE($4, redirect_uris),
                scopes = COALESCE($5, scopes),
                is_active = COALESCE($6, is_active),
                rotate_refresh_tokens = COALESCE($7, rotate_refresh_tokens),
                updated_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&data.name)
        .bind(&data.description)
        .bind(data.redirect_uris.as_ref().map(sqlx::types::Json))
        .bind(data.scopes.as_ref().map(sqlx::types::Json))
        .bind(data.is_active)
        .bind(data.rotate_refresh_tokens)
        .fetch_optional(&self.pool)
        .await?;

        Ok(client)
    }

    /// Update client secret.
    pub async fn update_client_secret(
        &self,
        id: Uuid,
        secret_hash: &str,
    ) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE oauth_clients SET client_secret_hash = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(secret_hash)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Deactivate an OAuth client and revoke all its tokens.
    pub async fn revoke_client(&self, id: Uuid) -> Result<bool, SqlxError> {
        // Start a transaction to ensure atomicity
        let mut tx = self.pool.begin().await?;

        // First, get the client_id for token revocation
        let client =
            sqlx::query_as::<_, OAuthClient>(r#"SELECT * FROM oauth_clients WHERE id = $1"#)
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;

        let Some(client) = client else {
            return Ok(false);
        };

        // Revoke all access tokens
        sqlx::query(
            r#"
            UPDATE oauth_access_tokens SET revoked_at = NOW()
            WHERE client_id = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(&client.client_id)
        .execute(&mut *tx)
        .await?;

        // Revoke all refresh tokens
        sqlx::query(
            r#"
            UPDATE oauth_refresh_tokens SET revoked_at = NOW()
            WHERE client_id = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(&client.client_id)
        .execute(&mut *tx)
        .await?;

        // Deactivate the client
        let result = sqlx::query(
            r#"
            UPDATE oauth_clients SET is_active = false, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(result.rows_affected() > 0)
    }

    // ==================== Authorization Codes ====================

    /// Create a new authorization code.
    pub async fn create_authorization_code(
        &self,
        data: CreateAuthorizationCode,
    ) -> Result<OAuthAuthorizationCode, SqlxError> {
        let code = sqlx::query_as::<_, OAuthAuthorizationCode>(
            r#"
            INSERT INTO oauth_authorization_codes (user_id, client_id, code_hash, scopes, redirect_uri, code_challenge, code_challenge_method, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(data.user_id)
        .bind(&data.client_id)
        .bind(&data.code_hash)
        .bind(sqlx::types::Json(&data.scopes))
        .bind(&data.redirect_uri)
        .bind(&data.code_challenge)
        .bind(&data.code_challenge_method)
        .bind(data.expires_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(code)
    }

    /// Find authorization code by hash.
    pub async fn find_authorization_code_by_hash(
        &self,
        code_hash: &str,
    ) -> Result<Option<OAuthAuthorizationCode>, SqlxError> {
        let code = sqlx::query_as::<_, OAuthAuthorizationCode>(
            r#"
            SELECT * FROM oauth_authorization_codes
            WHERE code_hash = $1 AND used_at IS NULL AND expires_at > NOW()
            "#,
        )
        .bind(code_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(code)
    }

    /// Mark authorization code as used.
    pub async fn consume_authorization_code(&self, id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE oauth_authorization_codes SET used_at = NOW()
            WHERE id = $1 AND used_at IS NULL
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Atomically find and consume authorization code in a single operation.
    /// This prevents TOCTOU race conditions.
    pub async fn find_and_consume_authorization_code(
        &self,
        code_hash: &str,
    ) -> Result<Option<OAuthAuthorizationCode>, SqlxError> {
        // Use UPDATE ... RETURNING to atomically mark as used and return the code
        let code = sqlx::query_as::<_, OAuthAuthorizationCode>(
            r#"
            UPDATE oauth_authorization_codes
            SET used_at = NOW()
            WHERE code_hash = $1 AND used_at IS NULL AND expires_at > NOW()
            RETURNING *
            "#,
        )
        .bind(code_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(code)
    }

    // ==================== Access Tokens ====================

    /// Create a new access token.
    pub async fn create_access_token(
        &self,
        data: CreateAccessToken,
    ) -> Result<OAuthAccessToken, SqlxError> {
        let token = sqlx::query_as::<_, OAuthAccessToken>(
            r#"
            INSERT INTO oauth_access_tokens (user_id, client_id, token_hash, scopes, expires_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(data.user_id)
        .bind(&data.client_id)
        .bind(&data.token_hash)
        .bind(sqlx::types::Json(&data.scopes))
        .bind(data.expires_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(token)
    }

    /// Find access token by hash.
    pub async fn find_access_token_by_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<OAuthAccessToken>, SqlxError> {
        let token = sqlx::query_as::<_, OAuthAccessToken>(
            r#"
            SELECT * FROM oauth_access_tokens
            WHERE token_hash = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(token)
    }

    /// Revoke an access token.
    pub async fn revoke_access_token(&self, id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE oauth_access_tokens SET revoked_at = NOW()
            WHERE id = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Revoke access token by hash.
    pub async fn revoke_access_token_by_hash(&self, token_hash: &str) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE oauth_access_tokens SET revoked_at = NOW()
            WHERE token_hash = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(token_hash)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    // ==================== Refresh Tokens ====================

    /// Create a new refresh token.
    pub async fn create_refresh_token(
        &self,
        data: CreateRefreshToken,
    ) -> Result<OAuthRefreshToken, SqlxError> {
        let token = sqlx::query_as::<_, OAuthRefreshToken>(
            r#"
            INSERT INTO oauth_refresh_tokens (user_id, client_id, token_hash, scopes, family_id, expires_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(data.user_id)
        .bind(&data.client_id)
        .bind(&data.token_hash)
        .bind(sqlx::types::Json(&data.scopes))
        .bind(data.family_id)
        .bind(data.expires_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(token)
    }

    /// Find refresh token by hash.
    ///
    /// Production lookup — filters out revoked tokens so a revoked refresh
    /// token cannot be exchanged for new access tokens (RFC 9700).
    pub async fn find_refresh_token_by_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<OAuthRefreshToken>, SqlxError> {
        let token = sqlx::query_as::<_, OAuthRefreshToken>(
            r#"
            SELECT * FROM oauth_refresh_tokens
            WHERE token_hash = $1
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(token)
    }

    /// Find refresh token by hash, INCLUDING revoked tokens.
    ///
    /// Used exclusively by the family-reuse detection path in
    /// `refresh_tokens`: if a caller presents a token we previously revoked,
    /// we must still see it so we can flag replay and burn the entire token
    /// family. Do not use this in any other code path — it bypasses the
    /// revoked filter that protects the production grant flow.
    pub async fn find_refresh_token_by_hash_including_revoked(
        &self,
        token_hash: &str,
    ) -> Result<Option<OAuthRefreshToken>, SqlxError> {
        let token = sqlx::query_as::<_, OAuthRefreshToken>(
            r#"
            SELECT * FROM oauth_refresh_tokens
            WHERE token_hash = $1
            "#,
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await?;

        Ok(token)
    }

    /// Revoke a refresh token.
    pub async fn revoke_refresh_token(&self, id: Uuid) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE oauth_refresh_tokens SET revoked_at = NOW()
            WHERE id = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Revoke refresh token by hash.
    pub async fn revoke_refresh_token_by_hash(&self, token_hash: &str) -> Result<bool, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE oauth_refresh_tokens SET revoked_at = NOW()
            WHERE token_hash = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(token_hash)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Revoke all refresh tokens in a family (for token reuse detection).
    pub async fn revoke_token_family(&self, family_id: Uuid) -> Result<u64, SqlxError> {
        let result = sqlx::query(
            r#"
            UPDATE oauth_refresh_tokens SET revoked_at = NOW()
            WHERE family_id = $1 AND revoked_at IS NULL
            "#,
        )
        .bind(family_id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    // ==================== User OAuth Grants ====================

    /// Create or update a user OAuth grant.
    pub async fn upsert_user_grant(
        &self,
        data: CreateUserOAuthGrant,
    ) -> Result<UserOAuthGrant, SqlxError> {
        let grant = sqlx::query_as::<_, UserOAuthGrant>(
            r#"
            INSERT INTO user_oauth_grants (user_id, client_id, scopes)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id, client_id)
            DO UPDATE SET scopes = $3, revoked_at = NULL, granted_at = NOW()
            RETURNING *
            "#,
        )
        .bind(data.user_id)
        .bind(&data.client_id)
        .bind(sqlx::types::Json(&data.scopes))
        .fetch_one(&self.pool)
        .await?;

        Ok(grant)
    }

    /// Find user grant for a client.
    pub async fn find_user_grant(
        &self,
        user_id: Uuid,
        client_id: &str,
    ) -> Result<Option<UserOAuthGrant>, SqlxError> {
        let grant = sqlx::query_as::<_, UserOAuthGrant>(
            r#"
            SELECT * FROM user_oauth_grants
            WHERE user_id = $1 AND client_id = $2 AND revoked_at IS NULL
            "#,
        )
        .bind(user_id)
        .bind(client_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(grant)
    }

    /// List all active grants for a user with client info.
    pub async fn list_user_grants(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<UserGrantWithClientRow>, SqlxError> {
        let grants = sqlx::query_as::<_, UserGrantWithClientRow>(
            r#"
            SELECT g.id, g.client_id, c.name as client_name, c.description as client_description, g.scopes, g.granted_at
            FROM user_oauth_grants g
            JOIN oauth_clients c ON g.client_id = c.client_id
            WHERE g.user_id = $1 AND g.revoked_at IS NULL AND c.is_active = true
            ORDER BY g.granted_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(grants)
    }

    /// Revoke a user's grant for a client.
    pub async fn revoke_user_grant(
        &self,
        user_id: Uuid,
        client_id: &str,
    ) -> Result<bool, SqlxError> {
        let mut tx = self.pool.begin().await?;

        // Revoke the grant
        let result = sqlx::query(
            r#"
            UPDATE user_oauth_grants SET revoked_at = NOW()
            WHERE user_id = $1 AND client_id = $2 AND revoked_at IS NULL
            "#,
        )
        .bind(user_id)
        .bind(client_id)
        .execute(&mut *tx)
        .await?;

        if result.rows_affected() == 0 {
            return Ok(false);
        }

        // Revoke all access tokens for this user-client pair
        sqlx::query(
            r#"
            UPDATE oauth_access_tokens SET revoked_at = NOW()
            WHERE user_id = $1 AND client_id = $2 AND revoked_at IS NULL
            "#,
        )
        .bind(user_id)
        .bind(client_id)
        .execute(&mut *tx)
        .await?;

        // Revoke all refresh tokens for this user-client pair
        sqlx::query(
            r#"
            UPDATE oauth_refresh_tokens SET revoked_at = NOW()
            WHERE user_id = $1 AND client_id = $2 AND revoked_at IS NULL
            "#,
        )
        .bind(user_id)
        .bind(client_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(true)
    }

    // ==================== Cleanup ====================

    /// Cleanup expired OAuth data.
    pub async fn cleanup_expired(&self) -> Result<u64, SqlxError> {
        let mut total = 0u64;

        // Delete expired and used authorization codes
        let result = sqlx::query(
            r#"
            DELETE FROM oauth_authorization_codes
            WHERE expires_at < NOW() OR (used_at IS NOT NULL AND used_at < NOW() - INTERVAL '1 hour')
            "#,
        )
        .execute(&self.pool)
        .await?;
        total += result.rows_affected();

        // Delete expired access tokens
        let result = sqlx::query(
            r#"
            DELETE FROM oauth_access_tokens
            WHERE expires_at < NOW() OR (revoked_at IS NOT NULL AND revoked_at < NOW() - INTERVAL '7 days')
            "#,
        )
        .execute(&self.pool)
        .await?;
        total += result.rows_affected();

        // Delete expired refresh tokens
        let result = sqlx::query(
            r#"
            DELETE FROM oauth_refresh_tokens
            WHERE expires_at < NOW() OR (revoked_at IS NOT NULL AND revoked_at < NOW() - INTERVAL '7 days')
            "#,
        )
        .execute(&self.pool)
        .await?;
        total += result.rows_affected();

        Ok(total)
    }
}
