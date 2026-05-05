'use client';

import { Footer, Header } from '@/components/ui';

export default function AboutPage() {
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
          About Reality Portal
        </h1>
        <p style={{ color: 'var(--ppt-fg-secondary)', lineHeight: 1.7 }}>
          Reality Portal is the leading property listing platform across Slovakia, Czech Republic,
          and beyond.
        </p>
      </main>
      <Footer />
    </div>
  );
}
