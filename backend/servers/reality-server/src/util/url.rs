//! Shared image/link URL hygiene for portal-rendered URLs (H7).
//!
//! A number of portal fields hold user-supplied URLs that later end up in
//! `<img src=>` / `<a href=>` on the frontend (agency `logo_url` /
//! `banner_url` / `watermark_url`, portal-user `profile_image_url`, listing
//! report attachment URLs). To block stored XSS we reject `javascript:`,
//! `data:`, `file:` and every other non-`http(s)` scheme, and cap the length
//! so a single field can't bloat DB rows / logs.
//!
//! This is an **XSS-focused scheme check** on a string that will be *rendered*
//! — deliberately distinct from the SSRF fetch validator in
//! [`common::url_validation`], which is for URLs the server will *fetch* and so
//! also blocks plain `http://` in production, private/cloud-metadata IPs, etc.
//! The two are not interchangeable: this one intentionally accepts
//! `http://localhost` and private hosts because those are never fetched
//! server-side, only echoed to a browser. Prior to this module the identical
//! check was copy-pasted into `routes::agency_branding`, `routes::users` and
//! `routes::reports`; keep the single copy here.

/// Maximum accepted length (in characters) of an image/link URL field.
pub const MAX_IMAGE_OR_LINK_URL_LEN: usize = 2048;

/// Validate a user-supplied URL that will be rendered in `<img src=>` /
/// `<a href=>` on the frontend.
///
/// Returns `Err` with a generic, caller-facing message when the URL is longer
/// than [`MAX_IMAGE_OR_LINK_URL_LEN`], fails to parse, or uses a scheme other
/// than `http`/`https`. The error string never echoes the rejected URL back
/// (reflected-XSS guard).
pub fn validate_image_or_link_url(s: &str) -> Result<(), String> {
    if s.chars().count() > MAX_IMAGE_OR_LINK_URL_LEN {
        return Err(format!(
            "URL must be at most {} characters",
            MAX_IMAGE_OR_LINK_URL_LEN
        ));
    }
    let parsed = url::Url::parse(s).map_err(|_| "Invalid URL".to_string())?;
    match parsed.scheme() {
        "https" | "http" => Ok(()),
        _ => Err("URL scheme must be http or https".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_https_url() {
        assert!(validate_image_or_link_url("https://example.com/logo.png").is_ok());
    }

    #[test]
    fn accepts_plain_http_url() {
        // Unlike the SSRF fetch validator, the XSS scheme check accepts plain
        // http:// (and localhost) — these URLs are only ever echoed to a
        // browser, never fetched server-side.
        assert!(validate_image_or_link_url("http://localhost:3000/img.png").is_ok());
    }

    #[test]
    fn rejects_javascript_scheme() {
        assert_eq!(
            validate_image_or_link_url("javascript:alert(1)"),
            Err("URL scheme must be http or https".to_string())
        );
    }

    #[test]
    fn rejects_data_scheme() {
        assert_eq!(
            validate_image_or_link_url("data:text/html,<script>alert(1)</script>"),
            Err("URL scheme must be http or https".to_string())
        );
    }

    #[test]
    fn rejects_file_scheme() {
        assert_eq!(
            validate_image_or_link_url("file:///etc/passwd"),
            Err("URL scheme must be http or https".to_string())
        );
    }

    #[test]
    fn rejects_unparseable_url() {
        assert_eq!(
            validate_image_or_link_url("not a url"),
            Err("Invalid URL".to_string())
        );
    }

    #[test]
    fn rejects_over_length_url() {
        let long = format!(
            "https://example.com/{}",
            "a".repeat(MAX_IMAGE_OR_LINK_URL_LEN)
        );
        assert_eq!(
            validate_image_or_link_url(&long),
            Err(format!(
                "URL must be at most {} characters",
                MAX_IMAGE_OR_LINK_URL_LEN
            ))
        );
    }

    #[test]
    fn accepts_url_at_length_boundary() {
        // Exactly MAX_IMAGE_OR_LINK_URL_LEN characters must pass (> is the cutoff).
        let base = "https://example.com/";
        let pad = MAX_IMAGE_OR_LINK_URL_LEN - base.chars().count();
        let at_limit = format!("{}{}", base, "a".repeat(pad));
        assert_eq!(at_limit.chars().count(), MAX_IMAGE_OR_LINK_URL_LEN);
        assert!(validate_image_or_link_url(&at_limit).is_ok());
    }
}
