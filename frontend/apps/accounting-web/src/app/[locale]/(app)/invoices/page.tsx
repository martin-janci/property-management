import { setRequestLocale } from 'next-intl/server';
import { InvoiceList } from '@/components/invoices/InvoiceList';

type Props = { params: Promise<{ locale: string }> };

export default async function InvoicesPage({ params }: Props) {
  const { locale } = await params;
  setRequestLocale(locale);
  return <InvoiceList locale={locale} />;
}
