/// <reference types="vitest/globals" />
/**
 * Tests for @ppt/shared validation utilities.
 *
 * Pure unit tests; no DOM or network. Hosted in ppt-web because
 * @ppt/shared has no test runner of its own.
 */

import {
  combineValidators,
  getPasswordStrength,
  isValidCompanyId,
  isValidEmail,
  isValidIban,
  isValidPhone,
  isValidPostalCode,
  isValidUrl,
  isValidVatId,
  validateDateRange,
  validateEmail,
  validateFutureDate,
  validateNumber,
  validatePassword,
  validatePastDate,
  validatePhone,
  validateRequired,
  validateString,
  validateUrl,
} from '@ppt/shared';

describe('@ppt/shared - validation', () => {
  describe('email', () => {
    it('accepts standard formats', () => {
      expect(isValidEmail('user@example.com')).toBe(true);
      expect(isValidEmail('first.last+tag@sub.example.co.uk')).toBe(true);
    });

    it('rejects malformed addresses', () => {
      expect(isValidEmail('invalid-email')).toBe(false);
      expect(isValidEmail('missing@tld')).toBe(false);
      expect(isValidEmail('@example.com')).toBe(false);
      expect(isValidEmail('user@')).toBe(false);
      expect(isValidEmail('a b@example.com')).toBe(false);
    });

    it('trims whitespace before validating', () => {
      expect(isValidEmail('  user@example.com  ')).toBe(true);
    });

    it('validateEmail returns specific error for empty input', () => {
      const r = validateEmail('   ');
      expect(r.valid).toBe(false);
      expect(r.error).toBe('Email is required');
    });

    it('validateEmail flags invalid format', () => {
      const r = validateEmail('not-an-email');
      expect(r.valid).toBe(false);
      expect(r.error).toBe('Invalid email format');
    });

    it('validateEmail succeeds without error on valid input', () => {
      expect(validateEmail('user@example.com')).toEqual({ valid: true });
    });
  });

  describe('phone', () => {
    it('accepts common international formats', () => {
      expect(isValidPhone('+421123456789')).toBe(true);
      expect(isValidPhone('+421 123 456 789')).toBe(true);
      expect(isValidPhone('0123456789')).toBe(true);
      expect(isValidPhone('(123) 456-7890')).toBe(true);
    });

    it('rejects too-short numbers', () => {
      expect(isValidPhone('123')).toBe(false);
      expect(isValidPhone('12345')).toBe(false);
    });

    it('rejects strings with non-phone characters', () => {
      expect(isValidPhone('not-a-phone')).toBe(false);
      expect(isValidPhone('123abc4567')).toBe(false);
    });

    it('validatePhone returns required error for empty input', () => {
      expect(validatePhone('')).toEqual({
        valid: false,
        error: 'Phone number is required',
      });
    });

    it('validatePhone flags invalid format', () => {
      expect(validatePhone('abc').valid).toBe(false);
    });
  });

  describe('password', () => {
    it('requires minimum length by default (8)', () => {
      const r = validatePassword('Aa1xxx');
      expect(r.valid).toBe(false);
      expect(r.error).toMatch(/at least 8/);
    });

    it('requires uppercase, lowercase, and number by default', () => {
      const all = validatePassword('alllower');
      expect(all.valid).toBe(false);
      // Concatenated by '. '
      expect(all.error).toMatch(/uppercase/);
      expect(all.error).toMatch(/number/);
    });

    it('passes with default requirements when satisfied', () => {
      expect(validatePassword('Strong1Pass')).toEqual({ valid: true });
    });

    it('honors custom requirements (special chars)', () => {
      const r = validatePassword('Strong1Pass', { requireSpecialChars: true });
      expect(r.valid).toBe(false);
      expect(r.error).toMatch(/special character/);
      expect(validatePassword('Strong1Pass!', { requireSpecialChars: true })).toEqual({
        valid: true,
      });
    });

    it('honors a relaxed minLength', () => {
      expect(
        validatePassword('Aa1', {
          minLength: 3,
          requireUppercase: true,
          requireLowercase: true,
          requireNumbers: true,
        })
      ).toEqual({ valid: true });
    });

    it('getPasswordStrength scores 0..4', () => {
      expect(getPasswordStrength('')).toBe(0);
      expect(getPasswordStrength('abcdefgh')).toBe(1); // length>=8
      expect(getPasswordStrength('Abcdefgh')).toBe(2); // mixed case
      expect(getPasswordStrength('Abcdefg1')).toBe(3); // mixed + digit
      expect(getPasswordStrength('Abcdefghij1!')).toBe(4); // length>=12 + everything
      // Caps at 4
      expect(getPasswordStrength('Abcdefghijklmn1!')).toBe(4);
    });
  });

  describe('url', () => {
    it('accepts well-formed URLs', () => {
      expect(isValidUrl('https://example.com')).toBe(true);
      expect(isValidUrl('http://localhost:3000/path?q=1')).toBe(true);
    });

    it('rejects strings without scheme', () => {
      expect(isValidUrl('example.com')).toBe(false);
      expect(isValidUrl('not a url')).toBe(false);
    });

    it('validateUrl rejects empty input with specific message', () => {
      expect(validateUrl('  ')).toEqual({
        valid: false,
        error: 'URL is required',
      });
    });

    it('validateUrl flags malformed URL', () => {
      expect(validateUrl('not-a-url').valid).toBe(false);
    });
  });

  describe('numbers', () => {
    it('parses string numbers', () => {
      expect(validateNumber('42')).toEqual({ valid: true });
      expect(validateNumber('3.14')).toEqual({ valid: true });
    });

    it('rejects NaN', () => {
      expect(validateNumber('abc').valid).toBe(false);
      expect(validateNumber(Number.NaN).valid).toBe(false);
    });

    it('enforces min and max', () => {
      expect(validateNumber(5, { min: 10 }).valid).toBe(false);
      expect(validateNumber(5, { max: 1 }).valid).toBe(false);
      expect(validateNumber(5, { min: 1, max: 10 })).toEqual({ valid: true });
    });

    it('integer flag rejects floats', () => {
      expect(validateNumber(3.14, { integer: true }).valid).toBe(false);
      expect(validateNumber(3, { integer: true })).toEqual({ valid: true });
    });
  });

  describe('strings', () => {
    it('enforces minLength/maxLength', () => {
      expect(validateString('ab', { minLength: 3 }).valid).toBe(false);
      expect(validateString('abcdef', { maxLength: 3 }).valid).toBe(false);
      expect(validateString('abc', { minLength: 1, maxLength: 5 })).toEqual({ valid: true });
    });

    it('enforces regex pattern with custom message', () => {
      const r = validateString('abc', {
        pattern: /^\d+$/,
        patternMessage: 'digits only',
      });
      expect(r).toEqual({ valid: false, error: 'digits only' });
    });

    it('falls back to default pattern message', () => {
      const r = validateString('abc', { pattern: /^\d+$/ });
      expect(r.error).toBe('Invalid format');
    });
  });

  describe('required', () => {
    it('rejects null and undefined', () => {
      expect(validateRequired(null).valid).toBe(false);
      expect(validateRequired(undefined).valid).toBe(false);
    });

    it('rejects whitespace-only strings', () => {
      expect(validateRequired('   ').valid).toBe(false);
    });

    it('rejects empty arrays', () => {
      const r = validateRequired([]);
      expect(r.valid).toBe(false);
      expect(r.error).toBe('At least one item is required');
    });

    it('accepts non-empty values', () => {
      expect(validateRequired('hi')).toEqual({ valid: true });
      expect(validateRequired([1])).toEqual({ valid: true });
      expect(validateRequired(0)).toEqual({ valid: true });
      expect(validateRequired(false)).toEqual({ valid: true });
    });
  });

  describe('dates', () => {
    it('validateFutureDate accepts a date in the future', () => {
      const future = new Date(Date.now() + 60_000);
      expect(validateFutureDate(future)).toEqual({ valid: true });
    });

    it('validateFutureDate rejects past or current dates', () => {
      const past = new Date(Date.now() - 60_000);
      expect(validateFutureDate(past).valid).toBe(false);
    });

    it('validateFutureDate rejects invalid date', () => {
      expect(validateFutureDate('not-a-date').valid).toBe(false);
    });

    it('validatePastDate accepts a date in the past', () => {
      expect(validatePastDate(new Date(Date.now() - 60_000))).toEqual({ valid: true });
    });

    it('validatePastDate rejects future dates', () => {
      expect(validatePastDate(new Date(Date.now() + 60_000)).valid).toBe(false);
    });

    it('validateDateRange accepts end after start', () => {
      const start = new Date(2026, 0, 1);
      const end = new Date(2026, 0, 2);
      expect(validateDateRange(start, end)).toEqual({ valid: true });
    });

    it('validateDateRange rejects equal or reversed dates', () => {
      const same = new Date(2026, 0, 1);
      expect(validateDateRange(same, same).valid).toBe(false);
      expect(validateDateRange(new Date(2026, 0, 2), new Date(2026, 0, 1)).valid).toBe(false);
    });

    it('validateDateRange rejects invalid input', () => {
      expect(validateDateRange('bad', new Date()).valid).toBe(false);
    });
  });

  describe('combineValidators', () => {
    it('runs in order and returns the first failure', () => {
      const v = combineValidators<string>(
        (s) => validateRequired(s),
        (s) => validateEmail(s)
      );
      expect(v('').error).toBe('This field is required');
      expect(v('not-email').error).toBe('Invalid email format');
      expect(v('user@example.com')).toEqual({ valid: true });
    });
  });

  describe('specialized validators', () => {
    it('postal code requires 5 digits, allows internal whitespace', () => {
      expect(isValidPostalCode('81101')).toBe(true);
      expect(isValidPostalCode('811 01')).toBe(true);
      expect(isValidPostalCode('1234')).toBe(false);
      expect(isValidPostalCode('123456')).toBe(false);
      expect(isValidPostalCode('abcde')).toBe(false);
    });

    it('company id (ICO) requires 8 digits', () => {
      expect(isValidCompanyId('12345678')).toBe(true);
      expect(isValidCompanyId('1234 5678')).toBe(true);
      expect(isValidCompanyId('1234567')).toBe(false);
      expect(isValidCompanyId('abcdefgh')).toBe(false);
    });

    it('VAT id (DIC) accepts SK10 and CZ8-10 formats', () => {
      expect(isValidVatId('SK1234567890')).toBe(true);
      expect(isValidVatId('sk1234567890')).toBe(true);
      expect(isValidVatId('CZ12345678')).toBe(true);
      expect(isValidVatId('CZ1234567890')).toBe(true);
      expect(isValidVatId('SK123')).toBe(false);
      expect(isValidVatId('XX12345678')).toBe(false);
    });

    it('IBAN matches basic structure (no checksum)', () => {
      expect(isValidIban('SK6807200002891987426353')).toBe(true);
      expect(isValidIban('sk68 0720 0002 8919 8742 6353')).toBe(true);
      expect(isValidIban('123456')).toBe(false);
      expect(isValidIban('SKAB12345678')).toBe(false); // letters in checksum slot
    });
  });
});
