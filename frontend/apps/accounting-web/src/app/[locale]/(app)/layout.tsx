/**
 * Authenticated app-shell layout for the accounting-web (app) route group.
 *
 * This route group holds the authenticated screens (invoices, contacts —
 * owned by FE-Screens). Its only job here (FE-Scaffold) is the shell:
 *   - mount the client providers (QueryClient + SDK config) so the
 *     @ppt/accounting-api-client hooks work, and
 *   - enable static rendering for the locale.
 *
 * The parent [locale]/layout.tsx already provides <html>/<body> and the
 * NextIntlClientProvider, so this layout intentionally renders no document
 * chrome — only the authed providers + a content wrapper that FE-Screens fills
 * with per-screen navigation/pages.
 *
 * UC-ACC-17 (frontend foundation / resource-server auth wiring).
 */

import { setRequestLocale } from 'next-intl/server';
import type { ReactNode } from 'react';
import { AppProviders } from '@/providers/AppProviders';

type Props = {
  children: ReactNode;
  params: Promise<{ locale: string }>;
};

export default async function AppLayout({ children, params }: Props) {
  const { locale } = await params;
  // Keep the (app) group statically renderable per-locale, like the marketing
  // surface. The authed data itself is fetched client-side via the SDK hooks.
  setRequestLocale(locale);

  return (
    <AppProviders>
      <div data-acc-app-shell>{children}</div>
    </AppProviders>
  );
}
