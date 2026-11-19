//! Public listing slug helpers.
//!
//! The reality portal exposes listings at `/listings/{slug}`. Historically the
//! `slug` field on a listing summary was just the listing UUID stringified
//! (`RichListingSummary` set `slug: id.to_string()`), which is neither
//! human-readable nor SEO-friendly. Listings have no dedicated `slug` column,
//! so we derive a cosmetic slug from the title and append the UUID as a stable,
//! collision-free suffix: `beautiful-apartment-<uuid>`.
//!
//! The UUID suffix is what makes the slug resolvable without a DB lookup table:
//! [`parse_listing_id`] extracts it so the detail handler can keep serving by a
//! bare UUID (backward-compatible deep links) *and* by the new title-prefixed
//! slug. The title prefix is purely decorative — only the trailing UUID is
//! authoritative — so a renamed listing never breaks an old link.

use uuid::Uuid;

/// A canonical UUID string is 36 chars: `8-4-4-4-12` hex groups with hyphens.
const UUID_LEN: usize = 36;

/// Derive a URL-friendly slug for a listing: `{slugified-title}-{uuid}`.
///
/// When the title slugifies to nothing (e.g. it was only punctuation or
/// non-ASCII), the slug degrades to the bare UUID so the result is still a
/// valid, resolvable path segment.
pub fn listing_slug(title: &str, id: Uuid) -> String {
    let base = slugify(title);
    if base.is_empty() {
        id.to_string()
    } else {
        format!("{base}-{id}")
    }
}

/// Resolve a listing path segment back to its UUID.
///
/// Accepts either:
/// - a bare UUID (`<uuid>`) — the legacy/opaque form, still emitted by older
///   clients and deep links; or
/// - a title-prefixed slug (`some-title-<uuid>`) produced by [`listing_slug`].
///
/// Returns `None` when no UUID can be recovered, letting the caller answer
/// `404` rather than surfacing a parse error.
pub fn parse_listing_id(segment: &str) -> Option<Uuid> {
    // Fast path: the whole segment is a UUID (legacy / bare-id links).
    if let Ok(id) = Uuid::parse_str(segment) {
        return Some(id);
    }
    // Slug path: the UUID is the trailing 36 chars, preceded by a '-'.
    let start = segment.len().checked_sub(UUID_LEN)?;
    if start == 0 {
        // No room for a title prefix + separator; the bare-UUID path above
        // already covered the exact-length case.
        return None;
    }
    // The char immediately before the candidate UUID must be the separator we
    // inserted, so arbitrary 36-char tails can't be mistaken for a suffix.
    if segment.as_bytes().get(start - 1) != Some(&b'-') {
        return None;
    }
    Uuid::parse_str(&segment[start..]).ok()
}

/// Lowercase an ASCII-word slug: alphanumerics kept, every other run collapsed
/// to a single hyphen, no leading/trailing hyphens.
///
/// Non-ASCII characters (e.g. Slovak diacritics `ž`, `č`) are dropped rather
/// than transliterated — acceptable here because the slug's title portion is
/// cosmetic and the trailing UUID carries the identity. Mirrors the private
/// `slugify` in `accounting-server`'s company-creation handler; kept local to
/// avoid widening a low-priority fix into a cross-crate `common` extraction.
fn slugify(input: &str) -> String {
    input
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_cleans_titles() {
        assert_eq!(slugify("Beautiful Apartment"), "beautiful-apartment");
        assert_eq!(slugify("  Hello  World  "), "hello-world");
        assert_eq!(
            slugify("3-izbový byt, Bratislava!"),
            "3-izbov-byt-bratislava"
        );
        assert_eq!(slugify("---"), "");
    }

    #[test]
    fn listing_slug_appends_uuid() {
        let id = Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap();
        assert_eq!(
            listing_slug("Beautiful Apartment", id),
            "beautiful-apartment-11111111-2222-3333-4444-555555555555"
        );
    }

    #[test]
    fn listing_slug_degrades_to_bare_uuid_when_title_empty() {
        let id = Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap();
        assert_eq!(listing_slug("...", id), id.to_string());
        assert_eq!(listing_slug("", id), id.to_string());
    }

    #[test]
    fn parse_accepts_bare_uuid() {
        let id = Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap();
        assert_eq!(parse_listing_id(&id.to_string()), Some(id));
    }

    #[test]
    fn parse_accepts_title_prefixed_slug() {
        let id = Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap();
        let slug = listing_slug("Beautiful Apartment", id);
        assert_eq!(parse_listing_id(&slug), Some(id));
    }

    #[test]
    fn parse_roundtrips_degraded_slug() {
        // A title that slugifies to empty yields a bare UUID, which must parse.
        let id = Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap();
        let slug = listing_slug("!!!", id);
        assert_eq!(parse_listing_id(&slug), Some(id));
    }

    #[test]
    fn parse_rejects_garbage() {
        assert_eq!(parse_listing_id("not-a-listing"), None);
        assert_eq!(parse_listing_id(""), None);
        // A 36-char non-UUID tail without our '-' separator must not match.
        assert_eq!(
            parse_listing_id("abcdefghij-klmnopqrstuvwxyz0123456789"),
            None
        );
    }

    #[test]
    fn parse_rejects_bad_uuid_suffix() {
        // Correct shape/length but not valid hex groups.
        assert_eq!(
            parse_listing_id("title-zzzzzzzz-2222-3333-4444-555555555555"),
            None
        );
    }
}
