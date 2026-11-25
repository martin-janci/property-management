import { getRequestConfig } from 'next-intl/server';
import { type Locale, locales } from './config';
import { withMaintenanceMessages } from './maintenance-messages';

export default getRequestConfig(async ({ requestLocale }) => {
  // Validate that the incoming locale is valid
  let locale = await requestLocale;

  if (!locale || !locales.includes(locale as Locale)) {
    locale = 'en';
  }

  const messages = (await import(`../../messages/${locale}.json`)).default as Record<
    string,
    unknown
  >;

  return {
    locale,
    // Merge the dedicated maintenance catalog under `layout.maintenance` so the
    // SSR kill-switch document can resolve it through next-intl without the
    // keys being duplicated into every `messages/*.json`. See
    // `i18n/maintenance-messages.ts`.
    messages: withMaintenanceMessages(messages, locale),
  };
});
