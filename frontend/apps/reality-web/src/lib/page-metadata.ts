import type { Metadata } from 'next';
import { getTranslations } from 'next-intl/server';
import { defaultLocale } from '../i18n/config';

const SITE_NAME = 'Reality Portal';

/**
 * Build the locale prefix used in canonical URLs.
 *
 * `i18n/routing.ts` uses `localePrefix: 'as-needed'`, which means the
 * default locale (English) is served at `/about` and *not* `/en/about`.
 * Canonical URLs must follow the same rule, otherwise English pages
 * would canonicalize to a URL that redirects (or to a duplicate that
 * disagrees with the sitemap).
 */
function localePrefix(locale: string): string {
  return locale === defaultLocale ? '' : `/${locale}`;
}

/**
 * Build a Next.js Metadata object for a page using the existing
 * `pages.<key>.{title,description}` translation entries.
 *
 * Each public page should `export const generateMetadata = pageMetadata('<key>')`.
 * The title is suffixed with ` — Reality Portal` so localized titles still
 * carry the brand in the SERP. If a key is missing in a locale, next-intl
 * falls back to English, which is much better than every page sharing the
 * root layout's catch-all title.
 */
export function pageMetadata(
  key: string,
  opts: { suffixBrand?: boolean } = {}
): (ctx: { params: Promise<{ locale: string }> }) => Promise<Metadata> {
  const { suffixBrand = true } = opts;
  return async ({ params }) => {
    const { locale } = await params;
    const t = await getTranslations({ locale, namespace: `pages.${key}` });
    const rawTitle = safeT(t, 'title');
    const description = safeT(t, 'description');
    if (!rawTitle) return {};
    const title =
      suffixBrand && !rawTitle.toLowerCase().includes('reality portal')
        ? `${rawTitle} — ${SITE_NAME}`
        : rawTitle;
    return {
      title,
      ...(description ? { description } : {}),
      openGraph: {
        title,
        ...(description ? { description } : {}),
        type: 'website',
        locale,
        siteName: SITE_NAME,
      },
      twitter: {
        card: 'summary_large_image',
        title,
        ...(description ? { description } : {}),
      },
      alternates: {
        canonical: `${localePrefix(locale)}${pagePathForKey(key)}`,
      },
      robots: {
        index: !PRIVATE_KEYS.has(key),
        follow: true,
      },
    };
  };
}

/**
 * Best-effort key → path mapping for the canonical URL.
 * Pages whose route name doesn't match the messages key get an override here.
 */
function pagePathForKey(key: string): string {
  const overrides: Record<string, string> = {
    forAgents: '/for-agents',
    priceMap: '/price-map',
    forgotPassword: '/auth/forgot-password',
    resetPassword: '/auth/reset-password',
    login: '/auth/login',
    register: '/auth/register',
    priceAlerts: '/favorites/alerts',
  };
  return overrides[key] ?? `/${key.replace(/[A-Z]/g, (c) => `-${c.toLowerCase()}`)}`;
}

// Pages we don't want crawled — user-data screens and auth flow internals.
const PRIVATE_KEYS = new Set([
  'login',
  'register',
  'forgotPassword',
  'resetPassword',
  'favorites',
  'priceAlerts',
  'inquiries',
  'profile',
]);

type TranslatorLike = {
  (key: string): string;
  has(key: string): boolean;
};

// Probe with `has` before calling `t(key)`. next-intl's onError logs
// MISSING_MESSAGE before throwing, so a try/catch can't suppress the log —
// `has` returns boolean without triggering it.
function safeT(t: TranslatorLike, key: string): string | undefined {
  if (!t.has(key)) return undefined;
  const v = t(key);
  return v && !v.startsWith('pages.') ? v : undefined;
}
