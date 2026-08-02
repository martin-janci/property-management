//! Generic Idempotency-Key middleware helpers.
//!
//! The helper below implements the reusable part of Story 2B.7:
//!   * tenant-scoped key ledger in PostgreSQL,
//!   * 24h lazy expiry,
//!   * request-hash mismatch rejection (422),
//!   * advisory-lock serialization for concurrent in-flight duplicates,
//!   * replay of the original response body/status for duplicate requests.

use axum::{
    body::{to_bytes, Body},
    http::{header::CONTENT_TYPE, HeaderMap, HeaderValue, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use common::errors::ErrorResponse;
use db::DbPool;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::FromRow;

use super::host_tenant::ResolvedTenant;

const IDEMPOTENCY_TTL_HOURS: i64 = 24;
const MAX_CAPTURED_BODY_BYTES: usize = 16 * 1024 * 1024;
const REPLAYED_HEADER: &str = "x-idempotency-replayed";

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
struct CachedResponseRow {
    request_hash: String,
    response_status: i32,
    response_content_type: Option<String>,
    response_body: Vec<u8>,
}

/// Execute `next` behind the generic idempotency ledger.
///
/// Callers typically wrap this in a route-local Axum middleware that extracts
/// the app state and passes `state.db.clone()` here.
pub async fn handle_idempotent_request(
    pool: DbPool,
    request: Request<Body>,
    next: Next,
) -> Response {
    let Some(key) = request
        .headers()
        .get("Idempotency-Key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
    else {
        return next.run(request).await;
    };

    let method = request.method().as_str().to_owned();
    let path = request
        .uri()
        .path_and_query()
        .map(|value| value.as_str().to_owned())
        .unwrap_or_else(|| request.uri().path().to_owned());
    let tenant_scope = tenant_scope(&request);

    let (parts, body) = request.into_parts();
    let request_body = match to_bytes(body, MAX_CAPTURED_BODY_BYTES).await {
        Ok(bytes) => bytes,
        Err(err) => {
            tracing::warn!(error = %err, "failed to read request body for idempotency");
            return error_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "REQUEST_BODY_TOO_LARGE",
                "Request body exceeds the idempotency capture limit",
            );
        }
    };
    let request_hash = request_hash(&method, &path, &request_body);
    let request = Request::from_parts(parts, Body::from(request_body.clone()));

    let mut conn = match pool.acquire().await {
        Ok(conn) => conn,
        Err(err) => {
            tracing::error!(error = %err, "failed to acquire idempotency connection");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "Failed to initialize idempotency guard",
            );
        }
    };

    let lock_id = advisory_lock_id(&tenant_scope, &method, &path, &key);
    if let Err(err) = sqlx::query("SELECT pg_advisory_lock($1)")
        .bind(lock_id)
        .execute(&mut *conn)
        .await
    {
        tracing::error!(error = %err, "failed to acquire idempotency advisory lock");
        return error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_ERROR",
            "Failed to initialize idempotency guard",
        );
    }

    let cached = match load_cached_response(&mut conn, &tenant_scope, &method, &path, &key).await {
        Ok(row) => row,
        Err(err) => {
            let _ = unlock(&mut conn, lock_id).await;
            tracing::error!(error = %err, "failed to load cached idempotency response");
            return error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                "Failed to initialize idempotency guard",
            );
        }
    };

    if let Some(row) = cached {
        if row.request_hash != request_hash {
            let _ = unlock(&mut conn, lock_id).await;
            return error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                "IDEMPOTENCY_KEY_REUSED",
                "Idempotency-Key was already used with a different request payload",
            );
        }

        let _ = unlock(&mut conn, lock_id).await;
        return cached_response(row);
    }

    let response = next.run(request).await;
    let response = match capture_response(response).await {
        Ok(captured) => {
            if !captured.status.is_server_error() {
                if let Err(err) = store_response(
                    &mut conn,
                    &tenant_scope,
                    &method,
                    &path,
                    &key,
                    &request_hash,
                    &captured,
                )
                .await
                {
                    tracing::error!(error = %err, "failed to store idempotent response");
                    let _ = unlock(&mut conn, lock_id).await;
                    return error_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "INTERNAL_ERROR",
                        "Failed to persist idempotent response",
                    );
                }
            }

            captured.into_response()
        }
        Err(response) => response,
    };

    let _ = unlock(&mut conn, lock_id).await;
    response
}

/// Derive the idempotency cache-scope tenant.
///
/// SECURITY: the scope MUST come from the server-side [`ResolvedTenant`]
/// injected by `host_tenant_middleware` — the exact same source RLS/tenant
/// extraction uses — and NEVER from a client-supplied header. The middleware
/// previously fell back to the raw `X-Tenant-ID` request header when no
/// `ResolvedTenant` was present; a caller could set that header to another
/// organization's id and land in its cache scope, enabling cross-tenant
/// idempotency-key collision (poisoning another tenant's replay) or bypass.
/// This function no longer reads any request header.
fn tenant_scope(request: &Request<Body>) -> String {
    match request.extensions().get::<ResolvedTenant>() {
        Some(resolved) if resolved.is_platform_host() => "platform-host".to_string(),
        Some(resolved) => resolved.organization_id.to_string(),
        // No resolved tenant means the request reached us without
        // `host_tenant_middleware` running (e.g. a public/allowlisted path).
        // Fall back to a fixed server-side sentinel that no client input can
        // influence — deliberately NOT the `X-Tenant-ID` header.
        None => "global".to_string(),
    }
}

