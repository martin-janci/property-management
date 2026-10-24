/**
 * Scheme-allowlist tests for `sanitizeHref` (stored-XSS defense for
 * agency-/listing-authored URLs — public agency profile website link and
 * listing Resources virtual-tour / floor-plan links).
 *
 * IG3: the `javascript:` cases below fail (return the raw URL) without the
 * scheme check in `sanitize-url.ts`.
 */

import { describe, expect, it } from 'vitest';
import { sanitizeHref } from './sanitize-url';

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
    expect(sanitizeHref('HTTPS://Agency.Example.com/Path')).toBe(
      'HTTPS://Agency.Example.com/Path',
    );
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
