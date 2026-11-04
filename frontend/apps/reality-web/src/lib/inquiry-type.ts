/**
 * Inquiry-type localization helper.
 *
 * The inquiries page used to render the raw `inquiry.type` enum via
 * `.replace('_', ' ')`, which left the label untranslated for sk/cs/de (and
 * every non-English) users. This routes the enum through next-intl with a
 * stable message key per value (`pages.inquiries.type.<value>`), falling back
 * to a humanized default for any value the frontend type doesn't know yet.
 */

import type { InquiryType } from '@ppt/reality-api-client';

/** The translation function shape we depend on — next-intl's `t`. */
type TranslateFn = (key: string) => string;

/**
 * Enum values that have a dedicated, stable message key under
 * `pages.inquiries.type.<value>` in every reality-web message catalog.
 *
 * Keep this in sync with `InquiryType` and with the `type` block in
 * `messages/*.json`. The i18n catalog completeness test
 * (`inquiry-type.test.ts`) pins that every value here resolves in every locale.
 */
export const KNOWN_INQUIRY_TYPES: readonly InquiryType[] = [
  'general',
  'viewing_request',
  'price_negotiation',
  'availability',
];

/** Humanize an unknown enum value: `some_unknown_value` -> `Some unknown value`. */
export function humanizeInquiryType(raw: string): string {
  const spaced = raw.replace(/_/g, ' ').trim();
  if (!spaced) {
    return raw;
  }
  return spaced.charAt(0).toUpperCase() + spaced.slice(1);
}

/**
 * Resolve a localized label for an inquiry type.
 *
 * `t` must be the `pages.inquiries`-scoped next-intl translation function.
 * Known enum values resolve via `t('type.<value>')` so sk/cs/de/en (and every
 * other catalog) render a translated label; any unknown value falls back to a
 * humanized default instead of surfacing the raw enum.
 */
export function inquiryTypeLabel(t: TranslateFn, type: string): string {
  if ((KNOWN_INQUIRY_TYPES as readonly string[]).includes(type)) {
    return t(`type.${type}`);
  }
  return humanizeInquiryType(type);
}
