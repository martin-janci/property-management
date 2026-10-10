/**
 * Regression: meter-reading input must reject / normalize non-numeric text
 * (code-review-mobile-rn-meter-reading-no-numeric-validation).
 *
 * Before the fix, MeterReadingScreen only checked `reading.trim()` truthiness,
 * so "abc", "1.2.3", "12,,3" and " " were queued verbatim to
 * POST /api/v1/meter-readings — a silent write of garbage. These pure-function
 * tests lock the two guards that prevent it. They intentionally avoid rendering
 * (no jest-expo dependency), so they stay runnable even while the RN render
 * suite is blocked by #2951.
 */
import { isValidReading, normalizeReadingInput } from './meterReadingInput';

describe('normalizeReadingInput', () => {
  it.each([
    ['', ''],
    ['123', '123'],
    ['12.34', '12.34'],
    // comma (sk/cs/de/hu/pl decimal separator) normalizes to a dot
    ['12,34', '12.34'],
    // letters and symbols are stripped
    ['abc', ''],
    ['12abc', '12'],
    ['1a2b3', '123'],
    ['$12.50', '12.50'],
    ['  42  ', '42'],
    // only the first separator survives — "1.2.3" was previously queued as-is
    ['1.2.3', '1.23'],
    ['12,,3', '12.3'],
    ['1,2.3', '1.23'],
    // emoji / unicode junk dropped
    ['12💧5', '125'],
    ['-5', '5'],
  ])('normalizes %j -> %j', (raw, expected) => {
    expect(normalizeReadingInput(raw)).toBe(expected);
  });

  it('produces output that always parses to a finite number (or is empty/".")', () => {
    for (const raw of ['abc', '1.2.3.4', 'x9y8', '99,,,9', '   7   ']) {
      const out = normalizeReadingInput(raw);
      if (out !== '' && out !== '.') {
        expect(Number.isFinite(Number.parseFloat(out))).toBe(true);
      }
    }
  });
});

describe('isValidReading', () => {
  it.each(['123', '12.34', '0', '0.0', '  42  ', '1000000'])(
    'accepts a valid non-negative number: %j',
    (value) => {
      expect(isValidReading(value)).toBe(true);
    }
  );

  it.each([
    ['empty', ''],
    ['whitespace', '   '],
    ['bare dot', '.'],
    ['letters', 'abc'],
    ['mixed', '12abc'],
    ['negative', '-5'],
  ])('rejects invalid input (%s): %j', (_label, value) => {
    expect(isValidReading(value)).toBe(false);
  });

  it('rejects a raw value that normalization would clean out', () => {
    // "abc" -> "" -> invalid: the two guards agree.
    expect(isValidReading(normalizeReadingInput('abc'))).toBe(false);
  });
});
