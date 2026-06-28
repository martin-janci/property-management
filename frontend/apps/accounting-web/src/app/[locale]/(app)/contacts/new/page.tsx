import { setRequestLocale } from 'next-intl/server';
import { ContactForm } from '@/components/contacts/ContactForm';

type Props = { params: Promise<{ locale: string }> };

export default async function NewContactPage({ params }: Props) {
  const { locale } = await params;
  setRequestLocale(locale);
  return <ContactForm />;
}
