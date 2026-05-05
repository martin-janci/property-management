'use client';

import { Footer, Header } from '@/components/ui';
import { useTranslations } from 'next-intl';

export default function PrivacyPage() {
  const t = useTranslations('pages.privacy');

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
        <p style={{ color: 'var(--ppt-fg-secondary)', lineHeight: 1.7 }}>{t('description')}</p>
      </main>
      <Footer />
    </div>
  );
}
