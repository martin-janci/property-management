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
    jwks: Arc<RwLock<Option<Jwks>>>,
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
}

impl OidcValidator {
    pub fn new(cfg: OidcConfig) -> Self {
        Self {
            cfg,
            jwks: Arc::new(RwLock::new(None)),
        }
    }

    async fn fetch_jwks(&self) -> Result<Jwks> {
        let resp = reqwest::get(&self.cfg.jwks_url).await?;
        let jwks: Jwks = resp.json().await?;
        Ok(jwks)
    }

    async fn key_for(&self, kid: &str) -> Result<DecodingKey> {
        {
            let g = self.jwks.read().await;
            if let Some(j) = g.as_ref() {
                if let Some(k) = j.keys.iter().find(|k| k.kid == kid) {
                    return DecodingKey::from_rsa_components(&k.n, &k.e)
                        .map_err(|e| crate::DeployError::Unauthorized(format!("jwk decode: {e}")));
                }
            }
        }
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
        *self.jwks.write().await = Some(fresh);
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
        Ok(claims)
    }
}

fn ref_matches(pattern: &str, candidate: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix('*') {
        candidate.starts_with(prefix)
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
    }
}
