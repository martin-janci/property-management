//! Shared SSRF-defence URL validation for all outbound HTTP calls.
//!
//! Any handler or service that accepts a user-supplied (or provider-supplied)
//! URL and then fetches it server-side **MUST** call [`validate_external_url`]
//! before issuing the request. The validator rejects:
//!
//! * non-`http`/`https` schemes (`file://`, `data:`, `javascript:`, ...),
//! * `http://` outside development builds (`RUST_ENV=development`),
//! * literal IPs in private, loopback, link-local, multicast, cloud-metadata
//!   (169.254.169.254), CGNAT (100.64/10), and reserved ranges (IPv4 + IPv6),
//! * well-known cloud-metadata/internal hostnames
//!   (`metadata.google.internal`, `169.254.169.254`, ...),
//! * `.local` / `.internal` TLD suffixes.
//!
//! **Important -- DNS rebinding**: this is a *static* check on the string.
//! Callers that resolve the hostname themselves (e.g. import workers) MUST
//! re-validate the resolved IP at fetch time.

use url::{Host, Url};

/// Structured error returned when a URL fails SSRF validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UrlValidationError {
    /// URL failed to parse.
    InvalidFormat(String),
    /// Scheme is not `http` or `https`.
    DisallowedScheme(String),
    /// Plain `http://` used in a production build.
    PlainHttpNotAllowed,
    /// URL has no host component.
    MissingHost,
    /// Host is a blocked hostname (e.g. cloud-metadata endpoint).
    BlockedHost(String),
    /// Host is a literal IP in a forbidden range.
    PrivateOrReservedIp(String),
    /// Host ends with a blocked TLD (`.local`, `.internal`).
    BlockedTld(String),
}

impl std::fmt::Display for UrlValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFormat(e) => write!(f, "Invalid URL format: {}", e),
            Self::DisallowedScheme(s) => write!(f, "URL scheme '{}' is not allowed", s),
            Self::PlainHttpNotAllowed => {
                write!(f, "Plain http:// URLs are not allowed in production; use https://")
            }
            Self::MissingHost => write!(f, "URL must have a host component"),
            Self::BlockedHost(h) => write!(f, "Host '{}' is not permitted for outbound requests", h),
            Self::PrivateOrReservedIp(reason) => {
                write!(f, "URL points to a forbidden network range: {}", reason)
            }
            Self::BlockedTld(h) => write!(f, "Internal domain not allowed: {}", h),
        }
    }
}

impl std::error::Error for UrlValidationError {}

/// Well-known hostnames that must always be blocked regardless of environment.
const BLOCKED_HOSTNAMES: &[&str] = &[
    "metadata.google.internal",
    "169.254.169.254",
    "localhost",
    "127.0.0.1",
    "0.0.0.0",
    "::1",
    "[::1]",
];

/// Returns `true` when the process is running in a development build.
///
/// `RUST_ENV` must be `"development"` to relax the `http://` restriction.
fn is_development() -> bool {
    std::env::var("RUST_ENV")
        .map(|v| v == "development")
        .unwrap_or(false)
}

