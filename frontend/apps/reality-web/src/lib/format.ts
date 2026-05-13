/**
 * Locale-aware formatting helpers.
 *
 * Replaces scattered `Number#toLocaleString('sk-SK')` /
 * `Intl.NumberFormat('en-US', {...})` call sites that all hardcoded a single
 * locale and produced visibly wrong output for the other five (DE buyers
 * seeing Slovak number formats, etc.).
 *
 * For client components prefer `useFormatter()` from next-intl when you
 * already have a hook scope; these helpers are convenient when you only
 * have a locale string (e.g. inside getTranslations / server-rendered
 * pages, or when threading a locale prop through a deep tree).
 *
 * Locale mapping intentionally uses the BCP-47 tag with the major region
 * (sk-SK, cs-CZ, …) so number-grouping + currency-formatting follow the
 * regional convention rather than the generic 2-letter language code.
 */

const BCP47: Record<string, string> = {
  en: 'en-GB',
  sk: 'sk-SK',
  cs: 'cs-CZ',
  de: 'de-DE',
  pl: 'pl-PL',
  hu: 'hu-HU',
};

function bcp47(locale: string): string {
  return BCP47[locale] ?? locale;
}

/**
 * Format a number with locale-correct grouping (e.g. `1 234` in SK,
 * `1,234` in EN, `1.234` in DE).
 */
export function formatNumber(
  value: number,
  locale: string,
  options?: Intl.NumberFormatOptions
): string {
  return new Intl.NumberFormat(bcp47(locale), options).format(value);
}

/**
 * Format a price. Defaults to EUR + no fraction digits — most reality
 * listings round to whole euros. Pass `options` to override.
 */
export function formatPrice(
  value: number,
  locale: string,
  options: Intl.NumberFormatOptions = {}
): string {
  return new Intl.NumberFormat(bcp47(locale), {
    style: 'currency',
    currency: 'EUR',
    maximumFractionDigits: 0,
    ...options,
  }).format(value);
}

/**
 * Format an area (m²). Some locales (DE) use a space between number and
 * unit, others (SK, CS) attach it directly — we keep a non-breaking space
 * which renders well across all six configured locales.
 */
export function formatArea(squareMeters: number, locale: string): string {
  return `${formatNumber(squareMeters, locale)} m²`;
}
