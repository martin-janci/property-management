'use client';

import { useTranslations } from 'next-intl';
import { Footer, Header } from '@/components/ui';

export default function ContactPage() {
  const t = useTranslations('pages.contact');

  return (
    <div style={{ minHeight: '100vh', display: 'flex', flexDirection: 'column' }}>
      <Header />
      <main style={{ flex: 1, maxWidth: 800, margin: '0 auto', padding: '48px 32px' }}>
        <h1
          style={{
            fontSize: '2rem',
            fontWeight: 800,
            color: 'var(--ppt-fg-primary)',
            marginBottom: 16,
          }}
        >
          {t('title')}
        </h1>
        <p style={{ color: 'var(--ppt-fg-secondary)', lineHeight: 1.7 }}>
          {t('description')}{' '}
          <a href={`mailto:${t('email')}`} style={{ color: 'var(--ppt-fg-link)' }}>
            {t('email')}
          </a>
          .
        </p>
      </main>
      <Footer />
    </div>
  );
}
