/**
 * Kill-switch HTML renderer tests (N8).
 *
 * Pins the locale-coercion behaviour and the basic markup invariants.
 * The actual 503 wiring is asserted in `middleware.ts` — exercising that
 * end-to-end requires running Next, which is out of scope here. See the
 * top of `kill-switch.ts` for the manual `curl -i` verification plan.
 */

import { describe, expect, it } from 'vitest';
import { normalizeKillSwitchLocale, renderKillSwitchHtml } from './kill-switch';

describe('normalizeKillSwitchLocale', () => {
  it('passes through known locales', () => {
    expect(normalizeKillSwitchLocale('sk')).toBe('sk');
    expect(normalizeKillSwitchLocale('cs')).toBe('cs');
    expect(normalizeKillSwitchLocale('de')).toBe('de');
    expect(normalizeKillSwitchLocale('pl')).toBe('pl');
    expect(normalizeKillSwitchLocale('hu')).toBe('hu');
    expect(normalizeKillSwitchLocale('en')).toBe('en');
  });

  it('peels region tags off (sk-SK -> sk)', () => {
    expect(normalizeKillSwitchLocale('sk-SK')).toBe('sk');
    expect(normalizeKillSwitchLocale('de-AT')).toBe('de');
  });

  it('falls back to en for unknown / null', () => {
    expect(normalizeKillSwitchLocale(null)).toBe('en');
    expect(normalizeKillSwitchLocale(undefined)).toBe('en');
    expect(normalizeKillSwitchLocale('')).toBe('en');
    expect(normalizeKillSwitchLocale('jp')).toBe('en');
  });
});

describe('renderKillSwitchHtml', () => {
  it('renders a complete document with noindex', () => {
    const html = renderKillSwitchHtml('en');
    expect(html.startsWith('<!doctype html>')).toBe(true);
    expect(html).toContain('<meta name="robots" content="noindex"');
    expect(html).toContain('<title>Service temporarily unavailable</title>');
  });

  it('localizes the title', () => {
    expect(renderKillSwitchHtml('sk')).toContain('Služba je dočasne nedostupná');
    expect(renderKillSwitchHtml('cs')).toContain('Služba je dočasně nedostupná');
    expect(renderKillSwitchHtml('de')).toContain('Dienst vorübergehend nicht verfügbar');
  });

  it('escapes the locale into the html lang attribute (no injection)', () => {
    // normalizeKillSwitchLocale already coerces to a known value, but
    // the integration with middleware passes whatever the API hands back.
    // Confirm an unknown / hostile string falls to `en`.
    expect(renderKillSwitchHtml('"><script>alert(1)</script>')).toContain('lang="en"');
  });
});
