import { describe, expect, it } from 'vitest';
import {
  type DateInput,
  formatDate,
  formatDateTime,
  formatDetailedDate,
  formatISODate,
  formatRelativeTime,
  formatTime,
} from './formatting';

describe('date formatters — invalid input', () => {
  // Inputs that produce an `Invalid Date` and previously made
  // Intl.DateTimeFormat#format / Date#toISOString throw a RangeError
  // ("Invalid time value").
  const invalidInputs: Array<[string, DateInput]> = [
    ['NaN', Number.NaN],
    ['unparseable string', 'not-a-real-date'],
    ['invalid Date object', new Date('nope')],
    // Untyped JS callers can pass through null/undefined despite the types.
    ['undefined', undefined as unknown as DateInput],
    ['null', null as unknown as DateInput],
  ];

  const formatters: Array<[string, (d: DateInput) => string]> = [
    ['formatDate', (d) => formatDate(d)],
    ['formatDetailedDate', (d) => formatDetailedDate(d)],
    ['formatDateTime', (d) => formatDateTime(d)],
    ['formatTime', (d) => formatTime(d)],
    ['formatISODate', (d) => formatISODate(d)],
    ['formatRelativeTime', (d) => formatRelativeTime(d)],
  ];

  for (const [fnName, fn] of formatters) {
    for (const [inputName, input] of invalidInputs) {
      it(`${fnName} returns the safe fallback (no throw) for ${inputName}`, () => {
        expect(() => fn(input)).not.toThrow();
        expect(fn(input)).toBe('');
      });
    }
  }
});

describe('date formatters — valid input still formats', () => {
  const date = new Date('2026-03-20T15:45:00.000Z');

  it('formatDate formats a valid date', () => {
    expect(formatDate(date, { locale: 'en-US' })).toContain('2026');
  });

  it('formatISODate returns the ISO day for a valid date', () => {
    expect(formatISODate(date)).toBe('2026-03-20');
  });

  it('formatRelativeTime returns a non-empty string for a valid date', () => {
    expect(formatRelativeTime(date, { locale: 'en-US' })).not.toBe('');
  });
});
