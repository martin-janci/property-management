/// <reference types="vitest/globals" />
/**
 * Unit tests for the pure cron helpers exported from CronPicker —
 * isValidCron, detectPreset, buildCron. Covers issue #714.
 *
 * These helpers run on the client to gate the "Save schedule" button
 * before round-tripping the expression to the backend. A drift here
 * lets an invalid expression through to the server (rejected late) or
 * misclassifies a valid one as custom (forces the user into the raw
 * field unnecessarily). Both are silent UX regressions, hence the
 * round-trip coverage below.
 */

import { describe, expect, it } from 'vitest';
import { buildCron, detectPreset, isValidCron } from './CronPicker';

// ──────────────────────────────────────────────────────────────
// isValidCron — accept/reject contract
// ──────────────────────────────────────────────────────────────

describe('isValidCron', () => {
  describe('valid expressions', () => {
    it.each([
      '* * * * *',
      '0 8 * * *', // daily 08:00
      '30 9 * * 1', // weekly Mon 09:30
      '0 0 1 * *', // monthly day-1 midnight
      '*/5 * * * *', // every 5 minutes
      '0 */2 * * *', // every 2 hours
      '0 9-17 * * 1-5', // weekdays 09:00–17:00 hourly
      '0,15,30,45 * * * *', // quarter-hour list
      '0 0 * * 7', // dow=7 (Sunday) accepted alongside 0
    ])('accepts %s', (expr) => {
      expect(isValidCron(expr)).toBe(true);
    });
  });

  describe('rejects malformed input', () => {
    it.each([
      ['', 'empty'],
      ['0 0 0', 'too few fields'],
      ['0 0 0 0 0 0', 'too many fields'],
      ['60 0 * * *', 'minute > 59'],
      ['0 24 * * *', 'hour > 23'],
      ['0 0 32 * *', 'dom > 31'],
      ['0 0 0 * *', 'dom < 1'],
      ['0 0 * 13 *', 'month > 12'],
      ['0 0 * 0 *', 'month < 1'],
      ['0 0 * * 8', 'dow > 7'],
      ['*/0 * * * *', 'step = 0 (would divide by zero)'],
      ['*/-1 * * * *', 'negative step'],
      ['*/abc * * * *', 'non-numeric step'],
      ['5-2 * * * *', 'reversed range'],
      ['abc * * * *', 'non-numeric field'],
      ['0/1/2 * * * *', 'malformed step (extra /)'],
    ])('rejects %s (%s)', (expr) => {
      expect(isValidCron(expr)).toBe(false);
    });
  });
});

// ──────────────────────────────────────────────────────────────
// detectPreset — classification + custom fallback
// ──────────────────────────────────────────────────────────────

describe('detectPreset', () => {
  it.each([
    ['0 8 * * *', 'daily'],
    ['0 0 * * *', 'daily'],
    ['30 14 * * 0', 'weekly'], // Sunday 14:30
    ['0 9 * * 6', 'weekly'], // Saturday 09:00
    ['0 0 1 * *', 'monthly'], // 1st of month
    ['15 17 15 * *', 'monthly'], // 15th 17:15
  ])('classifies %s as %s', (expr, expected) => {
    expect(detectPreset(expr)).toBe(expected);
  });

  it.each([
    ['0 9-17 * * 1-5', 'business-hours range'],
    ['0 0 1,15 * *', 'two-days-per-month list'],
    ['0 0 1 6 *', 'specific month'],
    ['0 0 1 * 1', 'dom AND dow both set (cron AND-semantics)'],
    ['0 0 0', 'wrong field count'],
  ])('falls back to "custom" for %s (%s)', (expr) => {
    expect(detectPreset(expr)).toBe('custom');
  });

  // detectPreset deliberately only inspects fields 3/4/5 (dom/month/dow);
  // it cannot tell '*/5 * * * *' from '0 8 * * *' because both have
  // dom=month=dow='*'. Both classify as 'daily', and parseCronTime falls
  // back to defaults for non-numeric minute/hour fields. This is the
  // documented current behaviour — the picker can't represent minute
  // granularity. The round-trip test below is what guards the lossless
  // cases; if detectPreset ever tightens this, update the docstring and
  // the parse helpers together.
  it('classifies dom=month=dow=* as daily even with non-numeric minute/hour', () => {
    expect(detectPreset('*/5 * * * *')).toBe('daily');
    expect(detectPreset('0 */2 * * *')).toBe('daily');
  });
});

// ──────────────────────────────────────────────────────────────
// Round-trip: detectPreset(buildCron(preset, …)) === preset
// ──────────────────────────────────────────────────────────────
//
// This is the load-bearing contract the issue called out specifically.
// If the two helpers ever drift (a new preset added in one but not the
// other, or a field-ordering change in buildCron), the picker would
// silently round-trip a preset selection through "custom" and back to
// a confused state.

describe('buildCron / detectPreset round-trip', () => {
  it('daily preset round-trips', () => {
    const expr = buildCron('daily', '30', '8', '1', '1');
    expect(expr).toBe('30 8 * * *');
    expect(detectPreset(expr)).toBe('daily');
    expect(isValidCron(expr)).toBe(true);
  });

  it('weekly preset round-trips (each day of week)', () => {
    for (const dow of ['0', '1', '2', '3', '4', '5', '6']) {
      const expr = buildCron('weekly', '0', '9', dow, '1');
      expect(detectPreset(expr)).toBe('weekly');
      expect(isValidCron(expr)).toBe(true);
    }
  });

  it('monthly preset round-trips (each day of month)', () => {
    for (const dom of ['1', '15', '28', '31']) {
      const expr = buildCron('monthly', '0', '6', '1', dom);
      expect(detectPreset(expr)).toBe('monthly');
      expect(isValidCron(expr)).toBe(true);
    }
  });

  it('custom preset builds to empty string (component falls back to raw input)', () => {
    expect(buildCron('custom', '0', '8', '1', '1')).toBe('');
  });
});
