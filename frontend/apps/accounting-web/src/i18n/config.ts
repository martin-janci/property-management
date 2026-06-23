/**
 * i18n configuration for the Accounting Portal public surface.
 *
 * Market-first locales: cs + sk (the target markets). en/de are roadmap
 * (PAP-303 §3). Default is `cs` — the largest addressable iDoklad-style
 * market — with `as-needed` prefixing so `/` serves cs and `/sk/...` serves sk.
 */

export const locales = ['cs', 'sk'] as const;
export type Locale = (typeof locales)[number];

export const defaultLocale: Locale = 'cs';

export const localeNames: Record<Locale, string> = {
  cs: 'Čeština',
  sk: 'Slovenčina',
};

export const localeFlags: Record<Locale, string> = {
  cs: '🇨🇿',
  sk: '🇸🇰',
};

/**
 * PLACEHOLDER brand identity — final product name + domain are a board
 * decision (PAP-303 open decision #1, "name ≠ iDoklad"). Centralised here so
 * a rename is a one-line change. Do not hard-code the name elsewhere.
 */
export const BRAND_NAME = 'Fakto';
