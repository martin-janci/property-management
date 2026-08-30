//! Trusted-proxy-aware client-IP resolution (issue #2789).
//!
//! api-server runs behind a CDN / reverse-proxy chain (Cloudflare in
//! production, an nginx / container-network ingress in most other deploys), so
//! the direct socket peer exposed by `ConnectInfo` is usually the proxy, not the
//! visitor. To attribute a request to its real origin we must unwind the
//! `X-Forwarded-For` / `CF-Connecting-IP` forwarding headers.
//!
//! But those are ordinary request headers: **any** client can send them. The
//! earlier resolver (PR #2784) believed the leftmost `X-Forwarded-For` hop and a
//! present `CF-Connecting-IP` unconditionally, which let an attacker forge the
//! source IP. That both poisoned the shared-document access audit log and
//! defeated the `token:ip` brute-force throttle from PR #2773 — rotating a
//! spoofed IP per request yielded a fresh limiter bucket every time (#2789).
//!
//! The fix is the standard **trusted-proxy** pattern: only believe forwarding
//! headers when the request demonstrably arrived through a proxy we control,
//! i.e. when the socket peer ([`ConnectInfo`]'s address — the one value a client
//! cannot forge) is inside a configured trusted-proxy allowlist. Otherwise the
//! headers are attacker-controlled and are ignored in favour of the peer.
//!
//! `CF-Connecting-IP` needs one extra gate. Being in the trusted set only means
//! the peer is a proxy we route through — it does **not** mean that proxy is
//! Cloudflare. A generic reverse proxy (nginx, a container-network sidecar) does
//! not strip an inbound `CF-Connecting-IP`, so if we believed it on any trusted
//! peer, a client behind such a proxy could forge `CF-Connecting-IP: <any>` and
//! reopen the exact `#2789` spoof class the rightmost-untrusted-XFF path closes.
//! We therefore only consult `CF-Connecting-IP` when the operator explicitly
//! declares the trusted edge is Cloudflare (`TRUST_CF_CONNECTING_IP=1`, off by
//! default); otherwise it is ignored and resolution falls through to XFF.
//!
//! [`ConnectInfo`]: axum::extract::ConnectInfo

use axum::http::HeaderMap;
use std::net::{IpAddr, SocketAddr};

/// A single CIDR block (IPv4 or IPv6), used to decide whether a socket peer (or
/// a forwarded hop) is a trusted proxy.
#[derive(Debug, Clone, Copy)]
enum Cidr {
    V4 { network: u32, prefix: u8 },
    V6 { network: u128, prefix: u8 },
}

impl Cidr {
    /// Parse `"a.b.c.d/n"`, `"::/n"`, or a bare address (treated as a host route
    /// — `/32` for IPv4, `/128` for IPv6). Returns `None` for anything that does
    /// not parse or whose prefix is out of range.
    fn parse(spec: &str) -> Option<Cidr> {
        let spec = spec.trim();
        if spec.is_empty() {
            return None;
        }
        let (addr_part, prefix_part) = match spec.split_once('/') {
            Some((a, p)) => (a, Some(p.trim())),
            None => (spec, None),
        };
        match addr_part.trim().parse::<IpAddr>().ok()? {
            IpAddr::V4(v4) => {
                let prefix = match prefix_part {
                    Some(p) => p.parse::<u8>().ok().filter(|&p| p <= 32)?,
                    None => 32,
                };
                Some(Cidr::V4 {
                    network: mask_v4(u32::from(v4), prefix),
                    prefix,
                })
            }
            IpAddr::V6(v6) => {
                let prefix = match prefix_part {
                    Some(p) => p.parse::<u8>().ok().filter(|&p| p <= 128)?,
                    None => 128,
                };
                Some(Cidr::V6 {
                    network: mask_v6(u128::from(v6), prefix),
                    prefix,
                })
            }
        }
    }

