import type { MetadataRoute } from 'next';
import { locales } from '@/i18n/config';

function siteUrl(): string {
  return (process.env.NEXT_PUBLIC_SITE_URL || 'https://eko.dev.rlt.sk').replace(/\/+$/, '');
}

// Public, indexable paths. `as-needed` prefixing: cs (default) serves the
// bare path, sk serves `/sk/...`.
const PUBLIC_PATHS = ['', '/signup'] as const;

export default function sitemap(): MetadataRoute.Sitemap {
  const base = siteUrl();
  // Static build timestamp — Date() is unavailable in some build sandboxes and
  // a per-request value would defeat ISR caching anyway.
  const lastModified = '2026-06-23';

  return PUBLIC_PATHS.flatMap((path) =>
    locales.map((locale) => {
      const prefix = locale === 'cs' ? '' : `/${locale}`;
      return {
        url: `${base}${prefix}${path}`,
        lastModified,
        changeFrequency: path === '' ? ('weekly' as const) : ('monthly' as const),
        priority: path === '' ? 1 : 0.8,
        alternates: {
          languages: Object.fromEntries(
            locales.map((alt) => [alt, `${base}${alt === 'cs' ? '' : `/${alt}`}${path}`])
          ),
        },
      };
    })
  );
}