/// Validate that `raw` is safe to fetch server-side (SSRF defence).
///
/// On success returns the parsed [`Url`]. On failure returns a
/// [`UrlValidationError`] that the caller can map to a 400/422 response.
pub fn validate_external_url(raw: &str) -> Result<Url, UrlValidationError> {
    let parsed =
        Url::parse(raw).map_err(|e| UrlValidationError::InvalidFormat(e.to_string()))?;

    // Scheme allowlist: only http(s).
    match parsed.scheme() {
        "https" => {}
        "http" => {
            if !is_development() {
                return Err(UrlValidationError::PlainHttpNotAllowed);
            }
        }
        other => return Err(UrlValidationError::DisallowedScheme(other.to_string())),
    }

    // Must have a host.
    let host = parsed.host().ok_or(UrlValidationError::MissingHost)?;
    let host_str = parsed.host_str().unwrap_or("");

    // Blocklisted hostnames.
    if BLOCKED_HOSTNAMES
        .iter()
        .any(|&h| host_str.eq_ignore_ascii_case(h))
    {
        return Err(UrlValidationError::BlockedHost(host_str.to_string()));
    }

    // Blocked TLDs.
    let lc = host_str.to_lowercase();
    if lc.ends_with(".local") || lc.ends_with(".internal") {
        return Err(UrlValidationError::BlockedTld(host_str.to_string()));
    }

    // IP range checks.
    match host {
        Host::Ipv4(ip) => {
            if let Err(reason) = check_ipv4(ip) {
                return Err(UrlValidationError::PrivateOrReservedIp(reason));
            }
        }
        Host::Ipv6(ip) => {
            if let Some(v4) = ip.to_ipv4_mapped() {
                if let Err(reason) = check_ipv4(v4) {
                    return Err(UrlValidationError::PrivateOrReservedIp(reason));
                }
            } else {
                let segs = ip.segments();
                if (segs[0] & 0xfe00) == 0xfc00 {
                    return Err(UrlValidationError::PrivateOrReservedIp(
                        "fc00::/7 (IPv6 unique-local)".into(),
                    ));
                }
                if (segs[0] & 0xffc0) == 0xfe80 {
                    return Err(UrlValidationError::PrivateOrReservedIp(
                        "fe80::/10 (IPv6 link-local)".into(),
                    ));
                }
                if (segs[0] & 0xff00) == 0xff00 {
                    return Err(UrlValidationError::PrivateOrReservedIp(
                        "ff00::/8 (IPv6 multicast)".into(),
                    ));
                }
                if ip.is_loopback() {
                    return Err(UrlValidationError::PrivateOrReservedIp(
                        "::1 (IPv6 loopback)".into(),
                    ));
                }
                if ip.is_unspecified() {
                    return Err(UrlValidationError::PrivateOrReservedIp(
                        ":: (IPv6 unspecified)".into(),
                    ));
                }
            }
        }
        Host::Domain(_) => {}
    }

    Ok(parsed)
}

