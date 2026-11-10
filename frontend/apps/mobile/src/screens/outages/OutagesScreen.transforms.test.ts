/**
 * Unit tests for the pure OutagesScreen transforms (UC-12).
 *
 * `narrow`, `toUiOutage` and `formatRange` are the data-shaping layer between
 * the raw `/api/v1/outages` payload and what the card renderer trusts. They are
 * exercised here in isolation (no RN tree is mounted) because a regression in
 * any of them silently mis-renders a card — or, for unexpected server enum
 * values, flows an out-of-range string into `commodityIcon` / `severityColor`
 * and crashes the switch-based renderers.
 *
 * TZ is pinned to UTC by jest.config.js, so the absolute date/time assertions
 * in `formatRange` are stable across CI runners.
 */

import { type ApiOutage, formatRange, narrow, toUiOutage } from './OutagesScreen';

describe('narrow', () => {
  const allowed = ['a', 'b', 'c'] as const;

  it('passes a value that is in the allow-list straight through', () => {
    expect(narrow('b', allowed, 'a')).toBe('b');
  });

  it('falls back for a value that is NOT in the allow-list (unexpected server enum)', () => {
    // The whole point of narrow: an unknown server string must never reach the
    // switch-based renderers — it is replaced by the known-safe fallback.
    expect(narrow('plasma', allowed, 'a')).toBe('a');
  });

  it('falls back for null', () => {
    expect(narrow(null, allowed, 'c')).toBe('c');
  });

  it('falls back for undefined', () => {
    expect(narrow(undefined, allowed, 'c')).toBe('c');
  });

  it('falls back for the empty string', () => {
    // `value && …` short-circuits on '' — '' is not a member of the allow-list.
    expect(narrow('', allowed, 'b')).toBe('b');
  });

  it('is case-sensitive — a differently-cased value is treated as unknown', () => {
    expect(narrow('B', allowed, 'a')).toBe('a');
  });

  it('matches exactly — surrounding whitespace is treated as unknown', () => {
    // `includes` is an exact-equality check, so a padded value is not a member.
    expect(narrow(' a ', allowed, 'b')).toBe('b');
  });

  it('returns the matched member itself, not the fallback, when present', () => {
    // Guards against an inverted branch that would always return the fallback.
    expect(narrow('c', allowed, 'a')).toBe('c');
  });
});

