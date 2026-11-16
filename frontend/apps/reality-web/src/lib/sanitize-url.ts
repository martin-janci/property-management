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

/** Sentinel origin used to resolve candidate redirect paths. */
const REDIRECT_SENTINEL_ORIGIN = 'https://redirect.invalid';

/**
 * Validate a post-login / post-SSO-callback redirect target.
 *
 * Returns a same-origin, path-absolute location (always starting with a single
 * `/`) that is safe to hand to `router.replace`/`router.push`, or `'/'` when
 * the supplied value is unusable or attacker-controlled.
 *
 * Unlike {@link sanitizeHref} — which allows protocol-relative `//host` through
 * because it cannot execute script — a redirect target that leaves the current
 * origin is an *open-redirect* vector, so anything that resolves off-origin is
 * rejected.
 *
 * Bypasses defended (all of which slipped past the previous naive
 * `startsWith('/') && !startsWith('//')` check):
 * - `//evil.com` — protocol-relative absolute URL.
 * - `/\evil.com`, `/\/\evil.com`, `\/\/evil.com` — backslash variants. Browsers
 *   (and the WHATWG URL parser, for special schemes) normalise `\` to `/`,
 *   turning these into protocol-relative URLs to `evil.com`.
 * - `https://evil.com`, `javascript:…`, `data:…` — explicit scheme/authority.
 * - Leading ASCII control chars / whitespace the browser strips while parsing
 *   (`\t`, `\n`, `\r`, space, NUL, …), e.g. `"/\t/evil.com"`.
 *
 * A query string and fragment on an otherwise same-origin path are preserved.
 */
export function sanitizeRedirectPath(raw: string | null | undefined): string {
  if (typeof raw !== 'string') return '/';

  // Drop C0 controls + space (<= 0x20) the browser / URL parser would ignore,
  // so `"/\tevil.com"` or `"\u0000//evil.com"` can't slip past the guards below.
  // Done via a code-point filter (no literal control char in a regex — biome
  // `noControlCharactersInRegex`).
  const cleaned = Array.from(raw)
    .filter((ch) => ch.charCodeAt(0) > 0x20)
    .join('');

  // Must be path-absolute: a single leading `/`. Reject empty, relative refs,
  // and protocol-relative `//` up front.
  if (!cleaned.startsWith('/') || cleaned.startsWith('//')) return '/';

  // Resolve against a fixed sentinel origin. The WHATWG URL parser normalises
  // backslashes to slashes for special (http[s]) schemes, so `"/\evil.com"`
  // becomes `"//evil.com"` and resolves to the evil origin — caught here.
  let resolved: URL;
  try {
    resolved = new URL(cleaned, REDIRECT_SENTINEL_ORIGIN);
  } catch {
    return '/';
  }
  if (resolved.origin !== REDIRECT_SENTINEL_ORIGIN) return '/';

  // Re-emit path-only (never the sentinel host) with any query / fragment.
  const path = `${resolved.pathname}${resolved.search}${resolved.hash}`;
  return path.startsWith('/') ? path : '/';
}
