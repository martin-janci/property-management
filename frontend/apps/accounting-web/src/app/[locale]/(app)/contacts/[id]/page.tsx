import { setRequestLocale } from 'next-intl/server';
import { ContactForm } from '@/components/contacts/ContactForm';

type Props = { params: Promise<{ locale: string; id: string }> };

// Edit a contact (UC-ACC-03.1/.9). The ContactForm hydrates from the loaded
// contact and persists via PATCH /contacts/{id}.
export default async function ContactDetailPage({ params }: Props) {
  const { locale, id } = await params;
  setRequestLocale(locale);
  return <ContactForm contactId={id} />;
}
