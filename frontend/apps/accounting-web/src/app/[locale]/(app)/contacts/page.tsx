import { setRequestLocale } from 'next-intl/server';
import { ContactList } from '@/components/contacts/ContactList';

type Props = { params: Promise<{ locale: string }> };

export default async function ContactsPage({ params }: Props) {
  const { locale } = await params;
  setRequestLocale(locale);
  return <ContactList />;
}
