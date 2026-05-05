'use client';

import { Footer, Header } from '@/components/ui';

export default function TermsPage() {
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
          Terms of Service
        </h1>
        <p style={{ color: 'var(--ppt-fg-secondary)', lineHeight: 1.7 }}>
          By using Reality Portal, you agree to our terms and conditions. Full terms of service will
          be published here.
        </p>
      </main>
      <Footer />
    </div>
  );
}
