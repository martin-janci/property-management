import { AuthProvider } from '@/lib/auth-context';
import { ComparisonProvider } from '@/lib/comparison-context';
import { QueryProvider } from '@/lib/query-provider';
import { NextIntlClientProvider } from 'next-intl';
import { getMessages, setRequestLocale } from 'next-intl/server';
import { notFound } from 'next/navigation';
import Script from 'next/script';
import { ComparisonTray } from '../../components/comparison';
import { type Locale, locales } from '../../i18n/config';

type Props = {
  children: React.ReactNode;
  params: Promise<{ locale: string }>;
};

export function generateStaticParams() {
  return locales.map((locale) => ({ locale }));
}

export async function generateMetadata({
  params,
}: {
  params: Promise<{ locale: string }>;
}) {
  const { locale } = await params;

  const titles: Record<Locale, string> = {
    en: 'Reality Portal - Find Your Perfect Property',
    sk: 'Reality Portál - Nájdite svoju ideálnu nehnuteľnosť',
    cs: 'Reality Portál - Najděte svou ideální nemovitost',
    de: 'Reality Portal - Finden Sie Ihre perfekte Immobilie',
    pl: 'Reality Portal - Znajdź swoją idealną nieruchomość',
    hu: 'Reality Portal - Találja meg tökéletes ingatlanát',
  };

  const descriptions: Record<Locale, string> = {
    en: 'Search thousands of property listings across Slovakia, Czech Republic, and beyond.',
    sk: 'Prehľadajte tisíce ponúk nehnuteľností na Slovensku, v Českej republike a ďalej.',
    cs: 'Prohledejte tisíce nabídek nemovitostí v České republice, na Slovensku a dále.',
    de: 'Durchsuchen Sie Tausende von Immobilienangeboten in der Slowakei, der Tschechischen Republik und darüber hinaus.',
    pl: 'Przeszukaj tysiące ofert nieruchomości na Słowacji, w Czechach i nie tylko.',
    hu: 'Keressen több ezer ingatlanhirdetés között Szlovákiában, Csehországban és azon túl.',
  };

  return {
    title: titles[locale as Locale] || titles.en,
    description: descriptions[locale as Locale] || descriptions.en,
    openGraph: {
      title: titles[locale as Locale] || titles.en,
      description: descriptions[locale as Locale] || descriptions.en,
      type: 'website',
      locale: locale,
    },
  };
}

export default async function LocaleLayout({ children, params }: Props) {
  const { locale } = await params;

  // Validate the locale
  if (!locales.includes(locale as Locale)) {
    notFound();
  }

  // Enable static rendering
  setRequestLocale(locale);

  // Get messages for the current locale
  const messages = await getMessages();

  // Read env vars server-side (at request time) and inject into the page so
  // client components can read them via window.__ENV__ without a rebuild.
  const runtimeEnv = {
    NEXT_PUBLIC_API_URL: process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8081',
    NEXT_PUBLIC_SITE_URL: process.env.NEXT_PUBLIC_SITE_URL || 'http://localhost:3001',
  };

  return (
    <html
      lang={locale}
      // Apply system color scheme by default; JS in ColorSchemeScript can
      // override with a stored user preference at runtime.
      suppressHydrationWarning
    >
      <head>
        {/* Runtime config: client components read window.__ENV__ via src/lib/env.ts */}
        <script
          dangerouslySetInnerHTML={{
            __html: `window.__ENV__=${JSON.stringify(runtimeEnv)};`,
          }}
        />
      </head>
      <body>
        {/* Sets data-color-scheme before first paint to avoid flash of wrong theme */}
        <Script id="color-scheme-init" strategy="beforeInteractive">{`
(function(){try{var s=localStorage.getItem('ppt-color-scheme');var m=s||(window.matchMedia('(prefers-color-scheme: dark)').matches?'dark':'light');document.documentElement.setAttribute('data-color-scheme',m);}catch(e){}})();
        `}</Script>
        <NextIntlClientProvider messages={messages}>
          <QueryProvider>
            <AuthProvider>
              <ComparisonProvider>
                {children}
                <ComparisonTray />
              </ComparisonProvider>
            </AuthProvider>
          </QueryProvider>
        </NextIntlClientProvider>
      </body>
    </html>
  );
}
