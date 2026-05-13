import type { MetadataRoute } from 'next';
import { locales } from '../i18n/config';

function siteUrl(): string {
  return process.env.NEXT_PUBLIC_SITE_URL || 'https://www.rlt.sk';
}

// Routes that must stay out of search-engine indexes regardless of which
// locale prefix the URL has. With `localePrefix: 'as-needed'`, the default
// locale (en) is served at the bare path while every other locale appears
// at `/sk/...`, `/de/...`, etc. — so each pattern needs to be enumerated
// across all six locales (the default + the five prefixed ones), otherwise
// e.g. `/sk/favorites` slips through.
const PRIVATE_PATHS = [
  '/api/',
  '/_next/',
  '/account/',
  '/favorites',
  '/inquiries',
  '/saved-searches',
  '/profile',
  '/compare',
  '/auth/',
] as const;

function expandLocalePaths(paths: ReadonlyArray<string>): string[] {
  const out = new Set<string>();
  for (const p of paths) {
    out.add(p);
    for (const lc of locales) {
      out.add(`/${lc}${p}`);
    }
  }
  return [...out];
}

export default function robots(): MetadataRoute.Robots {
  const base = siteUrl();
  return {
    rules: [
      {
        userAgent: '*',
        allow: '/',
        disallow: expandLocalePaths(PRIVATE_PATHS),
      },
    ],
    sitemap: `${base}/sitemap.xml`,
    host: base,
  };
}
