/**
 * 404 page (Next.js convention) for the [locale] segment.
 *
 * Runs as a server component — no styled-jsx here (it's client-only).
 * The shared `StateView` is a client component via `'use client'`, so it
 * can be rendered from this server component without issue.
 */

import { StateView } from '@/components/states';
import Link from 'next/link';
import type { CSSProperties } from 'react';

const linkStyle: CSSProperties = {
  padding: '10px 20px',
  borderRadius: 8,
  background: 'var(--ppt-color-primary)',
  color: 'var(--ppt-fg-on-accent)',
  fontWeight: 500,
  textDecoration: 'none',
  display: 'inline-block',
  textAlign: 'center',
};

export default function NotFound() {
  return (
    <StateView
      icon="🧭"
      code="404"
      title="Page not found"
      description="The page you're looking for doesn't exist or has moved."
    >
      <Link href="/" style={linkStyle}>
        Back to home
      </Link>
    </StateView>
  );
}
