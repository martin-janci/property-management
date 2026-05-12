'use client';

/**
 * Runtime error boundary (Next.js convention) for the [locale] segment.
 *
 * Receives the thrown error and a reset function from Next.js. We log the
 * error to the console for diagnostics in development; production
 * deployments should wire it to their telemetry pipeline.
 *
 * Lives inside the locale layout, so NextIntlClientProvider is in scope
 * and useTranslations() works — meaning the message and CTA show in the
 * user's locale, not English by default.
 */

import { useTranslations } from 'next-intl';
import { useEffect } from 'react';
import { StateView } from '@/components/states';

interface ErrorPageProps {
  error: Error & { digest?: string };
  reset: () => void;
}

export default function ErrorPage({ error, reset }: ErrorPageProps) {
  const t = useTranslations('error');
  useEffect(() => {
    console.error('reality-web error boundary:', error);
  }, [error]);

  return (
    <StateView icon="🛠️" code="500" title={t('title')} description={t('description')}>
      <button type="button" className="state-action" onClick={reset}>
        {t('retry')}
      </button>
      <style jsx global>{`
        .state-action {
          padding: 10px 20px; border-radius: 8px; border: none; cursor: pointer;
          background: var(--ppt-color-primary); color: var(--ppt-fg-on-accent); font-weight: 500;
          text-align: center; font-size: 15px;
        }
        .state-action:hover { background: var(--ppt-color-primary-hover); }
      `}</style>
    </StateView>
  );
}
