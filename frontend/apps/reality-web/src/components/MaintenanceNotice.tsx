/**
 * Maintenance (kill-switch) fallback document.
 *
 * Rendered by `src/app/[locale]/layout.tsx` when the per-tenant
 * `building_disabled` feature flag is on and the canonical edge kill-switch
 * (`src/middleware.ts`) did not fire — see the N8 note in the layout. This is
 * the SSR defense-in-depth branch: it replaces the whole app with a static
 * "temporarily unavailable" document.
 *
 * A complete `<html>` document is returned so the layout can short-circuit its
 * own tree with it. The copy is user-facing and is sourced from the next-intl
 * `layout.maintenance` catalog so it shows in the visitor's locale, with
 * next-intl's own missing-key fallback behind it — not hardcoded English.
 */

import { getTranslations } from 'next-intl/server';

export async function MaintenanceNotice({
  locale,
  tenantName,
}: {
  locale: string;
  tenantName: string;
}) {
  // Server-side translator for the request locale. next-intl applies its own
  // missing-key fallback (default locale) behind this, so an untranslated
  // locale degrades to real copy rather than a raw key.
  const t = await getTranslations('layout.maintenance');
  const title = t('title');
  const body = t('body', { name: tenantName });

  return (
    <html lang={locale}>
      <head>
        <title>{title}</title>
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
          <h1>{title}</h1>
          <p>{body}</p>
        </main>
      </body>
    </html>
  );
}
