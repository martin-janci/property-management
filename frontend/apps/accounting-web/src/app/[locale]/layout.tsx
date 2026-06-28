import type { Metadata } from 'next';
import { Inter } from 'next/font/google';
import { notFound } from 'next/navigation';
import { NextIntlClientProvider } from 'next-intl';
import { getMessages, getTranslations, setRequestLocale } from 'next-intl/server';
import { BRAND_NAME, type Locale, locales } from '@/i18n/config';

const inter = Inter({
  subsets: ['latin', 'latin-ext'],
  display: 'swap',
  variable: '--ppt-font-inter',
  weight: ['400', '500', '600', '700', '800'],
});

// Placeholder public origin (PAP-303 open decision #1). Staging/worktree
// deploys override via NEXT_PUBLIC_SITE_URL.
const SITE_URL = (process.env.NEXT_PUBLIC_SITE_URL || 'https://eko.dev.rlt.sk').replace(/\/+$/, '');

export function generateStaticParams() {
  return locales.map((locale) => ({ locale }));
}

export async function generateMetadata({
  params,
}: {
  params: Promise<{ locale: string }>;
}): Promise<Metadata> {
  const { locale } = await params;
  const t = await getTranslations({ locale, namespace: 'metadata.home' });

  const title = `${BRAND_NAME} — ${t('title')}`;
  const description = t('description');

  return {
    metadataBase: new URL(SITE_URL),
    title: {
      default: title,
      template: `%s · ${BRAND_NAME}`,
    },
    description,
    applicationName: BRAND_NAME,
    alternates: {
      canonical: locale === 'cs' ? '/' : `/${locale}`,
      languages: Object.fromEntries(locales.map((lc) => [lc, lc === 'cs' ? '/' : `/${lc}`])),
    },
    openGraph: {
      type: 'website',
      siteName: BRAND_NAME,
      title,
      description,
      locale,
    },
    twitter: {
      card: 'summary_large_image',
      title,
      description,
    },
    robots: { index: true, follow: true },
  };
}

const SOFTWARE_LD = (siteUrl: string) =>
  JSON.stringify({
    '@context': 'https://schema.org',
    '@type': 'SoftwareApplication',
    name: BRAND_NAME,
    applicationCategory: 'BusinessApplication',
    operatingSystem: 'Web',
    url: siteUrl,
    offers: { '@type': 'Offer', price: '0', priceCurrency: 'EUR' },
    areaServed: ['CZ', 'SK'],
  });

type Props = {
  children: React.ReactNode;
  params: Promise<{ locale: string }>;
};

export default async function LocaleLayout({ children, params }: Props) {
  const { locale } = await params;

  if (!locales.includes(locale as Locale)) {
    notFound();
  }

  // Enable static rendering for this locale.
  setRequestLocale(locale);
  const messages = await getMessages();

  return (
    <html lang={locale} className={inter.variable}>
      <body>
        <script
          type="application/ld+json"
          // biome-ignore lint/security/noDangerouslySetInnerHtml: static JSON-LD, no user input
          dangerouslySetInnerHTML={{ __html: SOFTWARE_LD(SITE_URL) }}
        />
        <NextIntlClientProvider locale={locale} messages={messages}>
          {children}
        </NextIntlClientProvider>
      </body>
    </html>
  );
}