    fn contains(&self, ip: IpAddr) -> bool {
        match (self, ip) {
            (Cidr::V4 { network, prefix }, IpAddr::V4(v4)) => {
                mask_v4(u32::from(v4), *prefix) == *network
            }
            (Cidr::V6 { network, prefix }, IpAddr::V6(v6)) => {
                mask_v6(u128::from(v6), *prefix) == *network
            }
            // Mismatched families never match; callers normalise IPv4-mapped
            // IPv6 down to IPv4 before this point (see `normalize`).
            _ => false,
        }
    }
}

fn mask_v4(bits: u32, prefix: u8) -> u32 {
    if prefix == 0 {
        0
    } else {
        bits & (u32::MAX << (32 - prefix))
    }
}

fn mask_v6(bits: u128, prefix: u8) -> u128 {
    if prefix == 0 {
        0
    } else {
        bits & (u128::MAX << (128 - prefix))
    }
}

/// Collapse an IPv4-mapped IPv6 address (`::ffff:a.b.c.d`) to its IPv4 form so a
/// dual-stack listener's peer address compares consistently against IPv4 CIDRs
/// and is stored in a stable canonical form.
fn normalize(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => IpAddr::V6(v6),
        },
        v4 => v4,
    }
}

/// The set of proxy addresses whose forwarding headers are trusted.
///
/// Built once at startup from `TRUSTED_PROXY_CIDRS` (see [`Self::from_env`]).
/// When the socket peer is not in this set the forwarding headers are treated as
/// untrusted and ignored.
#[derive(Debug, Clone, Default)]
pub struct TrustedProxies {
    cidrs: Vec<Cidr>,
    /// Whether `CF-Connecting-IP` is honoured from a trusted peer. **Off by
    /// default**: membership in `cidrs` only proves the peer is a proxy we route
    /// through, not that it is Cloudflare. A generic reverse proxy does not strip
    /// an inbound `CF-Connecting-IP`, so believing it on any trusted peer would
    /// let a client forge that header (the residual `#2789` spoof class). Only a
    /// deployment genuinely fronted by Cloudflare should opt in via
    /// `TRUST_CF_CONNECTING_IP=1`.
    trust_cf_connecting_ip: bool,
}

impl TrustedProxies {
    /// Build from a comma- or whitespace-separated CIDR / IP list. Unparseable
    /// entries are skipped with a `tracing::warn!` rather than aborting boot, so
    /// one bad entry in the env can't take the server down (it just narrows the
    /// trusted set — the fail-safe direction).
    pub fn parse(spec: &str) -> Self {
        let mut cidrs = Vec::new();
        for entry in spec.split([',', ' ', '\t', '\n']) {
            let entry = entry.trim();
            if entry.is_empty() {
                continue;
            }
            match Cidr::parse(entry) {
                Some(cidr) => cidrs.push(cidr),
                None => tracing::warn!(entry, "TRUSTED_PROXY_CIDRS: skipping unparseable entry"),
            }
        }
        Self {
            cidrs,
            trust_cf_connecting_ip: false,
        }
    }

    /// Return a copy that honours (or ignores) `CF-Connecting-IP` from trusted
    /// peers. Enable **only** when the trusted edge is genuinely Cloudflare —
    /// see [`trust_cf_connecting_ip`](Self::trust_cf_connecting_ip).
    #[must_use]
    pub fn with_cf_connecting_ip_trust(mut self, trust: bool) -> Self {
        self.trust_cf_connecting_ip = trust;
        self
    }

