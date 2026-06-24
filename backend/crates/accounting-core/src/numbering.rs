//! Invoice/document numbering format helpers (UC-ACC-02.3). OWNED BY: BE-Config.
//!
//! PURE formatting only — the atomic sequence allocation lives in the DB
//! (`db::repositories::acc_config::AccConfigRepository::allocate_next_number_rls`),
//! which is the only place that guarantees gapless numbers under concurrency.
//! This module turns a `(format, prefix, year, seq)` tuple into the printed
//! document number. The split is deliberate: gaplessness is a *legal* property
//! that can only be guaranteed by an atomic `UPDATE ... RETURNING` in one
//! transaction, while *formatting* is a deterministic pure function we can unit
//! test without a database.

use crate::errors::{AccountingError, Result};

/// Parameters resolved from an `acc_numbering_series` row plus the allocated seq.
#[derive(Debug, Clone)]
pub struct NumberParts<'a> {
    /// Series prefix, e.g. "INV".
    pub prefix: &'a str,
    /// Year the series resets on.
    pub year: i32,
    /// The sequence value allocated atomically by the DB.
    pub seq: i64,
}

/// Render a document number from a format template and parts (UC-ACC-02.3).
///
/// Supported placeholders inside `{...}`:
/// - `{prefix}` → the series prefix verbatim.
/// - `{year}`   → the 4-digit year.
/// - `{seq}`    → the allocated sequence, no padding.
/// - `{seq:0N}` → the sequence zero-padded to width `N` (e.g. `{seq:04}` → `0007`).
///
/// Any literal text outside braces is preserved. A `{{` / `}}` escapes a literal
/// brace. Returns [`AccountingError::Numbering`] on a malformed template (an
/// unterminated `{`, an unknown placeholder, or a non-numeric pad width).
///
/// # Examples
/// `"{prefix}{year}{seq:04}"` + `("INV", 2026, 7)` → `"INV20260007"`.
/// `"{prefix}-{year}-{seq:05}"` + `("FA", 2026, 123)` → `"FA-2026-00123"`.
pub fn format_number(format: &str, parts: &NumberParts<'_>) -> Result<String> {
    // UC-ACC-02.3: the printed number must be deterministic for a given
    // (format, prefix, year, seq). We hand-roll a tiny tokenizer over the
    // template so we control padding semantics exactly (no third-party fmt).
    let mut out = String::with_capacity(format.len() + 8);
    let mut chars = format.char_indices().peekable();

    while let Some((_, c)) = chars.next() {
        match c {
            '{' => {
                // Escaped literal brace: "{{" -> "{".
                if let Some(&(_, '{')) = chars.peek() {
                    chars.next();
                    out.push('{');
                    continue;
                }
                // Collect the placeholder body up to the closing '}'.
                let mut token = String::new();
                let mut closed = false;
                for (_, pc) in chars.by_ref() {
                    if pc == '}' {
                        closed = true;
                        break;
                    }
                    token.push(pc);
                }
                if !closed {
                    return Err(AccountingError::Numbering(format!(
                        "unterminated placeholder in numbering format: {format:?}"
                    )));
                }
                expand_placeholder(&token, parts, &mut out)?;
            }
            '}' => {
                // Escaped literal brace: "}}" -> "}". A lone '}' is malformed.
                if let Some(&(_, '}')) = chars.peek() {
                    chars.next();
                    out.push('}');
                } else {
                    return Err(AccountingError::Numbering(format!(
                        "unexpected '}}' in numbering format: {format:?}"
                    )));
                }
            }
            other => out.push(other),
        }
    }

    Ok(out)
}

/// Expand a single placeholder body (the text between `{` and `}`).
fn expand_placeholder(token: &str, parts: &NumberParts<'_>, out: &mut String) -> Result<()> {
    match token {
        "prefix" => out.push_str(parts.prefix),
        "year" => out.push_str(&parts.year.to_string()),
        "seq" => out.push_str(&parts.seq.to_string()),
        _ => {
            // Only the zero-padded sequence form `seq:0N` is otherwise supported.
            if let Some(spec) = token.strip_prefix("seq:") {
                let width = parse_pad_width(spec)?;
                // Negative sequences are not expected (allocation starts at 1),
                // but format defensively to keep the width contract.
                out.push_str(&format!("{:0width$}", parts.seq, width = width));
            } else {
                return Err(AccountingError::Numbering(format!(
                    "unknown placeholder {{{token}}} in numbering format"
                )));
            }
        }
    }
    Ok(())
}

