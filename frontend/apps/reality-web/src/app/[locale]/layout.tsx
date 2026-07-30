import { Inter } from 'next/font/google';
import { notFound } from 'next/navigation';
import { NextIntlClientProvider } from 'next-intl';
import { getMessages, setRequestLocale } from 'next-intl/server';
import { AuthProvider } from '@/lib/auth-context';
import { ComparisonProvider } from '@/lib/comparison-context';
import { FeatureFlags } from '@/lib/feature-flags';
import { QueryProvider } from '@/lib/query-provider';
import { serializeForScript } from '@/lib/serialize-script';
import { brandingToStyleObject, getTenantConfig } from '@/lib/tenant-config';
import { TenantProvider } from '@/providers/TenantProvider';
import { CookieConsentBanner } from '../../components/CookieConsentBanner';
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

// Normalize the configured site URL so downstream concatenations don't
// produce `https://www.rlt.sk//icon.svg` when NEXT_PUBLIC_SITE_URL ships
// with a trailing slash (which happens whenever someone copies the URL
// from a browser address bar into env config). Computed once at module
// load — env vars are static for the lifetime of the process.
const SITE_URL = (process.env.NEXT_PUBLIC_SITE_URL || 'https://www.rlt.sk').replace(/\/+$/, '');

// Organization JSON-LD payload is the same for every page render — pull
// the JSON.stringify out of the render path so we don't re-serialize on
// every request. (Inline `<script>` injection means no React reconcile
// either way, but this keeps the layout render hot path clean.)
// serializeForScript (not bare JSON.stringify) so `<`/`>`/`&`/U+2028/U+2029
// are escaped before this lands inside <script>. Values here are static, but
// routing every inline-script payload through the same hardened primitive
// keeps the invariant local — no reviewer has to re-audit the data source.
const ORGANIZATION_LD = serializeForScript({
  '@context': 'https://schema.org',
  '@type': 'Organization',
  name: 'Reality Portal',
  url: SITE_URL,
  logo: `${SITE_URL}/icon.svg`,
  email: 'info@rlt.sk',
  areaServed: ['SK', 'CZ', 'DE', 'PL', 'HU'],
});

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
    // metadataBase lets per-route generateMetadata emit relative canonicals
    // (e.g. '/about') and Next resolves them to absolute URLs in <head>.
    // Without this, OG/canonical URLs in headers are emitted as paths and
    // get flagged by Twitter / FB share validators.
    metadataBase: new URL(SITE_URL),
    title: titles[locale as Locale] || titles.en,
    description: descriptions[locale as Locale] || descriptions.en,
    openGraph: {
      title: titles[locale as Locale] || titles.en,
      description: descriptions[locale as Locale] || descriptions.en,
      type: 'website',
      locale: locale,
      images: ['/opengraph-image'],
    },
    twitter: {
      card: 'summary_large_image',
      title: titles[locale as Locale] || titles.en,
      description: descriptions[locale as Locale] || descriptions.en,
      images: ['/opengraph-image'],
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

  // Phase 3: per-host tenant config drives branding + kill switches.
  // `getTenantConfig` is React-cached so the layout, page, and any other
  // server component on this request all share one fetch.
  const tenantConfig = await getTenantConfig();

  // Per-tenant kill switch: when `building_disabled` is enabled the entire
  // app is replaced with a 503 page (defends operational leak #22).
  //
  // N8: the canonical kill-switch path is `src/middleware.ts`, which
  // returns a real `503 Service Unavailable` BEFORE any rendering. This
  // layout-level branch stays as defense in depth: if the edge fetch
  // failed but the SSR `getTenantConfig` succeeded, we still want to hide
  // the app. The HTTP status will be 200 in that fallback, but the
  // markup-level guard is preserved so users never see the real app
  // during maintenance.
  const killSwitch = tenantConfig.feature_flags[FeatureFlags.BUILDING_DISABLED];
  if (killSwitch?.enabled) {
    return (
      <html lang={locale}>
        <head>
          <title>Service temporarily unavailable</title>
          <meta name="robots" content="noindex" />
        </head>
        <body>
          <main
            style={{
              fontFamily: 'system-ui, sans-serif',
              padding: '2rem',
              maxWidth: 600,
              margin: '4rem auto',
              textAlign: 'center',
            }}
          >
            <h1>Service temporarily unavailable</h1>
            <p>{tenantConfig.name} is offline for maintenance. Please check back shortly.</p>
          </main>
        </body>
      </html>
    );
  }

  // React style object carrying `--ppt-*` CSS custom properties from the
  // tenant config. Sanitized inside the helper — keys + values are
  // allowlist-validated so no tenant-controlled blob can break out of the
  // style attribute.
  const brandingStyle = brandingToStyleObject(tenantConfig.branding);

  // Embed a minimal projection of the tenant config so client-side
  // `useFeatureFlag` and `useTenantContext` work without a fetch.
  const tenantBootstrapObj = {
    tenant_id: tenantConfig.tenant_id,
    feature_flags: tenantConfig.feature_flags,
  };
  // MUST be serializeForScript, not JSON.stringify: `tenant_id` and the
  // feature-flag keys are per-request, host-derived values. A bare
  // JSON.stringify would let a `</script>` (or U+2028/U+2029) inside any of
  // them break out of the inline bootstrap <script> below — an HTML-injection
  // window on every request. serializeForScript escapes those to `\uXXXX`.
  const tenantBootstrap = serializeForScript(tenantBootstrapObj);

  return (
    <html
      lang={locale}
      className={inter.variable}
      style={brandingStyle}
      // Apply system color scheme by default; JS in ColorSchemeScript can
      // override with a stored user preference at runtime.
      suppressHydrationWarning
    >
      <head>
        {tenantConfig.branding.logo_url ? (
          <link rel="icon" href={tenantConfig.branding.logo_url} />
        ) : null}
        <script
          // biome-ignore lint/security/noDangerouslySetInnerHtml: bootstrap must inline
          dangerouslySetInnerHTML={{
            __html: `window.__TENANT_CONFIG__=${tenantBootstrap};`,
          }}
        />
        {/*
         * Color-scheme bootstrap MUST be the first thing in <head> so it runs
         * before any layout paint. A previous version put this in <body> with
         * Next.js's <Script strategy="beforeInteractive"> which sounds right
         * but in the App Router that strategy still queues the script after
         * the framework's hydration boundary — first paint had no theme attr
         * set and dark-mode users saw a white flash on every navigation.
         * Plain inline <script> with `dangerouslySetInnerHTML` runs eagerly
         * during HTML parse, which is what we actually want here.
         */}
        <script
          // biome-ignore lint/security/noDangerouslySetInnerHtml: bootstrap must inline
          dangerouslySetInnerHTML={{
            // Validate the stored value before applying it — a corrupted
            // or legacy localStorage entry (e.g. 'system', 'auto', or any
            // typo from a manual edit) would otherwise become an invalid
            // data-color-scheme attribute and miss every [data-color-scheme=dark]
            // CSS selector, leaving the page in a broken half-light state.
            __html:
              "(function(){try{var s=localStorage.getItem('ppt-color-scheme');" +
              "if(s!=='light'&&s!=='dark')s=null;" +
              "var m=s||(window.matchMedia('(prefers-color-scheme: dark)').matches?'dark':'light');" +
              "document.documentElement.setAttribute('data-color-scheme',m);}catch(e){}})();",
          }}
        />
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
        {/* Organization JSON-LD: surfaces the brand panel on Google search
            (sitelinks + logo + areaServed). Emitted at the locale layout
            so every page carries it; per-page schema (Listing, Article,
            Breadcrumb) is added by individual routes on top.

            Payload is precomputed at module load (see ORGANIZATION_LD)
            rather than rebuilt on every render. */}
        <script
          type="application/ld+json"
          // biome-ignore lint/security/noDangerouslySetInnerHtml: JSON-LD must inline
          dangerouslySetInnerHTML={{ __html: ORGANIZATION_LD }}
        />
      </head>
      <body>
        {/* StyledJsxRegistry must wrap any subtree that uses `<style jsx>`
            so the styles get streamed into the SSR HTML rather than
            injected post-hydration (which caused a ~0.92 CLS shift). */}
        <StyledJsxRegistry>
          <TenantProvider initial={tenantBootstrapObj}>
            <NextIntlClientProvider messages={messages}>
              <QueryProvider>
                <AuthProvider>
                  <ComparisonProvider>
                    {children}
                    <ComparisonTray />
                    <CookieConsentBanner />
                    <DevPanelMount />
                  </ComparisonProvider>
                </AuthProvider>
              </QueryProvider>
            </NextIntlClientProvider>
          </TenantProvider>
        </StyledJsxRegistry>
      </body>
    </html>
  );
}
