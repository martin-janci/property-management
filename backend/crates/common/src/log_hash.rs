//! Logging helpers for personally-identifiable information (PII).
//!
//! Auth flows historically logged the raw `email` field to make grep-based
//! debugging easier. That builds an enumeration log of every account that
//! has ever been probed, which leaks PII into dev/staging/prod log sinks
//! (the H2 / H3 findings of the logging-hygiene sweep).
//!
//! [`email_log_hash`] returns a short, stable hex prefix of the SHA-256
//! digest of the normalised (trim + lowercase) email. The prefix preserves
//! "this is the same email I saw a minute ago" correlation for debugging
//! without exposing the address itself.

use sha2::{Digest, Sha256};

/// Number of hex characters returned by [`email_log_hash`].
///
/// 12 hex chars = 48 bits of entropy — plenty to disambiguate a small
/// number of concurrent users in a log window, far too few to brute-force
/// back to an address out of the global email space.
pub const EMAIL_LOG_HASH_LEN: usize = 12;

/// Return a 12-char hex prefix of `SHA-256(lower(trim(email)))`.
///
/// Intended for use as a `tracing` field value:
///
/// ```ignore
/// tracing::debug!(email_hash = %common::email_log_hash(&req.email), "Login failed");
/// ```
///
/// Never embed the raw email alongside the hash — that defeats the point.
pub fn email_log_hash(email: &str) -> String {
    let normalised = email.trim().to_lowercase();
    let digest = Sha256::digest(normalised.as_bytes());
    let mut hex = String::with_capacity(EMAIL_LOG_HASH_LEN);
    for byte in digest.iter().take(EMAIL_LOG_HASH_LEN.div_ceil(2)) {
        use std::fmt::Write;
        let _ = write!(hex, "{:02x}", byte);
    }
    hex.truncate(EMAIL_LOG_HASH_LEN);
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_is_exactly_12() {
        assert_eq!(email_log_hash("user@example.com").len(), EMAIL_LOG_HASH_LEN);
    }

    #[test]
    fn normalisation_collapses_case_and_whitespace() {
        let a = email_log_hash("User@Example.com");
        let b = email_log_hash("  user@example.com  ");
        let c = email_log_hash("user@example.com");
        assert_eq!(a, b);
        assert_eq!(b, c);
    }

    #[test]
    fn distinct_emails_have_distinct_hashes() {
        let a = email_log_hash("alice@example.com");
        let b = email_log_hash("bob@example.com");
        assert_ne!(a, b);
    }
}
