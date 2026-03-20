/**
 * Validation utilities for Property Management apps.
 * Provides common validation functions for forms and data.
 */

// ============================================
// Types
// ============================================

export interface ValidationResult {
  valid: boolean;
  error?: string;
}

export type Validator<T = string> = (value: T) => ValidationResult;

// ============================================
// Email Validation
// ============================================

/**
 * RFC 5322 compliant email regex pattern.
 * Matches most common email formats.
 */
const EMAIL_REGEX = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

/**
 * Validates an email address format.
 *
 * @example
 * isValidEmail('user@example.com') // true
 * isValidEmail('invalid-email') // false
 */
export function isValidEmail(email: string): boolean {
  return EMAIL_REGEX.test(email.trim());
}

/**
 * Email validator with error message.
 */
export function validateEmail(email: string): ValidationResult {
  if (!email.trim()) {
    return { valid: false, error: 'Email is required' };
  }
  if (!isValidEmail(email)) {
    return { valid: false, error: 'Invalid email format' };
  }
  return { valid: true };
}

// ============================================
// Phone Validation
// ============================================

/**
 * Phone regex for common formats.
 * Accepts: +421123456789, 0123456789, +421 123 456 789, etc.
 */
const PHONE_REGEX = /^\+?[\d\s\-()]{7,20}$/;

/**
 * Validates a phone number format.
 */
export function isValidPhone(phone: string): boolean {
  const normalized = phone.replace(/[\s\-()]/g, '');
  return PHONE_REGEX.test(phone) && normalized.length >= 7;
}

/**
 * Phone validator with error message.
 */
export function validatePhone(phone: string): ValidationResult {
  if (!phone.trim()) {
    return { valid: false, error: 'Phone number is required' };
  }
  if (!isValidPhone(phone)) {
    return { valid: false, error: 'Invalid phone number format' };
  }
  return { valid: true };
}

// ============================================
// Password Validation
// ============================================

export interface PasswordRequirements {
  minLength?: number;
  requireUppercase?: boolean;
  requireLowercase?: boolean;
  requireNumbers?: boolean;
  requireSpecialChars?: boolean;
}

const DEFAULT_PASSWORD_REQUIREMENTS: Required<PasswordRequirements> = {
  minLength: 8,
  requireUppercase: true,
  requireLowercase: true,
  requireNumbers: true,
  requireSpecialChars: false,
};

/**
 * Validates password strength against requirements.
 */
