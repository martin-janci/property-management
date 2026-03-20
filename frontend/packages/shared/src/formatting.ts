/**
 * Formatting utilities for Property Management apps.
 * Provides locale-aware formatting for dates, times, currency, and numbers.
 */

// ============================================
// Locale Detection
// ============================================

/**
 * Detects and returns the user's locale for formatting.
 * Falls back to 'en-US' if locale cannot be determined.
 */
export function getUserLocale(): string {
  if (typeof navigator !== 'undefined' && navigator.language) {
    return navigator.language;
  }

  if (typeof Intl !== 'undefined' && typeof Intl.DateTimeFormat === 'function') {
    try {
      const resolved = new Intl.DateTimeFormat().resolvedOptions().locale;
      if (resolved) {
        return resolved;
      }
    } catch {
      // Ignore and fall through to default
    }
  }

  return 'en-US';
}

// ============================================
// Currency Formatting
// ============================================

export interface CurrencyFormatOptions {
  /** Currency code (e.g., 'EUR', 'USD', 'CZK'). Default: 'EUR' */
  currency?: string;
  /** Locale to use for formatting. Default: user's locale */
  locale?: string;
  /** Whether amount is in cents (divide by 100). Default: false */
  fromCents?: boolean;
}

/**
 * Formats a number as currency using the specified or user's locale.
 *
 * @example
 * formatCurrency(1234.56) // "€1,234.56" (with EUR)
 * formatCurrency(1234.56, { currency: 'USD' }) // "$1,234.56"
 * formatCurrency(123456, { fromCents: true }) // "€1,234.56"
 */
export function formatCurrency(
  amount: number,
  options: CurrencyFormatOptions = {}
): string {
  const { currency = 'EUR', locale = getUserLocale(), fromCents = false } = options;
  const value = fromCents ? amount / 100 : amount;

  return new Intl.NumberFormat(locale, {
    style: 'currency',
    currency,
  }).format(value);
}

/**
 * Formats a number as compact currency (e.g., "$1.2K", "€500K").
 */
export function formatCompactCurrency(
  amount: number,
  options: CurrencyFormatOptions = {}
): string {
  const { currency = 'EUR', locale = getUserLocale(), fromCents = false } = options;
  const value = fromCents ? amount / 100 : amount;

  return new Intl.NumberFormat(locale, {
    style: 'currency',
    currency,
    notation: 'compact',
    maximumFractionDigits: 1,
  }).format(value);
}

// ============================================
// Date Formatting
// ============================================

export type DateInput = string | Date | number;

export interface DateFormatOptions {
  /** Locale to use for formatting. Default: user's locale */
  locale?: string;
}

/**
 * Safely converts various date inputs to Date objects.
 */
function toDate(input: DateInput): Date {
  if (input instanceof Date) {
    return input;
  }
  return new Date(input);
}

/**
 * Formats a date using short format (e.g., "Mar 20, 2026").
 */
export function formatDate(
  date: DateInput,
  options: DateFormatOptions = {}
): string {
  const { locale = getUserLocale() } = options;

  return new Intl.DateTimeFormat(locale, {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  }).format(toDate(date));
}

/**
 * Formats a date using detailed format (e.g., "March 20, 2026").
 */
export function formatDetailedDate(
  date: DateInput,
  options: DateFormatOptions = {}
): string {
  const { locale = getUserLocale() } = options;

  return new Intl.DateTimeFormat(locale, {
    month: 'long',
    day: 'numeric',
    year: 'numeric',
  }).format(toDate(date));
}

/**
 * Formats a date with time (e.g., "Mar 20, 2026, 3:45 PM").
 */
export function formatDateTime(
  date: DateInput,
  options: DateFormatOptions = {}
): string {
  const { locale = getUserLocale() } = options;

  return new Intl.DateTimeFormat(locale, {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  }).format(toDate(date));
}

/**
 * Formats time only (e.g., "3:45 PM").
 */
export function formatTime(
  date: DateInput,
  options: DateFormatOptions = {}
): string {
  const { locale = getUserLocale() } = options;

  return new Intl.DateTimeFormat(locale, {
    hour: 'numeric',
    minute: '2-digit',
  }).format(toDate(date));
}

/**
 * Formats a date as ISO date string (YYYY-MM-DD).
 */
export function formatISODate(date: DateInput): string {
  return toDate(date).toISOString().split('T')[0];
}

/**
 * Formats a relative time (e.g., "2 hours ago", "in 3 days").
 */
export function formatRelativeTime(
  date: DateInput,
  options: DateFormatOptions = {}
): string {
  const { locale = getUserLocale() } = options;
  const now = Date.now();
  const then = toDate(date).getTime();
  const diffMs = then - now;
  const diffSecs = Math.round(diffMs / 1000);
  const diffMins = Math.round(diffSecs / 60);
  const diffHours = Math.round(diffMins / 60);
  const diffDays = Math.round(diffHours / 24);
  const diffWeeks = Math.round(diffDays / 7);
  const diffMonths = Math.round(diffDays / 30);
  const diffYears = Math.round(diffDays / 365);

  const rtf = new Intl.RelativeTimeFormat(locale, { numeric: 'auto' });

  if (Math.abs(diffSecs) < 60) {
    return rtf.format(diffSecs, 'second');
  }
  if (Math.abs(diffMins) < 60) {
    return rtf.format(diffMins, 'minute');
  }
  if (Math.abs(diffHours) < 24) {
    return rtf.format(diffHours, 'hour');
  }
  if (Math.abs(diffDays) < 7) {
    return rtf.format(diffDays, 'day');
  }
  if (Math.abs(diffWeeks) < 4) {
    return rtf.format(diffWeeks, 'week');
  }
  if (Math.abs(diffMonths) < 12) {
    return rtf.format(diffMonths, 'month');
  }
  return rtf.format(diffYears, 'year');
}

