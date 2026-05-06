// backend/servers/deploy-server/src/auth/oidc.rs
use crate::config::OidcConfig;
use crate::Result;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct OidcValidator {
    cfg: OidcConfig,
    jwks: Arc<RwLock<Option<JwksCache>>>,
    http: reqwest::Client,
}

#[derive(Clone)]
struct JwksCache {
    jwks: Jwks,
    fetched_at: std::time::Instant,
}

#[derive(Clone, Deserialize)]
struct Jwks {
    keys: Vec<JwkKey>,
}

#[derive(Clone, Deserialize)]
struct JwkKey {
    kid: String,
    n: String,
    e: String,
    #[allow(dead_code)]
    alg: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GhOidcClaims {
    pub sub: String,
    pub aud: String,
    pub repository: String,
    #[serde(rename = "ref")]
    pub git_ref: String,
    #[serde(default)]
    pub runner_environment: Option<String>,
}

impl OidcValidator {
    pub fn new(cfg: OidcConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("build OIDC HTTP client");
        Self {
            cfg,
            jwks: Arc::new(RwLock::new(None)),
            http,
        }
    }

    async fn fetch_jwks(&self) -> Result<Jwks> {
        let resp = self.http.get(&self.cfg.jwks_url).send().await?;
        let jwks: Jwks = resp.json().await?;
        Ok(jwks)
    }

    async fn key_for(&self, kid: &str) -> Result<DecodingKey> {
        const REFRESH_AFTER: std::time::Duration = std::time::Duration::from_secs(3600);
        // Fast path: cached + fresh + has the kid
        {
            let g = self.jwks.read().await;
            if let Some(cache) = g.as_ref() {
                if cache.fetched_at.elapsed() < REFRESH_AFTER {
                    if let Some(k) = cache.jwks.keys.iter().find(|k| k.kid == kid) {
                        return DecodingKey::from_rsa_components(&k.n, &k.e).map_err(|e| {
                            crate::DeployError::Unauthorized(format!("jwk decode: {e}"))
                        });
                    }
                }
            }
        }
        // Refresh: stale OR kid not in cache
        let fresh = self.fetch_jwks().await?;
        let key = fresh
            .keys
            .iter()
            .find(|k| k.kid == kid)
            .ok_or_else(|| crate::DeployError::Unauthorized(format!("unknown kid {kid}")))
            .and_then(|k| {
                DecodingKey::from_rsa_components(&k.n, &k.e)
                    .map_err(|e| crate::DeployError::Unauthorized(format!("jwk decode: {e}")))
            })?;
        *self.jwks.write().await = Some(JwksCache {
            jwks: fresh,
            fetched_at: std::time::Instant::now(),
        });
        Ok(key)
    }

    pub async fn validate(&self, token: &str) -> Result<GhOidcClaims> {
        let header = decode_header(token)
            .map_err(|e| crate::DeployError::Unauthorized(format!("bad header: {e}")))?;
        let kid = header
            .kid
            .ok_or_else(|| crate::DeployError::Unauthorized("missing kid".into()))?;
        let key = self.key_for(&kid).await?;

        let mut val = Validation::new(Algorithm::RS256);
        val.set_audience(&[&self.cfg.audience]);
        val.set_issuer(&[&self.cfg.issuer]);

        let data = decode::<GhOidcClaims>(token, &key, &val)
            .map_err(|e| crate::DeployError::Unauthorized(format!("jwt verify: {e}")))?;
        let claims = data.claims;

        if !self
            .cfg
            .allowed_repos
            .iter()
            .any(|r| r == &claims.repository)
        {
            return Err(crate::DeployError::Forbidden(format!(
                "repo {} not allowed",
                claims.repository
            )));
        }
        if !self
            .cfg
            .allowed_refs
            .iter()
            .any(|p| ref_matches(p, &claims.git_ref))
        {
            return Err(crate::DeployError::Forbidden(format!(
                "ref {} not allowed",
                claims.git_ref
            )));
        }
        // Reject self-hosted runners — only github-hosted runners can mint deploy tokens.
        match claims.runner_environment.as_deref() {
            Some("github-hosted") => {}
            Some(other) => {
                return Err(crate::DeployError::Forbidden(format!(
                    "runner_environment {other} not allowed (only github-hosted)"
                )));
            }
            None => {
                return Err(crate::DeployError::Forbidden(
                    "OIDC token missing runner_environment claim".into(),
                ));
            }
        }
        Ok(claims)
    }
}

fn ref_matches(pattern: &str, candidate: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix("/*") {
        // Wildcard must match exactly one segment.
        candidate
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_prefix('/'))
            .map(|seg| !seg.is_empty() && !seg.contains('/'))
            .unwrap_or(false)
    } else if let Some(prefix) = pattern.strip_suffix('*') {
        // Bare `*` (no slash) — match any chars in the same segment.
        candidate
            .strip_prefix(prefix)
            .map(|rest| !rest.contains('/'))
            .unwrap_or(false)
    } else {
        pattern == candidate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ref_pattern_match() {
        assert!(ref_matches(
            "refs/heads/feature/*",
            "refs/heads/feature/foo"
        ));
        assert!(ref_matches("refs/heads/main", "refs/heads/main"));
        assert!(!ref_matches("refs/heads/main", "refs/heads/dev"));
        // segment boundary
        assert!(!ref_matches(
            "refs/heads/feature/*",
            "refs/heads/feature/foo/bar"
        ));
        assert!(!ref_matches("refs/heads/feature/*", "refs/heads/feature/"));
        assert!(!ref_matches(
            "refs/heads/feature/*",
            "refs/heads/feature/../main"
        ));
    }
}
