// backend/servers/deploy-server/src/auth/api_key.rs
use crate::config::ApiKey;
use sha2::{Digest, Sha256};

pub struct ApiKeyValidator {
    keys: Vec<ApiKey>,
}

impl ApiKeyValidator {
    pub fn new(keys: Vec<ApiKey>) -> Self {
        Self { keys }
    }

    pub fn validate(&self, presented: &str) -> Option<(&str, &[String])> {
        let presented_hash = hex::encode(Sha256::digest(presented.as_bytes()));
        self.keys
            .iter()
            .find(|k| k.hash == presented_hash)
            .map(|k| (k.name.as_str(), k.scopes.as_slice()))
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
