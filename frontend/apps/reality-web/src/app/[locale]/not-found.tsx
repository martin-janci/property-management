/**
 * 404 page (Next.js convention) for the [locale] segment.
 *
 * Runs as a server component — no styled-jsx here (it's client-only).
 * The shared `StateView` is a client component via `'use client'`, so it
 * can be rendered from this server component without issue.
 *
 * Uses `getTranslations` (server variant) — we're inside the locale
 * segment so the request's locale is already in scope.
 */

import Link from 'next/link';
import { getTranslations } from 'next-intl/server';
import type { CSSProperties } from 'react';
import { StateView } from '@/components/states';

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

export default async function NotFound() {
  const t = await getTranslations('notFound');
  return (
    <StateView icon="🧭" code="404" title={t('title')} description={t('description')}>
      <Link href="/" style={linkStyle}>
        {t('home')}
      </Link>
    </StateView>
  );
}
