/**
 * Branding sanitizer — N7 hardening tests.
 *
 * The sanitizer is the last line of defense between an agency-controlled
 * `css_vars` row and inline `style=` attributes on the public site. These
 * tests pin the denied / accepted shapes so a future "loosen the regex"
 * refactor does not silently reopen XSS / SSRF vectors.
 */

import { describe, expect, it } from 'vitest';
import {
  brandingToStyleObject,
  isSafeCssVarName,
  sanitizeCssValue,
  type TenantBranding,
} from './tenant-config';

const baseBranding: TenantBranding = {
  logo_url: null,
  primary_color: null,
  secondary_color: null,
  accent_color: null,
  font_family: null,
  css_vars: {},
};

describe('sanitizeCssValue', () => {
  it('sanitizes url() out', () => {
    expect(sanitizeCssValue('url(https://evil.com/x.png)')).toBeNull();
    // Whitespace-tricks should not bypass the denylist.
    expect(sanitizeCssValue('url ( //evil.com )')).toBeNull();
    expect(sanitizeCssValue('URL(//evil.com)')).toBeNull();
  });

  it('denies expression()', () => {
    expect(sanitizeCssValue('expression(alert(1))')).toBeNull();
    expect(sanitizeCssValue('Expression( document.cookie )')).toBeNull();
  });

  it('denies javascript:', () => {
    expect(sanitizeCssValue('javascript:alert(1)')).toBeNull();
    expect(sanitizeCssValue('JavaScript:alert(1)')).toBeNull();
  });

  it('denies data: even for image subtypes', () => {
    expect(sanitizeCssValue('data:image/png;base64,iVBOR')).toBeNull();
    expect(sanitizeCssValue('data:text/html,<script>alert(1)</script>')).toBeNull();
  });

  it('still rejects structural breakouts', () => {
    expect(sanitizeCssValue('red; background: url(//evil.com)')).toBeNull();
    expect(sanitizeCssValue('"><script>')).toBeNull();
    expect(sanitizeCssValue('a\nb')).toBeNull();
  });

  it('accepts hex colors when the var name signals a color', () => {
    expect(sanitizeCssValue('#fff', '--ppt-color-primary')).toBe('#fff');
    expect(sanitizeCssValue('#1A2B3C', '--ppt-color-primary')).toBe('#1A2B3C');
    expect(sanitizeCssValue('#11223344', '--ppt-background-soft')).toBe('#11223344');
  });

  it('accepts rgb()/rgba()/hsl()/hsla() for color-shaped vars', () => {
    expect(sanitizeCssValue('rgb(10, 20, 30)', '--ppt-color-accent')).toBe('rgb(10, 20, 30)');
    expect(sanitizeCssValue('rgba(0,0,0,0.5)', '--ppt-color-accent')).toBe('rgba(0,0,0,0.5)');
    expect(sanitizeCssValue('hsl(200,50%,40%)', '--ppt-color-accent')).toBe('hsl(200,50%,40%)');
    expect(sanitizeCssValue('hsla(200,50%,40%,0.8)', '--ppt-border-light')).toBe(
      'hsla(200,50%,40%,0.8)'
    );
  });

  it('rejects non-color values for color-shaped vars', () => {
    // No var() indirection — could chain to an unsanitized declaration.
    expect(sanitizeCssValue('var(--anything)', '--ppt-color-primary')).toBeNull();
    // Bare lengths / numbers do not belong in a color slot.
    expect(sanitizeCssValue('12px', '--ppt-color-primary')).toBeNull();
    expect(sanitizeCssValue('#zz', '--ppt-color-primary')).toBeNull();
  });

  it('accepts non-color values when the var name is not color-shaped', () => {
    expect(sanitizeCssValue('8px', '--ppt-radius-md')).toBe('8px');
    expect(sanitizeCssValue('"Inter", sans-serif')).toBeNull(); // quote breaks out
    expect(sanitizeCssValue('Inter, sans-serif', '--ppt-font-family')).toBe('Inter, sans-serif');
  });

  it('caps value length at 200 chars', () => {
    expect(sanitizeCssValue('a'.repeat(201))).toBeNull();
    expect(sanitizeCssValue('a'.repeat(200))).toBe('a'.repeat(200));
  });
});

describe('isSafeCssVarName', () => {
  it('accepts kebab-case `--ppt-*` tokens', () => {
    expect(isSafeCssVarName('--ppt-color-primary')).toBe(true);
    expect(isSafeCssVarName('--radius-md')).toBe(true);
  });

  it('rejects upper-case, missing prefix, or oversize names', () => {
    expect(isSafeCssVarName('color')).toBe(false);
    expect(isSafeCssVarName('--PPT-Color')).toBe(false);
    expect(isSafeCssVarName(`--${'a'.repeat(70)}`)).toBe(false);
  });
});

describe('brandingToStyleObject — end-to-end', () => {
  it('drops a url(...) css_var while keeping safe siblings', () => {
    const style = brandingToStyleObject({
      ...baseBranding,
      primary_color: '#abcdef',
      css_vars: {
        '--ppt-radius-md': '8px',
        '--ppt-bg-image': 'url(//evil.com/x.png)',
      },
    });
    expect(style).toEqual({
      '--ppt-color-primary': '#abcdef',
      '--ppt-radius-md': '8px',
    });
  });

  it('drops a non-color value placed in a color-shaped var', () => {
    const style = brandingToStyleObject({
      ...baseBranding,
      css_vars: {
        '--ppt-color-evil': 'var(--anything)',
        '--ppt-color-good': '#123',
      },
    });
    expect(style).toEqual({ '--ppt-color-good': '#123' });
  });
});
