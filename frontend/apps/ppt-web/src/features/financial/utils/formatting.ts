/**
 * Shared formatting utilities for financial components.
 *
 * Money and dates follow the active UI language (`i18n.language`) so a
 * Slovak/Czech/German… manager sees `1 234,50 €` / `15. 6. 2026` rather than
 * the US conventions. Formatters are created per call (and cached per locale)
 * so a runtime language switch takes effect immediately.
 */

import i18n from 'i18next';

/**
 * Returns the locale used for formatting: the active i18n language, then the
 * browser locale, then 'en-US'.
 */
export function getUserLocale(): string {
  if (i18n.language) {
    return i18n.language;
  }

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

const dateFormatterCache = new Map<string, Intl.DateTimeFormat>();

function getDateFormatter(
  variant: 'short' | 'long' | 'dayMonth',
  options: Intl.DateTimeFormatOptions
): Intl.DateTimeFormat {
  const locale = getUserLocale();
  const cacheKey = `${variant}|${locale}`;
  let formatter = dateFormatterCache.get(cacheKey);
  if (!formatter) {
    formatter = new Intl.DateTimeFormat(locale, options);
    dateFormatterCache.set(cacheKey, formatter);
  }
  return formatter;
}

/**
 * Formats a number as currency using the user's locale.
 */
export function formatCurrency(amount: number, currency = 'EUR'): string {
  return new Intl.NumberFormat(getUserLocale(), {
    style: 'currency',
    currency,
  }).format(amount);
}

/**
 * Formats a date string using the invoice date format (short month, day, year).
 */
export function formatDate(dateString: string): string {
  return getDateFormatter('short', { month: 'short', day: 'numeric', year: 'numeric' }).format(
    new Date(dateString)
  );
}

/**
 * Formats a date string as a short day + month (no year).
 */
export function formatDayMonth(dateString: string): string {
  return getDateFormatter('dayMonth', { month: 'short', day: 'numeric' }).format(
    new Date(dateString)
  );
}

/**
 * Formats a date string using the detailed date format (long month, day, year).
 */
export function formatDetailedDate(dateString: string): string {
  return getDateFormatter('long', { month: 'long', day: 'numeric', year: 'numeric' }).format(
    new Date(dateString)
  );
}
