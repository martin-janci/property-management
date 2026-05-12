import type { MetadataRoute } from 'next';
import { locales } from '../i18n/config';

function siteUrl(): string {
  return process.env.NEXT_PUBLIC_SITE_URL || 'https://www.rlt.sk';
}

const PUBLIC_PATHS = [
  '',
  '/listings',
  '/listings?transactionType=sale',
  '/listings?transactionType=rent',
  '/sell',
  '/journal',
  '/help',
  '/contact',
  '/about',
  '/for-agents',
  '/careers',
  '/price-map',
  '/privacy',
  '/terms',
  '/cookies',
];

export default function sitemap(): MetadataRoute.Sitemap {
  const base = siteUrl();
  const now = new Date();

  return PUBLIC_PATHS.flatMap((path) =>
    locales.map((locale) => {
      const localePrefix = locale === 'en' ? '' : `/${locale}`;
      return {
        url: `${base}${localePrefix}${path}`,
        lastModified: now,
        changeFrequency: path === '' ? ('daily' as const) : ('weekly' as const),
        priority: path === '' ? 1 : 0.6,
        alternates: {
          languages: Object.fromEntries(
            locales.map((alt) => [alt, `${base}${alt === 'en' ? '' : `/${alt}`}${path}`]),
          ),
        },
      };
    }),
  );
}
