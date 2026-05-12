import type { Metadata } from 'next';
import { getTranslations } from 'next-intl/server';

const SITE_NAME = 'Reality Portal';

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
  opts: { suffixBrand?: boolean } = {},
): (ctx: { params: Promise<{ locale: string }> }) => Promise<Metadata> {
  const { suffixBrand = true } = opts;
  return async ({ params }) => {
    const { locale } = await params;
    const t = await getTranslations({ locale, namespace: `pages.${key}` });
    const rawTitle = safeT(t, 'title');
    const description = safeT(t, 'description');
    if (!rawTitle) return {};
    const title = suffixBrand && !rawTitle.toLowerCase().includes('reality portal')
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
      },
    };
  };
}

function safeT(t: (key: string) => string, key: string): string | undefined {
  try {
    const v = t(key);
    return v && !v.startsWith('pages.') ? v : undefined;
  } catch {
    return undefined;
  }
}
