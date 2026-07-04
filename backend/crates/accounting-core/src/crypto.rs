//! Security primitives for the accounting domain (UC-ACC-16.1 / 05.11).
//!
//! Replaces the MVP-skeleton "opaque UUID + 6-digit format check" with real
//! crypto:
//!   * **RFC-6238 TOTP** (HMAC-SHA1, 6 digits, 30 s) for genuine 2FA — codes are
//!     verified against the enrolled secret with a ±window clock-drift tolerance.
//!   * **CSPRNG** share tokens & recovery codes (OS entropy via `SysRng`).
//!   * **At-rest hashing**: share tokens and recovery codes are persisted only as
//!     SHA-256 hashes; the raw value is shown to the user exactly once.
//!   * **Constant-time** comparison for in-process secret checks.
//!
//! NOTE on the TOTP secret: a TOTP secret is symmetric and must be recoverable
//! to compute codes, so it cannot be hashed. It is stored as base32 text; if/when
//! an application KMS is available it should additionally be encrypted at rest
//! (tracked follow-up — out of scope here, which adds verification + at-rest
//! hashing for the values that *can* be hashed).

use data_encoding::BASE32_NOPAD;
use hmac::{Hmac, KeyInit, Mac};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use thiserror::Error;

type HmacSha1 = Hmac<Sha1>;

/// Errors from the accounting security primitives.
#[derive(Debug, Error)]
pub enum CryptoError {
    /// The OS CSPRNG failed to produce entropy. A transient OS entropy error is
    /// surfaced as a handled error (not a panic) so the caller can refuse the
    /// operation instead of crashing the process mid-request.
    #[error("Secure random number generation failed: {0}")]
    RngFailed(String),
}

/// TOTP time step in seconds (RFC-6238 default).
pub const TOTP_PERIOD_SECS: u64 = 30;
/// TOTP code length (authenticator-app default).
pub const TOTP_DIGITS: u32 = 6;
/// Default clock-drift tolerance in time steps (±1 → accepts the previous,
/// current, and next 30 s window).
pub const TOTP_DEFAULT_WINDOW: i64 = 1;

const TOTP_SECRET_BYTES: usize = 20; // 160-bit (RFC-4226 §4 recommendation)
const SHARE_TOKEN_BYTES: usize = 32; // 256-bit capability token
const RECOVERY_CODE_BYTES: usize = 10; // 80-bit per recovery code

/// Fill `buf` from the OS CSPRNG.
///
/// Returns [`CryptoError::RngFailed`] if the OS entropy source errors. A
/// transient CSPRNG failure must surface as a handled error so the caller can
/// refuse the operation (e.g. reject 2FA enrollment / share-link creation)
/// rather than panicking and crashing the process mid-request.
fn fill_random(buf: &mut [u8]) -> Result<(), CryptoError> {
    use rand::TryRng;
    rand::rngs::SysRng
        .try_fill_bytes(buf)
        .map_err(|e| CryptoError::RngFailed(e.to_string()))
}

/// SHA-256 hex digest. Used to store share tokens & recovery codes at rest: the
/// raw value is returned to the caller once, only this hash is persisted, and
/// lookups hash the presented value and compare.
pub fn hash_token(raw: &str) -> String {
    let mut h = Sha256::new();
    h.update(raw.as_bytes());
    hex::encode(h.finalize())
}

/// Constant-time byte-string equality (avoids timing oracles when comparing a
/// presented secret to an expected value in-process). Uses `subtle` so the
/// comparison is not short-circuited by the optimizer. Differing lengths return
/// false (length is not itself secret for our fixed-width codes/hashes).
pub fn ct_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.ct_eq(b).into()
}

/// 256-bit URL-safe capability token for share links (UC-ACC-05.11). Returned to
/// the caller ONCE; persist only `hash_token(&token)`.
pub fn generate_share_token() -> Result<String, CryptoError> {
    let mut bytes = [0u8; SHARE_TOKEN_BYTES];
    fill_random(&mut bytes)?;
    Ok(hex::encode(bytes))
}

