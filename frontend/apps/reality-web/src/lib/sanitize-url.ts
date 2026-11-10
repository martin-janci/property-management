/**
 * Client-safe URL/href sanitization.
 *
 * Sibling of `sanitize-css.ts`: agency- and listing-authored URLs (website,
 * virtual-tour, floor-plan links) flow straight from tenant input into an
 * `href` attribute. Without a scheme check a `javascript:` / `data:` /
 * `vbscript:` URL becomes a stored-XSS vector the moment a visitor clicks the
 * link. This module enforces an http(s)-only scheme allowlist on any such
 * attacker-controlled URL before it reaches an `href`.
 *
 * Like `sanitize-css.ts`, this file MUST stay free of server-only imports
 * (`next/headers`, React `cache`, etc.) — it is pulled into both the server
 * and client bundles.
 */

/** Schemes that are safe to emit into an `href`. */
const ALLOWED_SCHEMES = new Set(['http', 'https']);

/**
 * Validate an agency-/listing-authored URL before it lands in an `href`.
 *
 * Returns the original string when it is safe to render, or `null` when it
 * must be dropped. Callers render nothing (or a plain-text fallback) on `null`
 * rather than emitting a suspicious link.
 *
 * Defense details:
 * - Browsers ignore ASCII whitespace and control characters (incl. tab,
 *   newline, CR, NUL) while parsing the scheme, so `java\tscript:` /
 *   `java\nscript:` / leading-space tricks all resolve to `javascript:`.
 *   We strip every char in `\u0000- ` before matching the scheme.
 * - When an explicit `scheme:` prefix is present, it MUST be `http` or
 *   `https`. Everything else (`javascript:`, `data:`, `vbscript:`,
 *   `mailto:`, `file:`, …) is rejected.
 * - A value with no scheme (protocol-relative `//host`, or a path-relative
 *   link) cannot execute script, so it is allowed through unchanged.
 */
export function sanitizeHref(url: string | null | undefined): string | null {
  if (typeof url !== 'string') return null;

  const trimmed = url.trim();
  if (trimmed === '') return null;

  // Strip characters a browser discards when determining the scheme — every
  // code point in C0 + space (<= 0x20), including tab / newline / CR / NUL.
  // Done via a code-point filter rather than a regex so no literal control
  // character appears in source (biome `noControlCharactersInRegex`).
  const stripped = Array.from(trimmed)
    .filter((ch) => ch.charCodeAt(0) > 0x20)
    .join('')
    .toLowerCase();

  // `scheme:` = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." ) ":"  (RFC 3986).
  const schemeMatch = /^([a-z][a-z0-9+.-]*):/.exec(stripped);
  if (schemeMatch) {
    return ALLOWED_SCHEMES.has(schemeMatch[1]) ? trimmed : null;
  }

  // No explicit scheme — relative / protocol-relative. Not a script vector.
  return trimmed;
}
