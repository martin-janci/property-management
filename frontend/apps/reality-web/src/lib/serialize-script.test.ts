/**
 * XSS-hardening tests for serializeForScript.
 *
 * Pins the exact escapes that stop attacker-influenced data (per-request
 * tenant config, /env.js runtime config) from breaking out of an inline
 * <script> element. A regression here reopens an HTML-injection window,
 * so these assertions are deliberately literal.
 */

import { describe, expect, it } from 'vitest';
import { serializeForScript } from './serialize-script';

describe('serializeForScript', () => {
  it('escapes a </script> breakout attempt so the tag cannot close early', () => {
    const out = serializeForScript({ tenant_id: '</script><script>alert(1)</script>' });
    // The raw closing tag must not survive anywhere in the output.
    expect(out).not.toContain('</script>');
    expect(out).not.toContain('<script>');
    // '<' and '>' are escaped to their unicode forms.
    expect(out).toContain('\\u003c');
    expect(out).toContain('\\u003e');
  });

  it('escapes angle brackets and ampersand', () => {
    const out = serializeForScript({ a: '<', b: '>', c: '&' });
    expect(out).not.toMatch(/[<>&]/);
    expect(out).toContain('\\u003c');
    expect(out).toContain('\\u003e');
    expect(out).toContain('\\u0026');
  });

  it('escapes U+2028 and U+2029 line/paragraph separators', () => {
    const LS = String.fromCharCode(0x2028);
    const PS = String.fromCharCode(0x2029);
    const out = serializeForScript({ a: LS, b: PS });
    // Raw separators would be a SyntaxError inside a pre-ES2019 JS string.
    expect(out).not.toContain(LS);
    expect(out).not.toContain(PS);
    expect(out).toContain('\\u2028');
    expect(out).toContain('\\u2029');
  });

  it('leaves safe payloads byte-for-byte equal to JSON.stringify', () => {
    const value = { tenant_id: 'acme', feature_flags: { building_disabled: { enabled: false } } };
    expect(serializeForScript(value)).toBe(JSON.stringify(value));
  });

  it('produces output that still parses back to the original value', () => {
    const value = { tenant_id: '</script>', nested: ['<', '>', '&'] };
    // The escapes are valid JSON escapes, so JSON.parse round-trips them.
    expect(JSON.parse(serializeForScript(value))).toEqual(value);
  });
});
