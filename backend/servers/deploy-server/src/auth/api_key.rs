// backend/servers/deploy-server/src/auth/api_key.rs
use crate::config::ApiKey;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

pub struct ApiKeyValidator {
    keys: Vec<ApiKey>,
}

impl ApiKeyValidator {
    pub fn new(keys: Vec<ApiKey>) -> Self {
        Self { keys }
    }

    /// Validate a presented bearer token against the configured API keys.
    ///
    /// The presented token is hashed (SHA-256) and the resulting hex digest is
    /// compared against each stored hash using a constant-time comparison
    /// (`subtle::ConstantTimeEq`). The constant-time path matters because the
    /// hex digests are derived from attacker-controlled input that arrives over
    /// the network: a naive `==` on `String` short-circuits on the first
    /// differing byte and leaks per-byte timing, which a determined attacker
    /// could exploit to recover the stored hash one byte at a time.
    ///
    /// All configured keys are scanned (no early break on first hit) so the
    /// total time taken is independent of which key matches or how many keys
    /// are configured up to N.
    pub fn validate(&self, presented: &str) -> Option<(&str, &[String])> {
        let presented_hash = hex::encode(Sha256::digest(presented.as_bytes()));
        let presented_bytes = presented_hash.as_bytes();

        let mut matched: Option<&ApiKey> = None;
        for k in &self.keys {
            // ConstantTimeEq returns Choice (0 or 1). We OR-accumulate the
            // matched index without short-circuiting so timing doesn't depend
            // on which key matched.
            if k.hash.as_bytes().ct_eq(presented_bytes).into() {
                matched = Some(k);
                // Don't break — keep iterating to keep total time stable.
            }
        }
        matched.map(|k| (k.name.as_str(), k.scopes.as_slice()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    #[test]
    fn matching_hash_returns_name() {
        let key = "secret-token-abc";
        let hash = hex::encode(Sha256::digest(key.as_bytes()));
        let v = ApiKeyValidator::new(vec![ApiKey {
            name: "claude-skill".into(),
            hash,
            scopes: vec![],
        }]);
        let (name, scopes) = v.validate(key).unwrap();
        assert_eq!(name, "claude-skill");
        assert!(scopes.is_empty());
    }

    #[test]
    fn wrong_token_rejected() {
        let v = ApiKeyValidator::new(vec![ApiKey {
            name: "x".into(),
            hash: "deadbeef".into(),
            scopes: vec![],
        }]);
        assert!(v.validate("nope").is_none());
    }

    #[test]
    fn validate_returns_scopes() {
        let key = "scoped-token";
        let hash = hex::encode(Sha256::digest(key.as_bytes()));
        let v = ApiKeyValidator::new(vec![ApiKey {
            name: "gc-cron".into(),
            hash,
            scopes: vec!["gc:tick".into()],
        }]);
        let (name, scopes) = v.validate(key).unwrap();
        assert_eq!(name, "gc-cron");
        assert_eq!(scopes, &["gc:tick".to_string()]);
    }
}
