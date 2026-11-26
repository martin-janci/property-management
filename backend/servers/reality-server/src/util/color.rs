//! Shared CSS hex-color hygiene for portal-rendered branding fields.
//!
//! Agency branding holds user-supplied colors (`primary_color`,
//! `secondary_color`) that the frontend later injects into inline `style`
//! attributes / CSS custom properties. An unvalidated value is both a data
//! hygiene problem (garbage colors silently stored) and a stored-content risk
//! (a non-color string ending up in a `style="color:…"` sink). We accept only
//! a canonical CSS hex color — a leading `#` followed by exactly 3, 4, 6, or 8
//! ASCII hex digits (`#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`) — and reject
//! everything else with a generic, caller-facing message.
//!
//! Deliberately narrow: named colors (`red`), `rgb()`/`hsl()` functions and
//! CSS keywords are **not** accepted. The branding UI emits hex, so keeping the
//! accepted set to hex removes the ambiguity (and the injection surface) that a
//! free-form color string would carry.

/// A canonical hex color is `#` plus 3, 4, 6, or 8 hex digits.
const VALID_HEX_LENGTHS: [usize; 4] = [3, 4, 6, 8];

/// Validate a user-supplied CSS hex color (e.g. `#1a2b3c`).
///
/// Returns `Err` with a generic, caller-facing message when the value is not a
/// `#`-prefixed string of exactly 3, 4, 6, or 8 ASCII hex digits. The error
/// string never echoes the rejected value back (reflected-content guard).
pub fn validate_hex_color(s: &str) -> Result<(), String> {
    let digits = s
        .strip_prefix('#')
        .ok_or_else(|| "Color must be a hex value starting with '#'".to_string())?;

    if !VALID_HEX_LENGTHS.contains(&digits.len()) {
        return Err(
            "Color must be a hex value with 3, 4, 6, or 8 digits (e.g. #1a2b3c)".to_string(),
        );
    }

    if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(
            "Color must be a hex value with 3, 4, 6, or 8 digits (e.g. #1a2b3c)".to_string(),
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_six_digit_hex() {
        assert!(validate_hex_color("#1a2b3c").is_ok());
        assert!(validate_hex_color("#FFFFFF").is_ok());
        assert!(validate_hex_color("#000000").is_ok());
    }

    #[test]
    fn accepts_three_four_and_eight_digit_hex() {
        assert!(validate_hex_color("#fff").is_ok()); // shorthand rgb
        assert!(validate_hex_color("#abcd").is_ok()); // rgba shorthand
        assert!(validate_hex_color("#1a2b3c4d").is_ok()); // rrggbbaa
    }

    #[test]
    fn rejects_missing_hash() {
        assert!(validate_hex_color("1a2b3c").is_err());
        assert!(validate_hex_color("ffffff").is_err());
    }

    #[test]
    fn rejects_named_color() {
        // Named CSS colors are intentionally not accepted.
        assert!(validate_hex_color("red").is_err());
        assert!(validate_hex_color("rebeccapurple").is_err());
    }

    #[test]
    fn rejects_non_hex_digits() {
        assert!(validate_hex_color("#gggggg").is_err());
        assert!(validate_hex_color("#12345z").is_err());
    }

    #[test]
    fn rejects_wrong_length() {
        assert!(validate_hex_color("#12").is_err()); // 2 digits
        assert!(validate_hex_color("#12345").is_err()); // 5 digits
        assert!(validate_hex_color("#").is_err()); // 0 digits
    }

    #[test]
    fn rejects_injection_attempt() {
        // A value that would be dangerous in a `style="color:…"` sink.
        assert!(validate_hex_color("#fff;background:url(javascript:alert(1))").is_err());
        assert!(validate_hex_color("red\");}").is_err());
    }
}