/// Returns `Ok(())` when `ip` is routable public IPv4.
fn check_ipv4(ip: std::net::Ipv4Addr) -> Result<(), String> {
    let o = ip.octets();
    if o[0] == 10 {
        return Err("10.0.0.0/8 (private)".into());
    }
    if o[0] == 172 && (16..=31).contains(&o[1]) {
        return Err("172.16.0.0/12 (private)".into());
    }
    if o[0] == 192 && o[1] == 168 {
        return Err("192.168.0.0/16 (private)".into());
    }
    if o[0] == 127 {
        return Err("127.0.0.0/8 (loopback)".into());
    }
    if o[0] == 169 && o[1] == 254 {
        return Err("169.254.0.0/16 (link-local / cloud metadata)".into());
    }
    if o[0] == 0 {
        return Err("0.0.0.0/8 (reserved)".into());
    }
    if o[0] == 100 && (o[1] & 0xc0) == 0x40 {
        return Err("100.64.0.0/10 (CGNAT)".into());
    }
    if (o[0] == 192 && o[1] == 0 && o[2] == 2)
        || (o[0] == 198 && o[1] == 51 && o[2] == 100)
        || (o[0] == 203 && o[1] == 0 && o[2] == 113)
    {
        return Err("documentation range".into());
    }
    if (224..=239).contains(&o[0]) {
        return Err("224.0.0.0/4 (multicast)".into());
    }
    if o == [255, 255, 255, 255] {
        return Err("255.255.255.255 (broadcast)".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_https_domain() {
        assert!(validate_external_url("https://example.com/feed").is_ok());
    }

    #[test]
    fn rejects_file_scheme() {
        assert!(matches!(
            validate_external_url("file:///etc/passwd"),
            Err(UrlValidationError::DisallowedScheme(_))
        ));
    }

    #[test]
    fn rejects_javascript_scheme() {
        assert!(matches!(
            validate_external_url("javascript:alert(1)"),
            Err(UrlValidationError::DisallowedScheme(_))
        ));
    }

    #[test]
    fn rejects_data_scheme() {
        assert!(matches!(
            validate_external_url("data:text/plain,hello"),
            Err(UrlValidationError::DisallowedScheme(_))
        ));
    }

    #[test]
    fn rejects_localhost() {
        assert!(matches!(
            validate_external_url("https://localhost/"),
            Err(UrlValidationError::BlockedHost(_))
        ));
    }

    #[test]
    fn rejects_loopback_literal() {
        assert!(matches!(
            validate_external_url("https://127.0.0.1/"),
            Err(UrlValidationError::BlockedHost(_) | UrlValidationError::PrivateOrReservedIp(_))
        ));
    }

    #[test]
    fn rejects_private_10() {
        assert!(matches!(
            validate_external_url("https://10.0.0.5/"),
            Err(UrlValidationError::PrivateOrReservedIp(_))
        ));
    }

    #[test]
    fn rejects_private_172() {
        assert!(matches!(
            validate_external_url("https://172.16.0.1/"),
            Err(UrlValidationError::PrivateOrReservedIp(_))
        ));
    }

    #[test]
    fn rejects_private_192_168() {
        assert!(matches!(
            validate_external_url("https://192.168.1.1/"),
            Err(UrlValidationError::PrivateOrReservedIp(_))
        ));
    }

    #[test]
    fn rejects_link_local_metadata() {
        assert!(matches!(
            validate_external_url("https://169.254.169.254/latest/meta-data/"),
            Err(UrlValidationError::BlockedHost(_) | UrlValidationError::PrivateOrReservedIp(_))
        ));
    }

    #[test]
    fn rejects_gcp_metadata_hostname() {
        assert!(matches!(
            validate_external_url("https://metadata.google.internal/computeMetadata/v1/"),
            Err(UrlValidationError::BlockedHost(_))
        ));
    }

    #[test]
    fn rejects_cgnat() {
        assert!(matches!(
            validate_external_url("https://100.64.0.1/"),
            Err(UrlValidationError::PrivateOrReservedIp(_))
        ));
    }

    #[test]
    fn rejects_ipv6_loopback() {
        assert!(matches!(
            validate_external_url("https://[::1]/"),
            Err(UrlValidationError::BlockedHost(_) | UrlValidationError::PrivateOrReservedIp(_))
        ));
    }

    #[test]
    fn rejects_ipv6_unique_local() {
        assert!(matches!(
            validate_external_url("https://[fc00::1]/"),
            Err(UrlValidationError::PrivateOrReservedIp(_))
        ));
    }

    #[test]
    fn rejects_ipv6_link_local() {
        assert!(matches!(
            validate_external_url("https://[fe80::1]/"),
            Err(UrlValidationError::PrivateOrReservedIp(_))
        ));
    }

    #[test]
    fn rejects_ipv4_mapped_ipv6_private() {
        assert!(matches!(
            validate_external_url("https://[::ffff:10.0.0.1]/"),
            Err(UrlValidationError::PrivateOrReservedIp(_))
        ));
    }

    #[test]
    fn rejects_dot_local_tld() {
        assert!(matches!(
            validate_external_url("https://printer.local/"),
            Err(UrlValidationError::BlockedTld(_))
        ));
    }

    #[test]
    fn rejects_dot_internal_tld() {
        assert!(matches!(
            validate_external_url("https://redis.internal/"),
            Err(UrlValidationError::BlockedTld(_))
        ));
    }

    #[test]
    fn rejects_invalid_url() {
        assert!(matches!(
            validate_external_url("not a url"),
            Err(UrlValidationError::InvalidFormat(_))
        ));
    }
}