fn request_hash(method: &str, path: &str, body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(method.as_bytes());
    hasher.update(b"\n");
    hasher.update(path.as_bytes());
    hasher.update(b"\n");
    hasher.update(body);
    hex::encode(hasher.finalize())
}

fn advisory_lock_id(tenant_scope: &str, method: &str, path: &str, key: &str) -> i64 {
    let mut hasher = Sha256::new();
    hasher.update(tenant_scope.as_bytes());
    hasher.update(b"\n");
    hasher.update(method.as_bytes());
    hasher.update(b"\n");
    hasher.update(path.as_bytes());
    hasher.update(b"\n");
    hasher.update(key.as_bytes());

    let digest = hasher.finalize();
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    i64::from_be_bytes(bytes)
}

async fn unlock(
    conn: &mut sqlx::pool::PoolConnection<sqlx::Postgres>,
    lock_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_advisory_unlock($1)")
        .bind(lock_id)
        .execute(&mut **conn)
        .await?;
    Ok(())
}

async fn load_cached_response(
    conn: &mut sqlx::pool::PoolConnection<sqlx::Postgres>,
    tenant_scope: &str,
    method: &str,
    path: &str,
    key: &str,
) -> Result<Option<CachedResponseRow>, sqlx::Error> {
    sqlx::query(
        r#"
        DELETE FROM idempotency_keys
        WHERE tenant_scope = $1
          AND request_method = $2
          AND request_path = $3
          AND idempotency_key = $4
          AND expires_at <= NOW()
        "#,
    )
    .bind(tenant_scope)
    .bind(method)
    .bind(path)
    .bind(key)
    .execute(&mut **conn)
    .await?;

    sqlx::query_as::<_, CachedResponseRow>(
        r#"
        SELECT request_hash, response_status, response_content_type, response_body
        FROM idempotency_keys
        WHERE tenant_scope = $1
          AND request_method = $2
          AND request_path = $3
          AND idempotency_key = $4
          AND expires_at > NOW()
        "#,
    )
    .bind(tenant_scope)
    .bind(method)
    .bind(path)
    .bind(key)
    .fetch_optional(&mut **conn)
    .await
}

struct CapturedResponse {
    headers: HeaderMap,
    status: StatusCode,
    content_type: Option<String>,
    body: Vec<u8>,
}

impl CapturedResponse {
    fn into_response(self) -> Response {
        let mut response = Response::new(Body::from(self.body));
        *response.status_mut() = self.status;
        *response.headers_mut() = self.headers;
        response
    }
}

async fn capture_response(response: Response) -> Result<CapturedResponse, Response> {
    let status = response.status();
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let (parts, body) = response.into_parts();
    let headers = parts.headers.clone();
    let body = match to_bytes(body, MAX_CAPTURED_BODY_BYTES).await {
        Ok(bytes) => bytes.to_vec(),
        Err(err) => {
            tracing::warn!(error = %err, "failed to capture response body for idempotency");
            return Err(Response::from_parts(parts, Body::empty()));
        }
    };

    Ok(CapturedResponse {
        headers,
        status,
        content_type,
        body,
    })
}

async fn store_response(
    conn: &mut sqlx::pool::PoolConnection<sqlx::Postgres>,
    tenant_scope: &str,
    method: &str,
    path: &str,
    key: &str,
    request_hash: &str,
    response: &CapturedResponse,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO idempotency_keys (
            tenant_scope,
            request_method,
            request_path,
            idempotency_key,
            request_hash,
            response_status,
            response_content_type,
            response_body,
            expires_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW() + ($9 * INTERVAL '1 hour'))
        ON CONFLICT (tenant_scope, request_method, request_path, idempotency_key)
        DO UPDATE SET
            request_hash = EXCLUDED.request_hash,
            response_status = EXCLUDED.response_status,
            response_content_type = EXCLUDED.response_content_type,
            response_body = EXCLUDED.response_body,
            updated_at = NOW(),
            expires_at = EXCLUDED.expires_at
        "#,
    )
    .bind(tenant_scope)
    .bind(method)
    .bind(path)
    .bind(key)
    .bind(request_hash)
    .bind(i32::from(response.status.as_u16()))
    .bind(&response.content_type)
    .bind(&response.body)
    .bind(IDEMPOTENCY_TTL_HOURS)
    .execute(&mut **conn)
    .await?;

    Ok(())
}

fn cached_response(row: CachedResponseRow) -> Response {
    let mut response = Response::new(Body::from(row.response_body));
    *response.status_mut() = StatusCode::from_u16(row.response_status as u16)
        .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    if let Some(content_type) = row.response_content_type {
        if let Ok(value) = HeaderValue::from_str(&content_type) {
            response.headers_mut().insert(CONTENT_TYPE, value);
        }
    }
    response
        .headers_mut()
        .insert(REPLAYED_HEADER, HeaderValue::from_static("true"));
    response
}

fn error_response(status: StatusCode, code: &str, message: &str) -> Response {
    (status, Json(ErrorResponse::new(code, message))).into_response()
}
