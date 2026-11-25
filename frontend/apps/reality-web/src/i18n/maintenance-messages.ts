/**
 * Maintenance / kill-switch fallback copy, as a dedicated next-intl catalog.
 *
 * The SSR kill-switch document (`components/MaintenanceNotice`) renders OUTSIDE
 * the normal `NextIntlClientProvider` tree — the layout returns it before the
 * provider is mounted (see the N8 defense-in-depth note in
 * `app/[locale]/layout.tsx`). Its copy therefore lives here as a small,
 * self-contained catalog (mirroring the edge kill-switch's inline table in
 * `lib/kill-switch.ts`) and is merged into the per-locale message tree under
 * `layout.maintenance` by `i18n/request.ts`, so `getTranslations('layout.maintenance')`
 * resolves it through the real next-intl layer — localized, with next-intl's
 * missing-key fallback behind it — instead of the previous hardcoded English.
 *
 * Keeping it out of the large `messages/*.json` catalogs keeps this
 * operational, rarely-touched copy next to the kill-switch logic it belongs to.
 */

import { type Locale, locales } from './config';

export type MaintenanceCopy = {
  /** Page title + heading shown on the maintenance document. */
  title: string;
  /** Body line. `{name}` is interpolated with the tenant/site name via ICU. */
  body: string;
};

export const maintenanceMessages: Record<Locale, MaintenanceCopy> = {
  en: {
    title: 'Service temporarily unavailable',
    body: '{name} is offline for maintenance. Please check back shortly.',
  },
  sk: {
    title: 'Služba je dočasne nedostupná',
    body: '{name} je nedostupný z dôvodu údržby. Skúste to prosím o chvíľu znovu.',
  },
  cs: {
    title: 'Služba je dočasně nedostupná',
    body: '{name} je nedostupný z důvodu údržby. Zkuste to prosím za chvíli znovu.',
  },
  de: {
    title: 'Dienst vorübergehend nicht verfügbar',
    body: '{name} ist wegen Wartungsarbeiten offline. Bitte versuchen Sie es bald erneut.',
  },
  pl: {
    title: 'Usługa tymczasowo niedostępna',
    body: '{name} jest niedostępny z powodu prac serwisowych. Spróbuj ponownie za chwilę.',
  },
  hu: {
    title: 'A szolgáltatás átmenetileg nem elérhető',
    body: 'A(z) {name} karbantartás miatt nem elérhető. Kérjük, próbálja újra rövidesen.',
  },
};

/**
 * Coerce an arbitrary locale string to one we have maintenance copy for.
 * Falls back to `en` for anything unrecognised (e.g. a malformed cookie or an
 * `Accept-Language` region we don't ship), so the document is never blank.
 */
export function resolveMaintenanceLocale(input: string | null | undefined): Locale {
  if (input && (locales as readonly string[]).includes(input)) {
    return input as Locale;
  }
  const head = (input ?? '').toLowerCase().split('-')[0];
  return (locales as readonly string[]).includes(head) ? (head as Locale) : 'en';
}

/**
 * Merge the maintenance catalog into a per-locale message tree under
 * `layout.maintenance`. Used by `i18n/request.ts` so both the server
 * (`getTranslations`) and client (`NextIntlClientProvider`) see the keys
 * without duplicating them into every `messages/*.json`.
 */
export function withMaintenanceMessages(
  messages: Record<string, unknown>,
  locale: string
): Record<string, unknown> {
  const resolved = resolveMaintenanceLocale(locale);
  const layout = (messages.layout ?? {}) as Record<string, unknown>;
  return {
    ...messages,
    layout: {
      ...layout,
      maintenance: maintenanceMessages[resolved],
    },
  };
}
