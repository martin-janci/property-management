import type { MetadataRoute } from 'next';

function siteUrl(): string {
  return (process.env.NEXT_PUBLIC_SITE_URL || 'https://eko.dev.rlt.sk').replace(/\/+$/, '');
}

export default function robots(): MetadataRoute.Robots {
  const base = siteUrl();
  return {
    rules: [
      {
        userAgent: '*',
        allow: '/',
        // The signup form is public/indexable; the authenticated app shell
        // (added by PAP-303 #3) will live under /app and is disallowed here
        // pre-emptively so it never leaks into the index.
        disallow: ['/app/', '/sk/app/', '/api/', '/_next/'],
      },
    ],
    sitemap: `${base}/sitemap.xml`,
    host: base,
  };
}
