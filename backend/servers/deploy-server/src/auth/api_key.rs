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

    pub fn validate(&self, presented: &str) -> Option<&str> {
        let presented_hash = hex::encode(Sha256::digest(presented.as_bytes()));
        self.keys
            .iter()
            .find(|k| k.hash == presented_hash)
            .map(|k| k.name.as_str())
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
        }]);
        assert_eq!(v.validate(key), Some("claude-skill"));
    }

    #[test]
    fn wrong_token_rejected() {
        let v = ApiKeyValidator::new(vec![ApiKey {
            name: "x".into(),
            hash: "deadbeef".into(),
        }]);
        assert!(v.validate("nope").is_none());
    }
}
