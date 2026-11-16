/**
 * Scheme-allowlist tests for `sanitizeHref` (stored-XSS defense for
 * agency-/listing-authored URLs — public agency profile website link and
 * listing Resources virtual-tour / floor-plan links).
 *
 * IG3: the `javascript:` cases below fail (return the raw URL) without the
 * scheme check in `sanitize-url.ts`.
 */

import { describe, expect, it } from 'vitest';
import { sanitizeHref, sanitizeRedirectPath } from './sanitize-url';

describe('sanitizeHref', () => {
  it('neutralizes a javascript: href', () => {
    expect(sanitizeHref('javascript:alert(1)')).toBeNull();
  });

  it('neutralizes case/whitespace/control-char obfuscated javascript: hrefs', () => {
    expect(sanitizeHref('JavaScript:alert(1)')).toBeNull();
    expect(sanitizeHref('  javascript:alert(1)')).toBeNull();
    expect(sanitizeHref('java\tscript:alert(1)')).toBeNull();
    expect(sanitizeHref('java\nscript:alert(1)')).toBeNull();
    expect(sanitizeHref('\u0000javascript:alert(1)')).toBeNull();
  });

  it('neutralizes other dangerous schemes', () => {
    expect(sanitizeHref('data:text/html,<script>alert(1)</script>')).toBeNull();
    expect(sanitizeHref('vbscript:msgbox(1)')).toBeNull();
    expect(sanitizeHref('file:///etc/passwd')).toBeNull();
    expect(sanitizeHref('mailto:evil@example.com')).toBeNull();
  });

  it('allows http(s) URLs through unchanged', () => {
    expect(sanitizeHref('https://agency.example.com')).toBe('https://agency.example.com');
    expect(sanitizeHref('http://tour.example.com/360')).toBe('http://tour.example.com/360');
    // Preserves original casing/content of a safe URL (only the scheme is checked).
    expect(sanitizeHref('HTTPS://Agency.Example.com/Path')).toBe('HTTPS://Agency.Example.com/Path');
  });

  it('allows scheme-less relative and protocol-relative URLs', () => {
    expect(sanitizeHref('//cdn.example.com/x')).toBe('//cdn.example.com/x');
    expect(sanitizeHref('/listings/foo')).toBe('/listings/foo');
    expect(sanitizeHref('example.com/path')).toBe('example.com/path');
  });

  it('drops empty / nullish input', () => {
    expect(sanitizeHref('')).toBeNull();
    expect(sanitizeHref('   ')).toBeNull();
    expect(sanitizeHref(null)).toBeNull();
    expect(sanitizeHref(undefined)).toBeNull();
  });
});

/**
 * Same-origin redirect-target validation for post-login / post-SSO-callback
 * navigation (open-redirect defense). Unlike `sanitizeHref`, a protocol-relative
 * `//host` target must be REJECTED here — it leaves the current origin.
 *
 * IG3: the backslash cases below ("/\\evil.com", "/\\/\\evil.com", and the
 * control-char-prefixed variants) pass the old naive
 * `startsWith('/') && !startsWith('//')` check but resolve to an off-origin
 * absolute URL in the browser — they only return '/' once the URL-origin check
 * in `sanitizeRedirectPath` lands.
 */
describe('sanitizeRedirectPath', () => {
  it('honours a genuine same-origin relative path', () => {
    expect(sanitizeRedirectPath('/dashboard/agency')).toBe('/dashboard/agency');
    expect(sanitizeRedirectPath('/')).toBe('/');
  });

  it('preserves query string and fragment on a same-origin path', () => {
    expect(sanitizeRedirectPath('/listings/foo?page=2#results')).toBe(
      '/listings/foo?page=2#results'
    );
  });

  it('rejects a protocol-relative target', () => {
    expect(sanitizeRedirectPath('//evil.com')).toBe('/');
    expect(sanitizeRedirectPath('//evil.com/path')).toBe('/');
  });

  it('rejects backslash-based protocol-relative bypasses', () => {
    // Browsers (and the WHATWG URL parser for special schemes) normalise "\" to
    // "/", so each of these is really a protocol-relative URL to evil.com.
    expect(sanitizeRedirectPath('/\\evil.com')).toBe('/');
    expect(sanitizeRedirectPath('/\\/\\evil.com')).toBe('/');
    expect(sanitizeRedirectPath('\\/\\/evil.com')).toBe('/');
    expect(sanitizeRedirectPath('/\\\\evil.com')).toBe('/');
  });

  it('rejects targets with leading control chars / whitespace that browsers strip', () => {
    expect(sanitizeRedirectPath('/\t/evil.com')).toBe('/');
    expect(sanitizeRedirectPath('\u0000//evil.com')).toBe('/');
    expect(sanitizeRedirectPath('  //evil.com')).toBe('/');
  });

  it('rejects absolute URLs and dangerous schemes', () => {
    expect(sanitizeRedirectPath('https://evil.com')).toBe('/');
    expect(sanitizeRedirectPath('http://evil.com/x')).toBe('/');
    expect(sanitizeRedirectPath('javascript:alert(1)')).toBe('/');
    expect(sanitizeRedirectPath('data:text/html,<script>alert(1)</script>')).toBe('/');
  });

  it('rejects relative (non-path-absolute) and empty / nullish input', () => {
    expect(sanitizeRedirectPath('listings/foo')).toBe('/');
    expect(sanitizeRedirectPath('')).toBe('/');
    expect(sanitizeRedirectPath('   ')).toBe('/');
    expect(sanitizeRedirectPath(null)).toBe('/');
    expect(sanitizeRedirectPath(undefined)).toBe('/');
  });
});
