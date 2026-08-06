//! URL validation helpers for SSRF defense.
//!
//! Used by handlers that accept user-supplied URLs which will later be
//! fetched server-side (e.g. agency import feeds). The validator rejects:
//!
//! * non-`http`/`https` schemes (including `file://`, `data:`, `javascript:`,
//!   `gopher://`, etc.),
//! * `http://` outside development builds,
//! * literal IPs in private, loopback, link-local, multicast, or reserved
//!   ranges (IPv4 + IPv6),
//! * the IPv4 broadcast address.
//!
//! Hostnames (domains) pass this check; the import worker MUST re-resolve and
//! re-validate the resolved IP at fetch time to defend against DNS-rebinding
//! attacks. See the worker maintainer note in `routes::agency_imports`.
//!
//! This module is deliberately small so the same helper can be reused by
//! other handlers that accept user-supplied URLs for server-side fetches
//! (e.g. logo / profile-image URLs — fix in a follow-up branch).

use url::{Host, Url};

/// Outcome of a URL validation check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrlValidationError {
    /// URL failed to parse as a `url::Url`.
    InvalidFormat(String),
    /// Scheme is not in the allowlist.
    DisallowedScheme(String),
    /// `http://` was used in a production build.
    PlainHttpNotAllowed,
    /// URL has no host component (e.g. `file:///etc/passwd`).
    MissingHost,
    /// Host is a literal IP in a forbidden range.
    PrivateOrReservedIp(String),
}

impl std::fmt::Display for UrlValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFormat(e) => write!(f, "Invalid URL format: {}", e),
            Self::DisallowedScheme(s) => write!(f, "URL scheme '{}' is not allowed", s),
            Self::PlainHttpNotAllowed => {
                write!(f, "Plain http:// URLs are not allowed; use https://")
            }
            Self::MissingHost => write!(f, "URL must have a host"),
            Self::PrivateOrReservedIp(reason) => {
                write!(f, "URL points to a forbidden network range: {}", reason)
            }
        }
    }
}

impl std::error::Error for UrlValidationError {}

/// Returns true when the process is running in a development build.
///
/// Matches the convention used by `state::AppConfig::from_env` — `RUST_ENV`
/// must be `"development"` to relax production-only checks.
fn is_development() -> bool {
    std::env::var("RUST_ENV").unwrap_or_default() == "development"
}

/// Returns `Some(reason)` when an IPv4 address falls in a private, loopback,
/// link-local, reserved, broadcast, or multicast range that must not be
/// fetched server-side; `None` when the address is publicly routable.
///
/// Shared by the [`Host::Ipv4`] arm and the [`Host::Ipv6`] arm (after
/// unmapping IPv4-mapped / IPv4-compatible addresses) so the same policy
/// can't be bypassed by re-encoding a forbidden IPv4 address into IPv6.
fn ipv4_forbidden_reason(octets: [u8; 4]) -> Option<&'static str> {
    // 10.0.0.0/8 — private
    if octets[0] == 10 {
        return Some("10.0.0.0/8 (private)");
    }
    // 172.16.0.0/12 — private
    if octets[0] == 172 && (16..=31).contains(&octets[1]) {
        return Some("172.16.0.0/12 (private)");
    }
    // 192.168.0.0/16 — private
    if octets[0] == 192 && octets[1] == 168 {
        return Some("192.168.0.0/16 (private)");
    }
    // 127.0.0.0/8 — loopback
    if octets[0] == 127 {
        return Some("127.0.0.0/8 (loopback)");
    }
    // 169.254.0.0/16 — link-local (covers AWS/GCP metadata 169.254.169.254)
    if octets[0] == 169 && octets[1] == 254 {
        return Some("169.254.0.0/16 (link-local / cloud metadata)");
    }
    // 0.0.0.0/8 — reserved
    if octets[0] == 0 {
        return Some("0.0.0.0/8 (reserved)");
    }
    // 255.255.255.255 — broadcast
    if octets == [255, 255, 255, 255] {
        return Some("255.255.255.255 (broadcast)");
    }
    // 224.0.0.0/4 — multicast
    if octets[0] >= 224 && octets[0] <= 239 {
        return Some("224.0.0.0/4 (multicast)");
    }
    None
}

