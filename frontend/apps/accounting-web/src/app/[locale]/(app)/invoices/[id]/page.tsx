import { setRequestLocale } from 'next-intl/server';
import { InvoiceDetail } from '@/components/invoices/InvoiceDetail';

type Props = { params: Promise<{ locale: string; id: string }> };

export default async function InvoiceDetailPage({ params }: Props) {
  const { locale, id } = await params;
  setRequestLocale(locale);
  return <InvoiceDetail id={id} locale={locale} />;
}