/// Parse a `seq` pad spec such as `04` (leading zero + digits) or plain digits.
fn parse_pad_width(spec: &str) -> Result<usize> {
    // Accept "04", "4", "004" — the leading zero is the conventional pad marker
    // but we treat the whole spec as the desired field width.
    let digits = spec.trim_start_matches('0');
    let width: usize = if digits.is_empty() {
        // All zeros, e.g. "00" -> width 2 (length of the spec).
        spec.len()
    } else {
        spec.parse().map_err(|_| {
            AccountingError::Numbering(format!("invalid seq pad width: {spec:?}"))
        })?
    };
    Ok(width)
}

/// Whether a series should reset for `current_year` given its last-issued
/// `series_year` (yearly-reset cadence, UC-ACC-02.3). A new series row per year
/// is the actual reset mechanism (the unique `(tenant_id, doc_type, year)` key);
/// this helper expresses the rollover predicate for callers that decide whether
/// to create the next year's series.
pub fn should_reset_for_year(series_year: i32, current_year: i32) -> bool {
    current_year > series_year
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(prefix: &str, year: i32, seq: i64) -> NumberParts<'_> {
        NumberParts { prefix, year, seq }
    }

    #[test]
    fn formats_default_template() {
        // UC-ACC-02.3 default template from migration 00194.
        let p = parts("INV", 2026, 7);
        assert_eq!(format_number("{prefix}{year}{seq:04}", &p).unwrap(), "INV20260007");
    }

    #[test]
    fn formats_with_separators_and_wider_pad() {
        let p = parts("FA", 2026, 123);
        assert_eq!(
            format_number("{prefix}-{year}-{seq:05}", &p).unwrap(),
            "FA-2026-00123"
        );
    }

    #[test]
    fn unpadded_seq_and_empty_prefix() {
        let p = parts("", 2026, 42);
        assert_eq!(format_number("{year}/{seq}", &p).unwrap(), "2026/42");
    }

    #[test]
    fn seq_exceeding_pad_width_is_not_truncated() {
        // Gapless numbering must never lose digits: a 5-digit seq with {seq:04}
        // still prints all 5 digits.
        let p = parts("INV", 2026, 12345);
        assert_eq!(format_number("{prefix}{seq:04}", &p).unwrap(), "INV12345");
    }

    #[test]
    fn escaped_braces_are_literal() {
        let p = parts("INV", 2026, 1);
        assert_eq!(format_number("{{{prefix}}}", &p).unwrap(), "{INV}");
    }

    #[test]
    fn literal_text_preserved() {
        let p = parts("X", 2026, 9);
        assert_eq!(
            format_number("Invoice {prefix} no {seq:03} ({year})", &p).unwrap(),
            "Invoice X no 009 (2026)"
        );
    }

    #[test]
    fn unterminated_placeholder_errors() {
        let p = parts("INV", 2026, 1);
        assert!(matches!(
            format_number("{prefix}{seq:04", &p),
            Err(AccountingError::Numbering(_))
        ));
    }

    #[test]
    fn unknown_placeholder_errors() {
        let p = parts("INV", 2026, 1);
        assert!(matches!(
            format_number("{month}", &p),
            Err(AccountingError::Numbering(_))
        ));
    }

    #[test]
    fn lone_closing_brace_errors() {
        let p = parts("INV", 2026, 1);
        assert!(matches!(
            format_number("seq}", &p),
            Err(AccountingError::Numbering(_))
        ));
    }

    #[test]
    fn invalid_pad_width_errors() {
        let p = parts("INV", 2026, 1);
        assert!(matches!(
            format_number("{seq:0a}", &p),
            Err(AccountingError::Numbering(_))
        ));
    }

    #[test]
    fn yearly_reset_predicate() {
        // UC-ACC-02.3 yearly reset rolls over when the calendar year advances.
        assert!(should_reset_for_year(2025, 2026));
        assert!(!should_reset_for_year(2026, 2026));
        assert!(!should_reset_for_year(2026, 2025));
    }
}