/// Validate that `raw` is a safe URL to fetch server-side.
///
/// On success returns the parsed [`Url`]. On failure returns a structured
/// [`UrlValidationError`] suitable for surfacing as a 400 Bad Request.
///
/// **Important**: this is a *static* check on the supplied string. The
/// caller MUST repeat the IP-range check after DNS resolution at fetch
/// time, otherwise an attacker can register a hostname that resolves to a
/// private IP (or use DNS rebinding to flip the answer between checks).
pub fn validate_fetch_url(raw: &str) -> Result<Url, UrlValidationError> {
    let parsed = Url::parse(raw).map_err(|e| UrlValidationError::InvalidFormat(e.to_string()))?;

    // Scheme allowlist: only http(s).
    let scheme = parsed.scheme();
    match scheme {
        "https" => {}
        "http" => {
            if !is_development() {
                return Err(UrlValidationError::PlainHttpNotAllowed);
            }
        }
        other => return Err(UrlValidationError::DisallowedScheme(other.to_string())),
    }

    // Must have a host (rules out `file:///path`, `data:...`, etc., though
    // those would already trip the scheme check — defense in depth).
    let host = parsed.host().ok_or(UrlValidationError::MissingHost)?;

    match host {
        Host::Ipv4(ip) => {
            if let Some(reason) = ipv4_forbidden_reason(ip.octets()) {
                return Err(UrlValidationError::PrivateOrReservedIp(reason.into()));
            }
        }
        Host::Ipv6(ip) => {
            if ip.is_loopback() {
                return Err(UrlValidationError::PrivateOrReservedIp(
                    "::1 (IPv6 loopback)".into(),
                ));
            }
            // IPv4-mapped (`::ffff:a.b.c.d`) and IPv4-compatible (`::a.b.c.d`)
            // addresses embed a real IPv4 destination. Unmap and run the IPv4
            // policy, otherwise the guard can be bypassed by re-encoding a
            // forbidden IPv4 address (e.g. `::ffff:169.254.169.254`,
            // `::ffff:127.0.0.1`) into IPv6.
            if let Some(v4) = ip.to_ipv4() {
                if let Some(reason) = ipv4_forbidden_reason(v4.octets()) {
                    return Err(UrlValidationError::PrivateOrReservedIp(format!(
                        "IPv4-mapped IPv6 -> {}",
                        reason
                    )));
                }
            }
            let segments = ip.segments();
            // fc00::/7 — IPv6 unique local
            if (segments[0] & 0xfe00) == 0xfc00 {
                return Err(UrlValidationError::PrivateOrReservedIp(
                    "fc00::/7 (IPv6 unique local)".into(),
                ));
            }
            // fe80::/10 — IPv6 link-local
            if (segments[0] & 0xffc0) == 0xfe80 {
                return Err(UrlValidationError::PrivateOrReservedIp(
                    "fe80::/10 (IPv6 link-local)".into(),
                ));
            }
        }
        Host::Domain(_) => {
            // Hostnames must be re-validated at fetch time by the worker
            // after DNS resolution. We can't do it here without performing
            // (cacheable, race-prone) DNS lookups inside the request path.
        }
    }

    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::env_lock;

    /// RAII guard that pins `RUST_ENV` to a given value for the lifetime of a
    /// single test, then restores the previous value on drop. Acquires the
    /// process-wide [`env_lock`] so concurrent tests in this crate can't race
    /// on the same env var.
    struct EnvGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
        previous: Option<String>,
    }

    impl EnvGuard {
        fn set(value: &str) -> Self {
            let lock = env_lock();
            let previous = std::env::var("RUST_ENV").ok();
            std::env::set_var("RUST_ENV", value);
            Self {
                _lock: lock,
                previous,
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match self.previous.take() {
                Some(v) => std::env::set_var("RUST_ENV", v),
                None => std::env::remove_var("RUST_ENV"),
            }
        }
    }

    #[test]
    fn accepts_https_domain() {
        let _g = EnvGuard::set("production");
        assert!(validate_fetch_url("https://example.com/feed.xml").is_ok());
    }

    #[test]
    fn rejects_http_in_prod() {
        let _g = EnvGuard::set("production");
        assert!(matches!(
            validate_fetch_url("http://example.com/feed.xml"),
            Err(UrlValidationError::PlainHttpNotAllowed)
        ));
    }

    #[test]
    fn allows_http_in_dev() {
        let _g = EnvGuard::set("development");
        assert!(validate_fetch_url("http://example.com/feed.xml").is_ok());
    }

    #[test]
    fn rejects_file_scheme() {
        let _g = EnvGuard::set("production");
        assert!(matches!(
            validate_fetch_url("file:///etc/passwd"),
            Err(UrlValidationError::DisallowedScheme(_))
        ));
    }

    #[test]
    fn rejects_data_scheme() {
        let _g = EnvGuard::set("production");
        assert!(matches!(
            validate_fetch_url("data:text/plain,hello"),
            Err(UrlValidationError::DisallowedScheme(_))
        ));
    }

    #[test]
    fn rejects_javascript_scheme() {
        let _g = EnvGuard::set("production");
        assert!(matches!(
            validate_fetch_url("javascript:alert(1)"),
            Err(UrlValidationError::DisallowedScheme(_))
        ));
    }

    #[test]
    fn rejects_aws_metadata_ip() {
        {
            let _g = EnvGuard::set("production");
            assert!(matches!(
                validate_fetch_url("http://169.254.169.254/latest/meta-data/"),
                // PlainHttp check trips first in prod; in dev the IP rule fires.
                Err(UrlValidationError::PlainHttpNotAllowed)
                    | Err(UrlValidationError::PrivateOrReservedIp(_))
            ));
        }
        let _g = EnvGuard::set("development");
        assert!(matches!(
            validate_fetch_url("http://169.254.169.254/latest/meta-data/"),
            Err(UrlValidationError::PrivateOrReservedIp(_))
        ));
    }

    #[test]
    fn rejects_https_loopback() {
        let _g = EnvGuard::set("production");
        assert!(matches!(
            validate_fetch_url("https://127.0.0.1/"),
            Err(UrlValidationError::PrivateOrReservedIp(_))
        ));
    }

    #[test]
    fn rejects_https_private_10() {
        let _g = EnvGuard::set("production");
        assert!(matches!(
            validate_fetch_url("https://10.0.0.5/feed.xml"),
            Err(UrlValidationError::PrivateOrReservedIp(_))
        ));
    }

    #[test]
    fn rejects_ipv6_loopback() {
        let _g = EnvGuard::set("production");
        assert!(matches!(
            validate_fetch_url("https://[::1]/"),
            Err(UrlValidationError::PrivateOrReservedIp(_))
        ));
    }

    #[test]
    fn rejects_ipv4_mapped_metadata_ip() {
        // `::ffff:169.254.169.254` must be unmapped and rejected as cloud
        // metadata link-local, not slipped through the IPv6 branch.
        let _g = EnvGuard::set("production");
        assert!(matches!(
            validate_fetch_url("https://[::ffff:169.254.169.254]/latest/meta-data/"),
            Err(UrlValidationError::PrivateOrReservedIp(_))
        ));
    }

    #[test]
    fn rejects_ipv4_mapped_loopback() {
        // `::ffff:127.0.0.1` must be unmapped and rejected as IPv4 loopback.
        let _g = EnvGuard::set("production");
        assert!(matches!(
            validate_fetch_url("https://[::ffff:127.0.0.1]/"),
            Err(UrlValidationError::PrivateOrReservedIp(_))
        ));
    }

    #[test]
    fn rejects_ipv4_mapped_private() {
        // `::ffff:10.0.0.5` must be unmapped and rejected as RFC1918 private.
        let _g = EnvGuard::set("production");
        assert!(matches!(
            validate_fetch_url("https://[::ffff:10.0.0.5]/feed.xml"),
            Err(UrlValidationError::PrivateOrReservedIp(_))
        ));
    }

    #[test]
    fn accepts_ipv4_mapped_public() {
        // A publicly-routable IPv4-mapped address must still pass so the
        // unmapping doesn't over-block legitimate destinations.
        let _g = EnvGuard::set("production");
        assert!(validate_fetch_url("https://[::ffff:93.184.216.34]/").is_ok());
    }
}
