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
//! **DNS-hostname SSRF**: [`validate_external_url`] is a *static* check on
//! the string — a hostname that has no literal-IP form (e.g.
//! `attacker.example.com`) but resolves to a private/internal address at
//! request time sails through it untouched. Any caller that is about to
//! issue the actual outbound request (i.e. every caller in this codebase)
//! **MUST** additionally call [`validate_resolved_host`] with the validated
//! URL's host/port immediately before sending the request, and reject on
//! error. `validate_resolved_host` resolves the hostname and checks *every*
//! returned address against the same forbidden ranges, failing closed
//! (resolution error, empty result, or any forbidden address => reject).
//!
//! Note this still leaves a narrow DNS-rebinding window between the
//! `validate_resolved_host` lookup and the HTTP client's own connect-time
//! lookup (TOCTOU) since the underlying HTTP client, not this module, owns
//! the socket connect. Closing that fully would require pinning the
//! validated IP for the connection (a custom resolver/connector), which is
//! out of scope here; re-resolving immediately before the request narrows
//! the window to the two lookups happening back-to-back.

use std::net::IpAddr;
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
    /// DNS resolution failed, or resolved to zero addresses. Fail closed:
    /// we can't prove the destination is safe, so we reject it.
    ResolutionFailed(String),
    /// The hostname resolved to (at least one) IP in a forbidden range.
    ResolvedToPrivateIp(String),
}

impl std::fmt::Display for UrlValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFormat(e) => write!(f, "Invalid URL format: {}", e),
            Self::DisallowedScheme(s) => write!(f, "URL scheme '{}' is not allowed", s),
            Self::PlainHttpNotAllowed => {
                write!(
                    f,
                    "Plain http:// URLs are not allowed in production; use https://"
                )
            }
            Self::MissingHost => write!(f, "URL must have a host component"),
            Self::BlockedHost(h) => {
                write!(f, "Host '{}' is not permitted for outbound requests", h)
            }
            Self::PrivateOrReservedIp(reason) => {
                write!(f, "URL points to a forbidden network range: {}", reason)
            }
            Self::BlockedTld(h) => write!(f, "Internal domain not allowed: {}", h),
            Self::ResolutionFailed(e) => write!(f, "Could not resolve host: {}", e),
            Self::ResolvedToPrivateIp(reason) => {
                write!(f, "Host resolves to a forbidden network range: {}", reason)
            }
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
    let parsed = Url::parse(raw).map_err(|e| UrlValidationError::InvalidFormat(e.to_string()))?;

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

    // IP range checks (literal IPs only — see `validate_resolved_host` for
    // the DNS-hostname case, which this static check cannot see).
    match host {
        Host::Ipv4(ip) => {
            if let Err(reason) = is_forbidden_ip(IpAddr::V4(ip)) {
                return Err(UrlValidationError::PrivateOrReservedIp(reason));
            }
        }
        Host::Ipv6(ip) => {
            if let Err(reason) = is_forbidden_ip(IpAddr::V6(ip)) {
                return Err(UrlValidationError::PrivateOrReservedIp(reason));
            }
        }
        Host::Domain(_) => {}
    }

    Ok(parsed)
}

/// Resolve `host`/`port` and validate that **every** returned address is a
/// routable, non-forbidden IP. This is the second half of SSRF defence for
/// DNS hostnames — [`validate_external_url`] only inspects the literal
/// string and cannot see where a domain actually resolves.
///
/// Fails closed: a resolution error, an empty result set, or any single
/// forbidden address in the result set causes rejection. Call this
/// immediately before issuing the outbound request (after
/// `validate_external_url` has already passed), using the same host/port
/// the HTTP client is about to connect to.
pub async fn validate_resolved_host(host: &str, port: u16) -> Result<(), UrlValidationError> {
    validate_resolved_host_with(host.to_string(), port, resolve_host).await
}

/// Same as [`validate_resolved_host`] but with the resolver injected, so
/// tests can exercise the private-IP-rejection path with a fake resolver
/// instead of depending on live DNS/network access (which CI/sandbox
/// runners may not have).
///
/// `host` is owned (`String`) rather than borrowed: an `F: FnOnce(&str, ..)
/// -> Fut` bound ties `Fut` to the elided lifetime of that borrow, which
/// rustc can't unify with the higher-ranked lifetime the call site needs
/// (`implementation of FnOnce is not general enough`). Taking ownership
/// sidesteps the borrow-checker issue entirely.
async fn validate_resolved_host_with<F, Fut>(
    host: String,
    port: u16,
    resolve: F,
) -> Result<(), UrlValidationError>
where
    F: FnOnce(String, u16) -> Fut,
    Fut: std::future::Future<Output = std::io::Result<Vec<IpAddr>>>,
{
    let addrs = resolve(host.clone(), port)
        .await
        .map_err(|e| UrlValidationError::ResolutionFailed(e.to_string()))?;

    if addrs.is_empty() {
        return Err(UrlValidationError::ResolutionFailed(format!(
            "no addresses found for host '{}'",
            host
        )));
    }

    for ip in addrs {
        if let Err(reason) = is_forbidden_ip(ip) {
            return Err(UrlValidationError::ResolvedToPrivateIp(format!(
                "{} resolves to {} ({})",
                host, ip, reason
            )));
        }
    }

    Ok(())
}

/// Default resolver: real DNS via the OS resolver (through Tokio's async
/// wrapper around `getaddrinfo`). Extracted so tests can substitute a fake
/// resolver instead of depending on live DNS/network access.
async fn resolve_host(host: String, port: u16) -> std::io::Result<Vec<IpAddr>> {
    let addrs = tokio::net::lookup_host((host.as_str(), port)).await?;
    Ok(addrs.map(|sa| sa.ip()).collect())
}

