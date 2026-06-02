/**
 * Shared constants + helpers for reality-web page objects.
 *
 * Keeps the app id and the next-intl locale-prefix rule in one place so no
 * page object re-derives them. reality-web uses `localePrefix: 'as-needed'`:
 * the default locale ('sk') is served unprefixed, every other locale is served
 * under `/<locale>`.
 */

import type { SitemapApp } from '@ppt/e2e';

/** This app's sitemap id. */
export const APP: SitemapApp = 'reality-web';

/** Locales reality-web ships (see src/i18n/config.ts). */
export type RealityLocale = 'en' | 'sk' | 'cs' | 'de' | 'pl' | 'hu';

/** The unprefixed default locale under `localePrefix: 'as-needed'`. */
export const DEFAULT_LOCALE: RealityLocale = 'sk';

/**
 * Apply a locale prefix to a sitemap-resolved path. The default locale stays
 * unprefixed; non-default locales gain a `/<locale>` segment.
 */
export function withLocale(path: string, locale?: RealityLocale): string {
  if (!locale || locale === DEFAULT_LOCALE) {
    return path;
  }
  const suffix = path === '/' ? '' : path;
  return `/${locale}${suffix}`;
}
