import type { MetadataRoute } from 'next';

function siteUrl(): string {
  return process.env.NEXT_PUBLIC_SITE_URL || 'https://www.rlt.sk';
}

export default function robots(): MetadataRoute.Robots {
  const base = siteUrl();
  return {
    rules: [
      {
        userAgent: '*',
        allow: '/',
        disallow: ['/api/', '/_next/', '/account/', '/favorites', '/inquiries'],
      },
    ],
    sitemap: `${base}/sitemap.xml`,
    host: base,
  };
}