/// Returns `Ok(())` when `ip` is a routable, non-forbidden address (IPv4 or
/// IPv6). Backs both the literal-IP check in `validate_external_url` and
/// the resolved-IP check in `validate_resolved_host`.
fn is_forbidden_ip(ip: IpAddr) -> Result<(), String> {
    match ip {
        IpAddr::V4(v4) => check_ipv4(v4),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return check_ipv4(v4);
            }
            let segs = v6.segments();
            if (segs[0] & 0xfe00) == 0xfc00 {
                return Err("fc00::/7 (IPv6 unique-local)".into());
            }
            if (segs[0] & 0xffc0) == 0xfe80 {
                return Err("fe80::/10 (IPv6 link-local)".into());
            }
            if (segs[0] & 0xff00) == 0xff00 {
                return Err("ff00::/8 (IPv6 multicast)".into());
            }
            if v6.is_loopback() {
                return Err("::1 (IPv6 loopback)".into());
            }
            if v6.is_unspecified() {
                return Err(":: (IPv6 unspecified)".into());
            }
            Ok(())
        }
    }
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

    /// A domain host has no literal-IP form, so `validate_external_url`
    /// (the static check) can't reject it on IP-range grounds — this is by
    /// design; the resolved-IP check is a separate, second step.
    #[test]
    fn validate_external_url_lets_ordinary_domains_through() {
        assert!(validate_external_url("https://attacker.example.com/x").is_ok());
    }

    /// Regression (SSRF via DNS): a hostname with no literal-IP form that
    /// resolves to a private/internal address must be rejected. Before
    /// `validate_resolved_host` existed, every caller only ran the static
    /// `validate_external_url` check — which explicitly no-ops on
    /// `Host::Domain` — so a hostname like `internal.attacker.example`
    /// resolving to `10.0.0.5` sailed straight through to the outbound
    /// request. Fails to compile on `dev` (`validate_resolved_host_with`
    /// does not exist there); passes here because every resolved address is
    /// checked against the forbidden ranges and any hit is rejected.
    ///
    /// Uses an injected fake resolver so the test is hermetic — no real DNS
    /// lookup, no network access required — matching this repo's SSRF-test
    /// convention (see `workflow_webhook_ssrf_tests.rs`).
    #[tokio::test]
    async fn validate_resolved_host_rejects_hostname_resolving_to_private_ip() {
        async fn fake_resolve(_host: String, _port: u16) -> std::io::Result<Vec<IpAddr>> {
            Ok(vec!["10.0.0.5".parse().unwrap()])
        }

        let result =
            validate_resolved_host_with("internal.attacker.example".to_string(), 443, fake_resolve)
                .await;

        assert!(
            matches!(result, Err(UrlValidationError::ResolvedToPrivateIp(_))),
            "expected rejection for a hostname resolving to a private IP, got: {result:?}"
        );
    }

    /// Fail-closed: if *any* resolved address is forbidden, the whole
    /// hostname is rejected — even when other addresses in the answer are
    /// public. A caller/HTTP client could pick any of the returned
    /// addresses, so accepting on the presence of a public one and ignoring
    /// the private one would leave the SSRF hole open.
    #[tokio::test]
    async fn validate_resolved_host_rejects_when_any_resolved_ip_is_private() {
        async fn fake_resolve(_host: String, _port: u16) -> std::io::Result<Vec<IpAddr>> {
            Ok(vec![
                "93.184.216.34".parse().unwrap(),
                "169.254.169.254".parse().unwrap(),
            ])
        }

        let result =
            validate_resolved_host_with("multi-answer.example".to_string(), 443, fake_resolve)
                .await;

        assert!(
            matches!(result, Err(UrlValidationError::ResolvedToPrivateIp(_))),
            "expected rejection when one of several resolved IPs is private, got: {result:?}"
        );
    }

    /// A hostname that resolves only to public addresses must pass.
    #[tokio::test]
    async fn validate_resolved_host_accepts_hostname_resolving_to_public_ip() {
        async fn fake_resolve(_host: String, _port: u16) -> std::io::Result<Vec<IpAddr>> {
            Ok(vec!["93.184.216.34".parse().unwrap()])
        }

        let result =
            validate_resolved_host_with("example.com".to_string(), 443, fake_resolve).await;
        assert!(result.is_ok(), "expected success, got: {result:?}");
    }

    /// Fail closed on resolution errors too — we can't prove the
    /// destination is safe, so we don't proceed.
    #[tokio::test]
    async fn validate_resolved_host_rejects_on_resolution_error() {
        async fn fake_resolve(_host: String, _port: u16) -> std::io::Result<Vec<IpAddr>> {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "simulated NXDOMAIN",
            ))
        }

        let result =
            validate_resolved_host_with("nonexistent.example".to_string(), 443, fake_resolve).await;
        assert!(matches!(
            result,
            Err(UrlValidationError::ResolutionFailed(_))
        ));
    }

    /// Fail closed on an empty answer set too.
    #[tokio::test]
    async fn validate_resolved_host_rejects_on_empty_resolution() {
        async fn fake_resolve(_host: String, _port: u16) -> std::io::Result<Vec<IpAddr>> {
            Ok(vec![])
        }

        let result =
            validate_resolved_host_with("empty-answer.example".to_string(), 443, fake_resolve)
                .await;
        assert!(matches!(
            result,
            Err(UrlValidationError::ResolutionFailed(_))
        ));
    }
}
