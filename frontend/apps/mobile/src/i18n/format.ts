/**
 * Locale-aware date/time formatting helpers.
 *
 * Historically many screens formatted dates with a hardcoded `'en-US'` locale
 * (an incomplete fix of #2282), so dates never localised for sk/cs/de/pl/hu
 * users regardless of the language they picked in-app. These helpers derive
 * the formatting locale from the app's *active* i18next language instead, so a
 * rendered date matches the rest of the UI.
 *
 * The active language is read from the shared i18next singleton (the same
 * instance configured in `./index`), so module-level formatters localise
 * without having to thread the language through every call site. An explicit
 * `language` override is accepted, mainly so the behaviour can be unit-tested
 * without mutating global i18next state.
 */
import i18n from 'i18next';
import { defaultLocale, locales } from './config';

/**
 * Map an i18next language (e.g. `'sk'`, `'de-DE'`) onto a BCP-47 locale to hand
 * to `Intl` / `toLocale*` APIs. Falls back to the default locale (`'en'`) for
 * unset or unsupported languages so formatting never throws or silently emits
 * a fixed `en-US`.
 */
export function resolveLocale(language?: string): string {
  const raw = language ?? i18n.language ?? defaultLocale;
  const base = raw.split('-')[0].toLowerCase();
  return (locales as readonly string[]).includes(base) ? base : defaultLocale;
}

/** `Date#toLocaleDateString` using the app's active i18n locale. */
export function formatDate(
  value: string | number | Date,
  options?: Intl.DateTimeFormatOptions,
  language?: string
): string {
  return new Date(value).toLocaleDateString(resolveLocale(language), options);
}

/** `Date#toLocaleTimeString` using the app's active i18n locale. */
export function formatTime(
  value: string | number | Date,
  options?: Intl.DateTimeFormatOptions,
  language?: string
): string {
  return new Date(value).toLocaleTimeString(resolveLocale(language), options);
}

/** `Date#toLocaleString` (date + time) using the app's active i18n locale. */
export function formatDateTime(
  value: string | number | Date,
  options?: Intl.DateTimeFormatOptions,
  language?: string
): string {
  return new Date(value).toLocaleString(resolveLocale(language), options);
}