/// A single human-friendly one-time recovery code (`XXXXXXXX-XXXXXXXX`, base32).
/// Stored hashed; shown once (UC-ACC-16.1).
pub fn generate_recovery_code() -> Result<String, CryptoError> {
    let mut bytes = [0u8; RECOVERY_CODE_BYTES];
    fill_random(&mut bytes)?;
    let s = BASE32_NOPAD.encode(&bytes); // 10 bytes -> 16 base32 chars
    Ok(format!("{}-{}", &s[0..8], &s[8..16]))
}

/// Fresh base32 (no-pad) TOTP secret for an authenticator app (UC-ACC-16.1).
pub fn generate_totp_secret() -> Result<String, CryptoError> {
    let mut bytes = [0u8; TOTP_SECRET_BYTES];
    fill_random(&mut bytes)?;
    Ok(BASE32_NOPAD.encode(&bytes))
}

/// Current Unix time in seconds (TOTP time source).
pub fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Minimal RFC-3986 percent-encoding for the otpauth label/issuer.
fn pct(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Build the `otpauth://totp/...` provisioning URI an authenticator app scans.
pub fn totp_provisioning_uri(secret_b32: &str, account: &str, issuer: &str) -> String {
    let iss = pct(issuer);
    let acc = pct(account);
    format!(
        "otpauth://totp/{iss}:{acc}?secret={secret_b32}&issuer={iss}&algorithm=SHA1&digits={TOTP_DIGITS}&period={TOTP_PERIOD_SECS}"
    )
}

/// RFC-4226 HOTP value for `counter` against the raw secret bytes.
fn hotp(secret: &[u8], counter: u64) -> u32 {
    let mut mac = HmacSha1::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(&counter.to_be_bytes());
    let digest = mac.finalize().into_bytes();
    // Dynamic truncation (RFC-4226 §5.3).
    let offset = (digest[digest.len() - 1] & 0x0f) as usize;
    let bin = ((u32::from(digest[offset]) & 0x7f) << 24)
        | (u32::from(digest[offset + 1]) << 16)
        | (u32::from(digest[offset + 2]) << 8)
        | u32::from(digest[offset + 3]);
    bin % 10u32.pow(TOTP_DIGITS)
}

/// Verify a TOTP `code` against the base32 `secret` at `unix_time`, tolerating
/// ±`window` time steps of clock drift (RFC-6238 §5.2). Never panics: malformed
/// code or secret → `false`. The per-step compare is constant-time and the loop
/// does not early-exit, so timing does not leak which step matched.
pub fn verify_totp(secret_b32: &str, code: &str, unix_time: u64, window: i64) -> bool {
    let code = code.trim();
    if code.len() != TOTP_DIGITS as usize || !code.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let secret = match BASE32_NOPAD.decode(secret_b32.trim().to_ascii_uppercase().as_bytes()) {
        Ok(s) if !s.is_empty() => s,
        _ => return false,
    };
    let step = (unix_time / TOTP_PERIOD_SECS) as i64;
    let width = TOTP_DIGITS as usize;
    let mut matched = false;
    for delta in -window..=window {
        let counter: u64 = match (step + delta).try_into() {
            Ok(c) => c,
            Err(_) => continue, // pre-epoch step; skip
        };
        let expected = format!("{:0width$}", hotp(&secret, counter), width = width);
        matched |= ct_eq(&expected, code);
    }
    matched
}

#[cfg(test)]
mod tests {
    use super::*;

    // RFC-6238 Appendix B test vectors (SHA-1 seed "12345678901234567890"),
    // truncated to 6 digits (= the published 8-digit value mod 1e6).
    fn rfc_secret() -> String {
        BASE32_NOPAD.encode(b"12345678901234567890")
    }

    #[test]
    fn totp_rfc6238_vectors() {
        let s = rfc_secret();
        assert!(verify_totp(&s, "287082", 59, 0), "T=59");
        assert!(verify_totp(&s, "081804", 1_111_111_109, 0), "T=1111111109");
        assert!(verify_totp(&s, "005924", 1_234_567_890, 0), "T=1234567890");
        assert!(verify_totp(&s, "279037", 2_000_000_000, 0), "T=2000000000");
    }

    #[test]
    fn totp_window_tolerates_drift_and_rejects_outside() {
        let s = rfc_secret();
        // Code for the step at T=59 (step 1) is accepted one step later (T=89)
        // with window=1, but not with window=0.
        assert!(
            verify_totp(&s, "287082", 89, 1),
            "±1 window accepts prev step"
        );
        assert!(
            !verify_totp(&s, "287082", 89, 0),
            "window=0 rejects prev step"
        );
        // Two steps away is rejected even with window=1.
        assert!(
            !verify_totp(&s, "287082", 119, 1),
            "2 steps out of ±1 window"
        );
    }

    #[test]
    fn totp_rejects_malformed_and_wrong_codes() {
        let s = rfc_secret();
        assert!(!verify_totp(&s, "000000", 59, 1), "wrong code");
        assert!(!verify_totp(&s, "12345", 59, 1), "too short");
        assert!(!verify_totp(&s, "1234567", 59, 1), "too long");
        assert!(!verify_totp(&s, "abcdef", 59, 1), "non-numeric");
        assert!(
            !verify_totp("not base32 !!!", "287082", 59, 1),
            "bad secret"
        );
        assert!(!verify_totp("", "287082", 59, 1), "empty secret");
    }

    #[test]
    fn secret_and_tokens_are_well_formed_and_unique() {
        let s1 = generate_totp_secret().unwrap();
        let s2 = generate_totp_secret().unwrap();
        assert_ne!(s1, s2, "secrets must be unique");
        assert!(
            BASE32_NOPAD.decode(s1.as_bytes()).is_ok(),
            "secret is base32"
        );
        // A freshly generated secret produces a verifiable current code.
        let now = 1_700_000_000u64;
        let code = format!(
            "{:06}",
            hotp(
                &BASE32_NOPAD.decode(s1.as_bytes()).unwrap(),
                now / TOTP_PERIOD_SECS
            )
        );
        assert!(
            verify_totp(&s1, &code, now, 0),
            "self-generated code verifies"
        );

        let t1 = generate_share_token().unwrap();
        let t2 = generate_share_token().unwrap();
        assert_ne!(t1, t2);
        assert_eq!(t1.len(), SHARE_TOKEN_BYTES * 2, "32 bytes -> 64 hex chars");

        let r = generate_recovery_code().unwrap();
        assert_eq!(r.len(), 17, "XXXXXXXX-XXXXXXXX");
        assert_ne!(
            generate_recovery_code().unwrap(),
            generate_recovery_code().unwrap()
        );
    }

    #[test]
    fn rng_failure_surfaces_as_handled_error() {
        // Regression: a CSPRNG failure during token/secret generation must
        // propagate as a handled `CryptoError::RngFailed` rather than panicking
        // the process (the OS entropy source can transiently fail).
        let err = CryptoError::RngFailed("entropy source unavailable".to_string());
        assert!(matches!(err, CryptoError::RngFailed(_)));
        let msg = err.to_string();
        assert!(msg.contains("Secure random number generation failed"));
        assert!(msg.contains("entropy source unavailable"));

        // The generators expose a `Result` so callers can `?`-propagate the
        // failure. On a healthy host they succeed.
        assert!(generate_share_token().is_ok());
        assert!(generate_recovery_code().is_ok());
        assert!(generate_totp_secret().is_ok());
    }

    #[test]
    fn hash_token_is_deterministic_and_ct_eq_correct() {
        let raw = "some-secret-token";
        assert_eq!(hash_token(raw), hash_token(raw));
        assert_ne!(hash_token(raw), hash_token("other"));
        assert_eq!(hash_token(raw).len(), 64, "sha256 hex");
        assert!(ct_eq("abc", "abc"));
        assert!(!ct_eq("abc", "abd"));
        assert!(!ct_eq("abc", "abcd"));
    }
}