export function validatePassword(
  password: string,
  requirements: PasswordRequirements = {}
): ValidationResult {
  const reqs = { ...DEFAULT_PASSWORD_REQUIREMENTS, ...requirements };
  const errors: string[] = [];

  if (password.length < reqs.minLength) {
    errors.push(`Must be at least ${reqs.minLength} characters`);
  }
  if (reqs.requireUppercase && !/[A-Z]/.test(password)) {
    errors.push('Must contain an uppercase letter');
  }
  if (reqs.requireLowercase && !/[a-z]/.test(password)) {
    errors.push('Must contain a lowercase letter');
  }
  if (reqs.requireNumbers && !/\d/.test(password)) {
    errors.push('Must contain a number');
  }
  if (reqs.requireSpecialChars && !/[!@#$%^&*(),.?":{}|<>]/.test(password)) {
    errors.push('Must contain a special character');
  }

  if (errors.length > 0) {
    return { valid: false, error: errors.join('. ') };
  }
  return { valid: true };
}

/**
 * Calculates password strength score (0-4).
 * 0 = very weak, 4 = very strong
 */
export function getPasswordStrength(password: string): number {
  let score = 0;

  if (password.length >= 8) score++;
  if (password.length >= 12) score++;
  if (/[a-z]/.test(password) && /[A-Z]/.test(password)) score++;
  if (/\d/.test(password)) score++;
  if (/[!@#$%^&*(),.?":{}|<>]/.test(password)) score++;

  return Math.min(score, 4);
}

// ============================================
// URL Validation
// ============================================

/**
 * Validates a URL format.
 */
export function isValidUrl(url: string): boolean {
  try {
    new URL(url);
    return true;
  } catch {
    return false;
  }
}

/**
 * URL validator with error message.
 */
export function validateUrl(url: string): ValidationResult {
  if (!url.trim()) {
    return { valid: false, error: 'URL is required' };
  }
  if (!isValidUrl(url)) {
    return { valid: false, error: 'Invalid URL format' };
  }
  return { valid: true };
}

// ============================================
// Numeric Validation
// ============================================

export interface NumberRangeOptions {
  min?: number;
  max?: number;
  integer?: boolean;
}

/**
 * Validates a number within a range.
 */
export function validateNumber(
  value: number | string,
  options: NumberRangeOptions = {}
): ValidationResult {
  const num = typeof value === 'string' ? Number.parseFloat(value) : value;

  if (Number.isNaN(num)) {
    return { valid: false, error: 'Must be a valid number' };
  }

  if (options.integer && !Number.isInteger(num)) {
    return { valid: false, error: 'Must be a whole number' };
  }

  if (options.min !== undefined && num < options.min) {
    return { valid: false, error: `Must be at least ${options.min}` };
  }

  if (options.max !== undefined && num > options.max) {
    return { valid: false, error: `Must be at most ${options.max}` };
  }

  return { valid: true };
}

// ============================================
// String Validation
// ============================================

export interface StringLengthOptions {
  minLength?: number;
  maxLength?: number;
  pattern?: RegExp;
  patternMessage?: string;
}

/**
 * Validates string length and format.
 */
export function validateString(value: string, options: StringLengthOptions = {}): ValidationResult {
  const { minLength, maxLength, pattern, patternMessage } = options;

  if (minLength !== undefined && value.length < minLength) {
    return { valid: false, error: `Must be at least ${minLength} characters` };
  }

  if (maxLength !== undefined && value.length > maxLength) {
    return { valid: false, error: `Must be at most ${maxLength} characters` };
  }

  if (pattern && !pattern.test(value)) {
    return { valid: false, error: patternMessage || 'Invalid format' };
  }

  return { valid: true };
}

/**
 * Validates that a required field has a value.
 */
export function validateRequired(value: unknown): ValidationResult {
  if (value === null || value === undefined) {
    return { valid: false, error: 'This field is required' };
  }

  if (typeof value === 'string' && !value.trim()) {
    return { valid: false, error: 'This field is required' };
  }

  if (Array.isArray(value) && value.length === 0) {
    return { valid: false, error: 'At least one item is required' };
  }

  return { valid: true };
}

// ============================================
// Date Validation
// ============================================

/**
 * Validates that a date is in the future.
 */
export function validateFutureDate(date: Date | string): ValidationResult {
  const d = typeof date === 'string' ? new Date(date) : date;

  if (Number.isNaN(d.getTime())) {
    return { valid: false, error: 'Invalid date' };
  }

  if (d <= new Date()) {
    return { valid: false, error: 'Date must be in the future' };
  }

  return { valid: true };
}

/**
 * Validates that a date is in the past.
 */
export function validatePastDate(date: Date | string): ValidationResult {
  const d = typeof date === 'string' ? new Date(date) : date;

  if (Number.isNaN(d.getTime())) {
    return { valid: false, error: 'Invalid date' };
  }

  if (d >= new Date()) {
    return { valid: false, error: 'Date must be in the past' };
  }

  return { valid: true };
}

/**
 * Validates a date range (end after start).
 */
export function validateDateRange(
  startDate: Date | string,
  endDate: Date | string
): ValidationResult {
  const start = typeof startDate === 'string' ? new Date(startDate) : startDate;
  const end = typeof endDate === 'string' ? new Date(endDate) : endDate;

  if (Number.isNaN(start.getTime()) || Number.isNaN(end.getTime())) {
    return { valid: false, error: 'Invalid date' };
  }

  if (end <= start) {
    return { valid: false, error: 'End date must be after start date' };
  }

  return { valid: true };
}

// ============================================
// Composite Validation
// ============================================

/**
 * Combines multiple validators, returning the first error.
 *
 * @example
 * const validator = combineValidators(
 *   (v) => validateRequired(v),
 *   (v) => validateEmail(v)
 * );
 * validator(''); // { valid: false, error: 'This field is required' }
 */
export function combineValidators<T>(...validators: Validator<T>[]): Validator<T> {
  return (value: T): ValidationResult => {
    for (const validator of validators) {
      const result = validator(value);
      if (!result.valid) {
        return result;
      }
    }
    return { valid: true };
  };
}

// ============================================
// Specialized Validators
// ============================================

/**
 * Validates a Slovak/Czech postal code (5 digits, optionally with space).
 */
export function isValidPostalCode(postalCode: string): boolean {
  const normalized = postalCode.replace(/\s/g, '');
  return /^\d{5}$/.test(normalized);
}

/**
 * Validates a Slovak/Czech company ID (ICO) - 8 digits.
 */
export function isValidCompanyId(ico: string): boolean {
  const normalized = ico.replace(/\s/g, '');
  return /^\d{8}$/.test(normalized);
}

/**
 * Validates a Slovak/Czech VAT ID (DIC).
 */
export function isValidVatId(dic: string): boolean {
  const normalized = dic.replace(/\s/g, '').toUpperCase();
  // Slovak: SK + 10 digits, Czech: CZ + 8-10 digits
  return /^(SK\d{10}|CZ\d{8,10})$/.test(normalized);
}

/**
 * Validates an IBAN.
 * Basic format check - does not validate checksum.
 */
export function isValidIban(iban: string): boolean {
  const normalized = iban.replace(/\s/g, '').toUpperCase();
  // IBAN: 2 letters + 2 digits + up to 30 alphanumeric
  return /^[A-Z]{2}\d{2}[A-Z0-9]{4,30}$/.test(normalized);
}
