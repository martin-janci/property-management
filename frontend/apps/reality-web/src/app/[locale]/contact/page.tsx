'use client';

import { Footer, Header } from '@/components/ui';

export default function ContactPage() {
  return (
    <div style={{ minHeight: '100vh', display: 'flex', flexDirection: 'column' }}>
      <Header />
      <main style={{ flex: 1, maxWidth: 800, margin: '0 auto', padding: '48px 32px' }}>
        <h1 style={{ fontSize: '2rem', fontWeight: 800, color: 'var(--ppt-fg-primary)', marginBottom: 16 }}>
          Contact Us
        </h1>
        <p style={{ color: 'var(--ppt-fg-secondary)', lineHeight: 1.7 }}>
          For support or business inquiries, please reach out to us at{' '}
          <a href="mailto:info@rlt.sk" style={{ color: 'var(--ppt-fg-link)' }}>
            info@rlt.sk
          </a>
          .
        </p>
      </main>
      <Footer />
    </div>
  );
}
