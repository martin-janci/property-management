/**
 * Active-locale resolution for `Intl.*` / `toLocale*` formatting.
 *
 * Date and number formatters that are called without a locale fall back to the
 * JavaScript runtime default (usually `en-US` in the browser / CI), so a
 * Slovak, Czech or German manager would see US conventions regardless of the
 * language they picked. Passing the value returned here as the first argument
 * to `Intl.NumberFormat` / `Intl.DateTimeFormat` / `Number.prototype.toLocaleString`
 * / `Date.prototype.toLocaleDateString` keeps those outputs in sync with the
 * active UI language, including an immediate change on a runtime language switch.
 */

import i18n from 'i18next';

/**
 * Returns the locale used for formatting: the active i18n language first (so a
 * runtime language switch takes effect immediately), then the browser locale,
 * then `en-US`.
 */
export function getActiveLocale(): string {
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