// ============================================
// Number Formatting
// ============================================

export interface NumberFormatOptions {
  /** Locale to use for formatting. Default: user's locale */
  locale?: string;
  /** Minimum fraction digits. Default: 0 */
  minimumFractionDigits?: number;
  /** Maximum fraction digits. Default: 2 */
  maximumFractionDigits?: number;
}

/**
 * Formats a number with locale-aware separators.
 *
 * @example
 * formatNumber(1234567.89) // "1,234,567.89"
 */
export function formatNumber(
  value: number,
  options: NumberFormatOptions = {}
): string {
  const {
    locale = getUserLocale(),
    minimumFractionDigits = 0,
    maximumFractionDigits = 2,
  } = options;

  return new Intl.NumberFormat(locale, {
    minimumFractionDigits,
    maximumFractionDigits,
  }).format(value);
}

/**
 * Formats a number as compact notation (e.g., "1.2K", "5M").
 */
export function formatCompactNumber(
  value: number,
  options: NumberFormatOptions = {}
): string {
  const { locale = getUserLocale(), maximumFractionDigits = 1 } = options;

  return new Intl.NumberFormat(locale, {
    notation: 'compact',
    maximumFractionDigits,
  }).format(value);
}

/**
 * Formats a number as percentage.
 *
 * @example
 * formatPercent(0.1234) // "12.34%"
 * formatPercent(0.5) // "50%"
 */
export function formatPercent(
  value: number,
  options: NumberFormatOptions = {}
): string {
  const {
    locale = getUserLocale(),
    minimumFractionDigits = 0,
    maximumFractionDigits = 2,
  } = options;

  return new Intl.NumberFormat(locale, {
    style: 'percent',
    minimumFractionDigits,
    maximumFractionDigits,
  }).format(value);
}

// ============================================
// Phone & Address Formatting
// ============================================

/**
 * Formats a phone number for display.
 * Handles various input formats and normalizes them.
 *
 * @example
 * formatPhone('+421123456789') // "+421 123 456 789"
 * formatPhone('0123456789', { countryCode: '+421' }) // "+421 123 456 789"
 */
export function formatPhone(
  phone: string,
  options: { countryCode?: string } = {}
): string {
  // Remove all non-digit characters except +
  let normalized = phone.replace(/[^\d+]/g, '');

  // Add country code if provided and not present
  if (options.countryCode && !normalized.startsWith('+')) {
    // Remove leading 0 if present
    if (normalized.startsWith('0')) {
      normalized = normalized.slice(1);
    }
    normalized = options.countryCode + normalized;
  }

  // Format with spaces for readability
  if (normalized.startsWith('+')) {
    // International format: +XXX XXX XXX XXX
    const digits = normalized.slice(1);
    if (digits.length >= 10) {
      const countryCodeLen = digits.length > 10 ? 3 : 2;
      const cc = digits.slice(0, countryCodeLen);
      const rest = digits.slice(countryCodeLen);
      const parts = rest.match(/.{1,3}/g) || [];
      return `+${cc} ${parts.join(' ')}`;
    }
  }

  // Fallback: just return with spaces every 3 digits
  const parts = normalized.match(/.{1,3}/g) || [];
  return parts.join(' ');
}

export interface AddressComponents {
  street?: string;
  streetNumber?: string;
  city?: string;
  postalCode?: string;
  country?: string;
}

/**
 * Formats address components into a single line.
 *
 * @example
 * formatAddress({ street: 'Main St', streetNumber: '123', city: 'Prague' })
 * // "Main St 123, Prague"
 */
export function formatAddress(components: AddressComponents): string {
  const parts: string[] = [];

  // Street with number
  if (components.street) {
    const streetLine = components.streetNumber
      ? `${components.street} ${components.streetNumber}`
      : components.street;
    parts.push(streetLine);
  }

  // City with postal code
  if (components.city) {
    const cityLine = components.postalCode
      ? `${components.postalCode} ${components.city}`
      : components.city;
    parts.push(cityLine);
  }

  // Country
  if (components.country) {
    parts.push(components.country);
  }

  return parts.join(', ');
}

/**
 * Formats a file size in bytes to human-readable format.
 *
 * @example
 * formatFileSize(1024) // "1 KB"
 * formatFileSize(1048576) // "1 MB"
 */
export function formatFileSize(bytes: number): string {
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let unitIndex = 0;
  let size = bytes;

  while (size >= 1024 && unitIndex < units.length - 1) {
    size /= 1024;
    unitIndex++;
  }

  return `${formatNumber(size, { maximumFractionDigits: unitIndex === 0 ? 0 : 1 })} ${units[unitIndex]}`;
}
