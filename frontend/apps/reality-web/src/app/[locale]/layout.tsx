import { Inter } from 'next/font/google';
import { notFound } from 'next/navigation';
import Script from 'next/script';
import { NextIntlClientProvider } from 'next-intl';
import { getMessages, setRequestLocale } from 'next-intl/server';
import { AuthProvider } from '@/lib/auth-context';
import { ComparisonProvider } from '@/lib/comparison-context';
import { QueryProvider } from '@/lib/query-provider';
import { ComparisonTray } from '../../components/comparison';
import { DevPanelMount } from '../../components/DevPanelMount';
import { StyledJsxRegistry } from '../../components/StyledJsxRegistry';
import { type Locale, locales } from '../../i18n/config';

// Self-host Inter via next/font so the browser doesn't reflow once the
// webfont arrives (FOUT). `display: 'swap'` keeps text visible while the
// font loads; the CSS `size-adjust` Next emits keeps fallback metrics
// close enough that any swap is imperceptible. Exposed as a CSS variable
// so the existing `--ppt-font-family` token can pick it up.
const inter = Inter({
  subsets: ['latin', 'latin-ext'],
  display: 'swap',
  variable: '--ppt-font-inter',
  weight: ['400', '500', '600', '700', '800'],
});

type Props = {
  children: React.ReactNode;
  params: Promise<{ locale: string }>;
};

export function generateStaticParams() {
  return locales.map((locale) => ({ locale }));
}

export async function generateMetadata({ params }: { params: Promise<{ locale: string }> }) {
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

  return (
    <html
      lang={locale}
      className={inter.variable}
      // Apply system color scheme by default; JS in ColorSchemeScript can
      // override with a stored user preference at runtime.
      suppressHydrationWarning
    >
      <head>
        {/*
         * Runtime env config: the /env.js route handler (src/app/env.js/route.ts)
         * serves window.__ENV__ dynamically at request time, so the same built
         * image can be deployed to different environments without a rebuild.
         * Loaded without async/defer so it executes synchronously before the
         * React bundle, making window.__ENV__ available to all client modules.
         * The locale layout itself remains statically renderable.
         */}
        {/* eslint-disable-next-line @next/next/no-sync-scripts */}
        <script src="/env.js" />
      </head>
      <body>
        {/* Sets data-color-scheme before first paint to avoid flash of wrong theme */}
        <Script id="color-scheme-init" strategy="beforeInteractive">{`
(function(){try{var s=localStorage.getItem('ppt-color-scheme');var m=s||(window.matchMedia('(prefers-color-scheme: dark)').matches?'dark':'light');document.documentElement.setAttribute('data-color-scheme',m);}catch(e){}})();
        `}</Script>
        {/* StyledJsxRegistry must wrap any subtree that uses `<style jsx>`
            so the styles get streamed into the SSR HTML rather than
            injected post-hydration (which caused a ~0.92 CLS shift). */}
        <StyledJsxRegistry>
          <NextIntlClientProvider messages={messages}>
            <QueryProvider>
              <AuthProvider>
                <ComparisonProvider>
                  {children}
                  <ComparisonTray />
                  <DevPanelMount />
                </ComparisonProvider>
              </AuthProvider>
            </QueryProvider>
          </NextIntlClientProvider>
        </StyledJsxRegistry>
      </body>
    </html>
  );
}
