import { NextIntlClientProvider } from 'next-intl';
import { getMessages, setRequestLocale } from 'next-intl/server';
import { AuthProvider } from '@/lib/auth-context';
import { QueryProvider } from '@/lib/query-provider';
import { defaultLocale } from '@/i18n/config';

// The /accounting payment-matching review is auth-gated and entirely
// data-driven (TanStack Query against the live API), so it must never be
// statically prerendered — during SSG there is no QueryClient in scope.
// Setting `force-dynamic` in this server layout (route-segment config is
// ignored when exported from a `'use client'` page) opts the segment out of
// static generation. This route lives outside the `[locale]` tree, so it does
// not inherit that layout's providers; we supply the same QueryProvider,
// NextIntl, and Auth providers here using the default locale's messages.
export const dynamic = 'force-dynamic';

export default async function AccountingLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  setRequestLocale(defaultLocale);
  const messages = await getMessages({ locale: defaultLocale });

  return (
    <NextIntlClientProvider locale={defaultLocale} messages={messages}>
      <QueryProvider>
        <AuthProvider>{children}</AuthProvider>
      </QueryProvider>
    </NextIntlClientProvider>
  );
}
