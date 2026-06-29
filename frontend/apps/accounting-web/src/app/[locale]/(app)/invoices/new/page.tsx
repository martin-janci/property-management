import { setRequestLocale } from 'next-intl/server';
import { InvoiceForm } from '@/components/invoices/InvoiceForm';

type Props = { params: Promise<{ locale: string }> };

export default async function NewInvoicePage({ params }: Props) {
  const { locale } = await params;
  setRequestLocale(locale);
  return <InvoiceForm locale={locale} />;
}