    /// Load the trusted-proxy allowlist from `TRUSTED_PROXY_CIDRS`.
    ///
    /// When the var is unset or empty we default to loopback + the RFC1918 /
    /// RFC4193 private ranges — the network a co-located reverse proxy, a
    /// container-network ingress, or a sidecar connects from. This keeps the
    /// audit-attribution + throttle-keying working for the common
    /// private-network proxy deployment out of the box while still refusing to
    /// believe headers from a directly-reachable public client.
    ///
    /// Deployments fronted by a **public** edge (Cloudflare, a cloud LB) must
    /// set `TRUSTED_PROXY_CIDRS` to that edge's published ranges — otherwise the
    /// edge's public peer address is (correctly) untrusted and the visitor is
    /// attributed to the edge IP rather than being spoofable.
    ///
    /// When the trusted edge is Cloudflare, additionally set
    /// `TRUST_CF_CONNECTING_IP=1` so `CF-Connecting-IP` is believed; otherwise
    /// (the default) that header is ignored and resolution uses XFF, closing the
    /// spoof for generic private-range reverse proxies.
    pub fn from_env() -> Self {
        let base = match std::env::var("TRUSTED_PROXY_CIDRS") {
            Ok(spec) if !spec.trim().is_empty() => Self::parse(&spec),
            _ => Self::private_defaults(),
        };
        base.with_cf_connecting_ip_trust(cf_connecting_ip_trusted_from_env())
    }

    /// Loopback + private ranges — the default trusted set (see [`from_env`]).
    ///
    /// [`from_env`]: Self::from_env
    fn private_defaults() -> Self {
        Self::parse(
            "127.0.0.0/8, ::1/128, 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16, fc00::/7, fe80::/10",
        )
    }

    /// Whether `ip` (an observed socket peer or forwarded hop) is a trusted
    /// proxy. IPv4-mapped IPv6 peers are normalised first.
    pub fn contains(&self, ip: IpAddr) -> bool {
        let ip = normalize(ip);
        self.cidrs.iter().any(|c| c.contains(ip))
    }

    /// Whether the trusted set is empty (no proxy is trusted → headers are never
    /// believed). Present for tests / diagnostics.
    pub fn is_empty(&self) -> bool {
        self.cidrs.is_empty()
    }
}

/// Read `TRUST_CF_CONNECTING_IP` as a boolean opt-in (default `false`).
/// Accepts `1`, `true`, `yes`, `on` (case-insensitive); anything else — unset,
/// empty, `0`, garbage — is `false`, the fail-safe direction.
fn cf_connecting_ip_trusted_from_env() -> bool {
    matches!(
        std::env::var("TRUST_CF_CONNECTING_IP")
            .map(|v| v.trim().to_ascii_lowercase())
            .as_deref(),
        Ok("1" | "true" | "yes" | "on")
    )
}

/// Parse a single textual IP (trimming surrounding whitespace) and return it in
/// canonical, IPv4-mapped-normalised form.
fn parse_ip(raw: &str) -> Option<IpAddr> {
    raw.trim().parse::<IpAddr>().ok().map(normalize)
}

/// Read `CF-Connecting-IP` (the single origin address Cloudflare overwrites on
/// every proxied request). Only meaningful once the peer is already known to be
/// trusted.
fn cf_connecting_ip(headers: &HeaderMap) -> Option<IpAddr> {
    parse_ip(headers.get("cf-connecting-ip")?.to_str().ok()?)
}

/// Walk `X-Forwarded-For` **right → left**, returning the first hop that is not
/// itself a trusted proxy.
///
/// Proxies *append* the peer they saw to the right of the list, so the trusted
/// edge's own view sits on the right and any client-supplied (potentially
/// forged) values sit on the left. Selecting the rightmost *untrusted* hop —
/// rather than the leftmost, which the earlier code did — means an attacker who
/// prepends `X-Forwarded-For: <forged>` cannot win: their forged entry is to the
/// left of the real address our trusted proxy appended. Unparseable hops are
/// skipped.
fn rightmost_untrusted_xff(headers: &HeaderMap, trusted: &TrustedProxies) -> Option<IpAddr> {
    let raw = headers.get("x-forwarded-for")?.to_str().ok()?;
    for hop in raw.split(',').rev() {
        let ip = match parse_ip(hop) {
            Some(ip) => ip,
            None => continue,
        };
        if !trusted.contains(ip) {
            return Some(ip);
        }
    }
    None
}