describe('toUiOutage', () => {
  const full: ApiOutage = {
    id: 'o-1',
    commodity: 'electricity',
    title: 'Power maintenance',
    description: 'Scheduled grid upgrade',
    status: 'in_progress',
    severity: 'high',
    starts_at: '2026-03-15T09:00:00Z',
    ends_at: '2026-03-15T11:00:00Z',
  };

  it('maps a fully-populated payload (snake_case -> camelCase, fields preserved)', () => {
    expect(toUiOutage(full)).toEqual({
      id: 'o-1',
      commodity: 'electricity',
      title: 'Power maintenance',
      description: 'Scheduled grid upgrade',
      status: 'in_progress',
      severity: 'high',
      startsAt: '2026-03-15T09:00:00Z',
      endsAt: '2026-03-15T11:00:00Z',
    });
  });

  it('narrows an unknown commodity to the safe fallback (water)', () => {
    expect(toUiOutage({ ...full, commodity: 'plasma' }).commodity).toBe('water');
  });

  it('narrows an unknown status to the safe fallback (scheduled)', () => {
    expect(toUiOutage({ ...full, status: 'cancelled' }).status).toBe('scheduled');
  });

  it('narrows an unknown severity to the safe fallback (low)', () => {
    expect(toUiOutage({ ...full, severity: 'catastrophic' }).severity).toBe('low');
  });

  it('coerces a null description to an empty string', () => {
    expect(toUiOutage({ ...full, description: null }).description).toBe('');
  });

  it('coerces an undefined description to an empty string', () => {
    const { description, ...rest } = full;
    expect(toUiOutage(rest).description).toBe('');
  });

  it('coerces a null severity to the low fallback', () => {
    expect(toUiOutage({ ...full, severity: null }).severity).toBe('low');
  });

  it('coerces an undefined severity to the low fallback', () => {
    const { severity, ...rest } = full;
    expect(toUiOutage(rest).severity).toBe('low');
  });

  it('narrows an empty-string commodity to the safe fallback (water)', () => {
    // Empty string is a falsy, non-member value: the `value && …` guard in
    // `narrow` must route it to the fallback rather than the renderers.
    expect(toUiOutage({ ...full, commodity: '' }).commodity).toBe('water');
  });

  it('narrows an empty-string status to the safe fallback (scheduled)', () => {
    expect(toUiOutage({ ...full, status: '' }).status).toBe('scheduled');
  });

  it('coerces an empty-string severity to the low fallback', () => {
    expect(toUiOutage({ ...full, severity: '' }).severity).toBe('low');
  });

  it('preserves id and title verbatim even when the enum fields are narrowed away', () => {
    // Narrowing must only touch the enum fields — identity/display fields pass
    // through untouched regardless of bad enum values elsewhere in the payload.
    const ui = toUiOutage({ ...full, id: 'o-42', title: 'Lift outage', commodity: 'plasma' });
    expect(ui.id).toBe('o-42');
    expect(ui.title).toBe('Lift outage');
  });

  it('keeps every produced enum field within its respective allow-list', () => {
    // Guards the renderers: commodity/status/severity must always be renderable.
    const ui = toUiOutage({
      ...full,
      commodity: '???',
      status: '???',
      severity: '???',
    });
    expect(['water', 'electricity', 'gas', 'heat', 'internet']).toContain(ui.commodity);
    expect(['scheduled', 'in_progress', 'resolved']).toContain(ui.status);
    expect(['low', 'medium', 'high']).toContain(ui.severity);
  });
});

describe('formatRange', () => {
  it('renders a same-day range as "date · time – time"', () => {
    const out = formatRange('2026-03-15T09:00:00Z', '2026-03-15T11:30:00Z');
    // Same-day branch is the only one that emits the middot separator.
    expect(out).toContain('·');
    expect(out).toContain('–');
    expect(typeof out).toBe('string');
    // TZ is pinned to UTC and the default locale resolves to 'en', so the
    // absolute date is stable across CI runners (file header). Time rendering
    // (12h vs 24h, AM/PM) is left to the locale, so only the date is pinned.
    expect(out).toMatch(/^Mar 15 · /);
  });

  it('renders a multi-day range as "date – date" (no middot, no times)', () => {
    const out = formatRange('2026-03-15T09:00:00Z', '2026-03-18T11:30:00Z');
    expect(out).not.toContain('·');
    expect(out).toContain('–');
    // Dates-only branch is fully deterministic under UTC + 'en'.
    expect(out).toBe('Mar 15 – Mar 18');
  });

  it('treats a sub-day range crossing no calendar boundary as same-day', () => {
    const out = formatRange('2026-03-15T00:30:00Z', '2026-03-15T23:30:00Z');
    expect(out).toContain('·');
  });

  it('does not throw on an empty start/end string (returns a string)', () => {
    // `new Date('')` is Invalid Date; the helpers must degrade to a string
    // rather than crash the card render.
    expect(() => formatRange('', '')).not.toThrow();
    expect(typeof formatRange('', '')).toBe('string');
  });

  it('does not throw when only one endpoint is an invalid date', () => {
    expect(() => formatRange('2026-03-15T09:00:00Z', 'not-a-date')).not.toThrow();
    expect(() => formatRange('not-a-date', '2026-03-15T09:00:00Z')).not.toThrow();
  });

  it('does not throw on an inverted range (end before start)', () => {
    expect(() => formatRange('2026-03-18T09:00:00Z', '2026-03-15T09:00:00Z')).not.toThrow();
  });
});
