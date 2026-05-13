/**
 * Root-level 404 page (Next.js convention).
 *
 * Catches requests that fall outside the `[locale]` segment — e.g.
 * `/nonexistent-page` (no locale prefix) — that would otherwise show
 * Next's generic "404: This page could not be found." title and body.
 * The locale-aware variant lives at `[locale]/not-found.tsx`.
 *
 * Defaults to English copy; the brand `<title>` here matters mainly so
 * that the SERP and tab title don't expose Next's unbranded default.
 */

import type { Metadata } from 'next';
import Link from 'next/link';

export const metadata: Metadata = {
  title: 'Page not found — Reality Portal',
  description: 'The page you are looking for could not be found.',
  robots: { index: false, follow: false },
};

export default function NotFound() {
  return (
    <html lang="en">
      <body
        style={{
          margin: 0,
          fontFamily: 'system-ui, -apple-system, sans-serif',
          minHeight: '100vh',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          background: '#f8fafc',
          color: '#111827',
        }}
      >
        <main
          style={{
            textAlign: 'center',
            padding: '48px 24px',
            maxWidth: 480,
          }}
        >
          <div style={{ fontSize: 64, marginBottom: 16 }}>🧭</div>
          <div
            style={{
              fontSize: 14,
              fontWeight: 600,
              color: '#6b7280',
              letterSpacing: '.1em',
              textTransform: 'uppercase',
              marginBottom: 8,
            }}
          >
            404
          </div>
          <h1 style={{ fontSize: '1.75rem', fontWeight: 800, margin: '0 0 12px' }}>
            Page not found
          </h1>
          <p style={{ color: '#4b5563', marginBottom: 28 }}>
            The page you are looking for doesn&rsquo;t exist or has been moved.
          </p>
          <Link
            href="/"
            style={{
              display: 'inline-block',
              padding: '10px 22px',
              background: '#2563eb',
              color: '#fff',
              borderRadius: 8,
              fontWeight: 600,
              textDecoration: 'none',
            }}
          >
            Back to home
          </Link>
        </main>
      </body>
    </html>
  );
}
