/**
 * sitemap() output contract — accounting-web (#1828 finding-3).
 *
 * The PR headlined SEO (sitemap/hreflang) but shipped no tests. This pins the
 * per-locale URL shape and the hreflang `alternates.languages` map that search
 * engines consume.
 */
import { describe, expect, it } from 'vitest';
import { locales } from '@/i18n/config';
import sitemap from './sitemap';

const DEFAULT_BASE = 'https://eko.dev.rlt.sk'; // NEXT_PUBLIC_SITE_URL fallback

describe('accounting-web sitemap', () => {
  const entries = sitemap();

  it('emits one entry per (public path × locale)', () => {
    // 2 public paths ('', '/signup') × 2 locales (cs, sk).
    expect(entries).toHaveLength(2 * locales.length);
  });

  it('serves cs at the bare path and sk under /sk (as-needed prefixing)', () => {
    const urls = entries.map((e) => e.url);
    expect(urls).toContain(`${DEFAULT_BASE}`); // cs home
    expect(urls).toContain(`${DEFAULT_BASE}/sk`); // sk home
    expect(urls).toContain(`${DEFAULT_BASE}/signup`); // cs signup
    expect(urls).toContain(`${DEFAULT_BASE}/sk/signup`); // sk signup
  });

  it('gives the home path the highest priority and a weekly change frequency', () => {
    const csHome = entries.find((e) => e.url === DEFAULT_BASE);
    expect(csHome?.priority).toBe(1);
    expect(csHome?.changeFrequency).toBe('weekly');
  });

  it('every entry carries an hreflang alternates map covering all locales', () => {
    for (const entry of entries) {
      const langs = entry.alternates?.languages ?? {};
      // Each entry advertises a URL for every supported locale.
      for (const locale of locales) {
        expect(langs).toHaveProperty(locale);
      }
      // The sk alternate of any entry is /sk-prefixed; the cs alternate is bare.
      expect(String(langs.sk)).toContain('/sk');
      expect(String(langs.cs)).not.toContain('/sk');
    }
  });
});
