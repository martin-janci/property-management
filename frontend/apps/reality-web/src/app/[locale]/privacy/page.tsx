'use client';

import { Footer, Header } from '@/components/ui';

export default function PrivacyPage() {
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
          Privacy Policy
        </h1>
        <p style={{ color: 'var(--ppt-fg-secondary)', lineHeight: 1.7 }}>
          We are committed to protecting your personal data in accordance with GDPR and applicable
          laws. Full privacy policy details will be published here.
        </p>
      </main>
      <Footer />
    </div>
  );
}