/// Resolve the real client IP for audit logging and per-client throttle keying.
///
/// Contract (issue #2789):
/// 1. If the socket `addr` is **not** a trusted proxy, the forwarding headers
///    are attacker-controlled — return the (normalised) socket peer and ignore
///    the headers entirely.
/// 2. Otherwise prefer `CF-Connecting-IP` **only when the trusted edge is
///    declared to be Cloudflare** (`TrustedProxies::trust_cf_connecting_ip`),
///    then the rightmost untrusted `X-Forwarded-For` hop, then fall back to the
///    socket peer. When the edge is a generic reverse proxy, `CF-Connecting-IP`
///    is client-forgeable and is ignored.
///
/// The returned string is the canonical form of the chosen [`IpAddr`].
pub fn resolve_client_ip(
    headers: &HeaderMap,
    addr: SocketAddr,
    trusted: &TrustedProxies,
) -> String {
    let peer = normalize(addr.ip());
    if !trusted.contains(peer) {
        return peer.to_string();
    }
    let cf = if trusted.trust_cf_connecting_ip {
        cf_connecting_ip(headers)
    } else {
        None
    };
    cf.or_else(|| rightmost_untrusted_xff(headers, trusted))
        .unwrap_or(peer)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Default trusted set (loopback + private ranges) as loaded when
    /// `TRUSTED_PROXY_CIDRS` is unset. `CF-Connecting-IP` trust is OFF — the
    /// shipped default for a generic reverse proxy.
    fn default_trusted() -> TrustedProxies {
        TrustedProxies::private_defaults()
    }

    /// A Cloudflare-fronted trusted set: same private ranges but with
    /// `CF-Connecting-IP` trust explicitly opted in (`TRUST_CF_CONNECTING_IP=1`).
    fn cf_trusted() -> TrustedProxies {
        TrustedProxies::private_defaults().with_cf_connecting_ip_trust(true)
    }

    /// A trusted (private-range) reverse-proxy socket peer.
    fn proxy_peer() -> SocketAddr {
        "10.0.0.5:443".parse().unwrap()
    }

    /// A directly-connecting, UNtrusted public socket peer.
    fn public_peer() -> SocketAddr {
        "203.0.113.200:51344".parse().unwrap()
    }

    // ── Trust gating (the #2789 core) ───────────────────────────────────────

    /// IG3: a spoofed `X-Forwarded-For` (and `CF-Connecting-IP`) from an
    /// UNtrusted peer must be ignored — the resolver returns the real socket
    /// peer, so the attacker cannot mint a fresh throttle bucket per request or
    /// forge the audit-log source IP. This FAILS against the pre-fix resolver
    /// (which returned the spoofed leftmost XFF hop unconditionally).
    #[test]
    fn ignores_forwarding_headers_from_untrusted_peer() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "1.2.3.4".parse().unwrap());
        headers.insert("cf-connecting-ip", "5.6.7.8".parse().unwrap());
        let ip = resolve_client_ip(&headers, public_peer(), &default_trusted());
        assert_eq!(
            ip, "203.0.113.200",
            "headers from a non-proxy peer must not override the socket address"
        );
    }

    /// A different spoofed IP on each request from the same untrusted peer still
    /// resolves to the one socket address — so the `token:ip` throttle bucket is
    /// stable and the limiter actually trips.
    #[test]
    fn untrusted_peer_ip_is_stable_across_spoofed_headers() {
        let mut a = HeaderMap::new();
        a.insert("x-forwarded-for", "1.1.1.1".parse().unwrap());
        let mut b = HeaderMap::new();
        b.insert("x-forwarded-for", "2.2.2.2".parse().unwrap());
        let ta = default_trusted();
        let ra = resolve_client_ip(&a, public_peer(), &ta);
        let rb = resolve_client_ip(&b, public_peer(), &ta);
        assert_eq!(
            ra, rb,
            "rotating spoofed XFF must not change the throttle key"
        );
        assert_eq!(ra, "203.0.113.200");
    }

    // ── Trusted-proxy path ──────────────────────────────────────────────────

    #[test]
    fn falls_back_to_socket_peer_without_proxy_headers() {
        let ip = resolve_client_ip(&HeaderMap::new(), proxy_peer(), &default_trusted());
        assert_eq!(ip, "10.0.0.5", "no proxy headers ⇒ trusted peer address");
    }

    #[test]
    fn prefers_cf_connecting_ip_from_trusted_peer() {
        let mut headers = HeaderMap::new();
        headers.insert("cf-connecting-ip", "203.0.113.7".parse().unwrap());
        // A conflicting XFF must not win over the Cloudflare-set client IP.
        headers.insert("x-forwarded-for", "198.51.100.1".parse().unwrap());
        // Cloudflare-fronted deployment: CF trust is explicitly opted in.
        let ip = resolve_client_ip(&headers, proxy_peer(), &cf_trusted());
        assert_eq!(ip, "203.0.113.7", "CF-Connecting-IP takes precedence");
    }

    /// The residual `#2789` hole: a generic private-range reverse proxy (CF trust
    /// OFF — the shipped default) does not strip an inbound `CF-Connecting-IP`,
    /// so it is client-forgeable. It must NOT become the resolved client IP; with
    /// no XFF we fall back to the trusted socket peer.
    #[test]
    fn forged_cf_connecting_ip_ignored_when_flag_off() {
        let mut headers = HeaderMap::new();
        headers.insert("cf-connecting-ip", "6.6.6.6".parse().unwrap());
        let ip = resolve_client_ip(&headers, proxy_peer(), &default_trusted());
        assert_ne!(
            ip, "6.6.6.6",
            "a forged CF-Connecting-IP must not be trusted unless the edge is Cloudflare"
        );
        assert_eq!(ip, "10.0.0.5", "falls back to the trusted socket peer");
    }

    /// With CF trust OFF, a forged `CF-Connecting-IP` is ignored and resolution
    /// falls through to the rightmost untrusted XFF hop — the same real client
    /// the XFF path would pick with no CF header present.
    #[test]
    fn forged_cf_connecting_ip_falls_through_to_xff_when_flag_off() {
        let mut headers = HeaderMap::new();
        headers.insert("cf-connecting-ip", "6.6.6.6".parse().unwrap());
        headers.insert("x-forwarded-for", "203.0.113.9, 10.0.0.5".parse().unwrap());
        let ip = resolve_client_ip(&headers, proxy_peer(), &default_trusted());
        assert_eq!(
            ip, "203.0.113.9",
            "CF header ignored (flag off); rightmost untrusted XFF hop wins"
        );
    }

    /// The real client is the rightmost *untrusted* hop: internal proxies
    /// (private ranges) on the right are skipped, and a forged public value
    /// prepended on the LEFT does not win.
    #[test]
    fn uses_rightmost_untrusted_forwarded_for_hop() {
        let mut headers = HeaderMap::new();
        // forged(left), real client, two trusted proxies appended on the right.
        headers.insert(
            "x-forwarded-for",
            "9.9.9.9, 203.0.113.9, 10.0.0.7, 10.0.0.5".parse().unwrap(),
        );
        let ip = resolve_client_ip(&headers, proxy_peer(), &default_trusted());
        assert_eq!(
            ip, "203.0.113.9",
            "rightmost non-proxy hop is the origin client; forged left hop ignored"
        );
    }

    #[test]
    fn skips_garbage_cf_header_and_uses_forwarded_for() {
        let mut headers = HeaderMap::new();
        headers.insert("cf-connecting-ip", "not-an-ip".parse().unwrap());
        headers.insert("x-forwarded-for", "203.0.113.11".parse().unwrap());
        let ip = resolve_client_ip(&headers, proxy_peer(), &cf_trusted());
        assert_eq!(
            ip, "203.0.113.11",
            "an unparsable CF header must fall through to XFF"
        );
    }

    #[test]
    fn skips_all_garbage_headers_and_uses_socket_peer() {
        let mut headers = HeaderMap::new();
        headers.insert("cf-connecting-ip", "garbage".parse().unwrap());
        headers.insert(
            "x-forwarded-for",
            "also-garbage, still-bad".parse().unwrap(),
        );
        let ip = resolve_client_ip(&headers, proxy_peer(), &default_trusted());
        assert_eq!(
            ip, "10.0.0.5",
            "present-but-invalid proxy headers must not poison the audit trail"
        );
    }

    #[test]
    fn canonicalises_parsed_address() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "cf-connecting-ip",
            "2001:0db8:0000:0000:0000:0000:0000:0001".parse().unwrap(),
        );
        let ip = resolve_client_ip(&headers, proxy_peer(), &cf_trusted());
        assert_eq!(
            ip, "2001:db8::1",
            "the stored value is the canonical IP form"
        );
    }

    #[test]
    fn trims_whitespace_around_forwarded_for_hop() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            " 203.0.113.20 , 10.0.0.5".parse().unwrap(),
        );
        let ip = resolve_client_ip(&headers, proxy_peer(), &default_trusted());
        assert_eq!(ip, "203.0.113.20", "surrounding whitespace is trimmed");
    }

    /// An IPv4-mapped IPv6 socket peer (dual-stack listener) is normalised and
    /// still recognised against an IPv4 trusted CIDR.
    #[test]
    fn normalises_ipv4_mapped_peer_for_trust_check() {
        let trusted = TrustedProxies::parse("10.0.0.0/8").with_cf_connecting_ip_trust(true);
        let peer: SocketAddr = "[::ffff:10.0.0.9]:443".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert("cf-connecting-ip", "203.0.113.42".parse().unwrap());
        let ip = resolve_client_ip(&headers, peer, &trusted);
        assert_eq!(
            ip, "203.0.113.42",
            "an IPv4-mapped trusted peer must still unlock the forwarding headers"
        );
    }

    // ── Config parsing ──────────────────────────────────────────────────────

    #[test]
    fn parse_accepts_mixed_cidr_and_bare_ip() {
        let t = TrustedProxies::parse("127.0.0.1, 10.0.0.0/8, 2001:db8::/32, junk, /8");
        assert!(t.contains("127.0.0.1".parse().unwrap()));
        assert!(t.contains("10.9.9.9".parse().unwrap()));
        assert!(t.contains("2001:db8::dead".parse().unwrap()));
        assert!(!t.contains("11.0.0.1".parse().unwrap()));
        // "junk" and "/8" are skipped, but the valid entries survived.
        assert!(!t.is_empty());
    }

    #[test]
    fn empty_trusted_set_never_believes_headers() {
        let t = TrustedProxies::default();
        assert!(t.is_empty());
        let mut headers = HeaderMap::new();
        headers.insert("cf-connecting-ip", "203.0.113.7".parse().unwrap());
        // Even a loopback peer is untrusted when the set is empty.
        let ip = resolve_client_ip(&headers, "127.0.0.1:8080".parse().unwrap(), &t);
        assert_eq!(ip, "127.0.0.1");
    }

    #[test]
    fn cidr_boundaries_hold() {
        let t = TrustedProxies::parse("192.168.0.0/16");
        assert!(t.contains("192.168.255.255".parse::<IpAddr>().unwrap()));
        assert!(!t.contains("192.169.0.0".parse::<IpAddr>().unwrap()));
        // /0 matches everything.
        let all = TrustedProxies::parse("0.0.0.0/0");
        assert!(all.contains(IpAddr::V4(std::net::Ipv4Addr::new(8, 8, 8, 8))));
    }
}
