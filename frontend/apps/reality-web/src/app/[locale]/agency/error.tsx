'use client';

/**
 * Route-segment error boundary for the agency dashboards (Issue #2343,
 * finding 3). The parent `[locale]/error.tsx` already catches render-time
 * throws for the whole locale segment; this scoped boundary is a backstop
 * for the agency client components (dashboard, listings, realtors,
 * branding) so a render-time throw in any one of them degrades gracefully
 * to an in-place error/retry surface instead of white-screening, without
 * unmounting the surrounding locale chrome.
 *
 * Lives inside the locale layout, so NextIntlClientProvider is in scope and
 * useTranslations() resolves in the user's locale.
 */

import { useTranslations } from 'next-intl';
import { useEffect } from 'react';
import { StateView } from '@/components/states';

interface ErrorPageProps {
  error: Error & { digest?: string };
  reset: () => void;
}

export default function AgencyErrorBoundary({ error, reset }: ErrorPageProps) {
  const t = useTranslations('error');
  useEffect(() => {
    console.error('reality-web agency error boundary:', error);
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
