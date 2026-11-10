'use client';

/**
 * Realtor analytics page (UC-51.10).
 *
 * Displays headline performance metrics for the signed-in realtor.
 */

import { useTranslations } from 'next-intl';
import { useEffect, useState } from 'react';
import { ProtectedRoute } from '@/components/auth';
import { Footer, Header } from '@/components/ui';
import { getMyRealtorAnalytics, type RealtorAnalytics, RealtorApiError } from '@/lib/realtor-api';

const PERIODS: ReadonlyArray<{
  value: string;
  labelKey: 'period7d' | 'period30d' | 'period90d' | 'period1y';
}> = [
  { value: '7d', labelKey: 'period7d' },
  { value: '30d', labelKey: 'period30d' },
  { value: '90d', labelKey: 'period90d' },
  { value: '1y', labelKey: 'period1y' },
];

function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="stat">
      <span className="stat-label">{label}</span>
      <span className="stat-value">{value}</span>
      <style jsx>{`
        .stat {
          background: var(--ppt-bg-surface); padding: 20px; border-radius: 12px;
          box-shadow: 0 1px 3px rgba(0,0,0,.1); display: flex; flex-direction: column; gap: 8px;
        }
        .stat-label { font-size: 12px; text-transform: uppercase; letter-spacing: .5px; color: var(--ppt-fg-muted); font-weight: 600; }
        .stat-value { font-size: 1.875rem; font-weight: 700; color: var(--ppt-fg-primary); }
      `}</style>
    </div>
  );
}

function AnalyticsContent() {
  const t = useTranslations('pages.realtorAnalytics');
  const [period, setPeriod] = useState<string>('30d');
  const [analytics, setAnalytics] = useState<RealtorAnalytics | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string>();

  useEffect(() => {
    let cancelled = false;
    setIsLoading(true);
    setError(undefined);
    getMyRealtorAnalytics(period)
      .then((value) => {
        if (!cancelled) setAnalytics(value);
      })
      .catch((err) => {
        if (!cancelled) setError(err instanceof RealtorApiError ? err.message : t('error'));
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [period, t]);

  return (
    <div className="container">
      <header className="head">
        <div>
          <h1 className="title">{t('title')}</h1>
          <p className="subtitle">{t('subtitle')}</p>
        </div>
        <select value={period} onChange={(e) => setPeriod(e.target.value)} className="period">
          {PERIODS.map((option) => (
            <option key={option.value} value={option.value}>
              {t(option.labelKey)}
            </option>
          ))}
        </select>
      </header>

      {isLoading && <p className="state">{t('loading')}</p>}
      {error && <p className="state error">{error}</p>}

      {analytics && (
        <div className="grid">
          <StatCard label={t('totalListings')} value={analytics.totalListings.toLocaleString()} />
          <StatCard label={t('activeListings')} value={analytics.activeListings.toLocaleString()} />
          <StatCard label={t('totalViews')} value={analytics.totalViews.toLocaleString()} />
          <StatCard label={t('inquiries')} value={analytics.totalInquiries.toLocaleString()} />
          <StatCard
            label={t('conversionRate')}
            value={`${(analytics.conversionRate * 100).toFixed(1)}%`}
          />
        </div>
      )}

      <style jsx>{`
        .container { max-width: 960px; margin: 0 auto; padding: 32px 16px; }
        .head { display: flex; justify-content: space-between; align-items: flex-end; gap: 16px; margin-bottom: 24px; flex-wrap: wrap; }
        .title { font-size: 2rem; font-weight: 700; color: var(--ppt-fg-primary); margin: 0 0 4px; }
        .subtitle { color: var(--ppt-fg-muted); margin: 0; }
        .period {
          padding: 8px 12px; border: 1px solid var(--ppt-border-strong); border-radius: 8px; background: var(--ppt-bg-surface);
          font-size: 14px; color: var(--ppt-fg-primary);
        }
        .state { padding: 32px; text-align: center; color: var(--ppt-fg-muted); }
        .error { color: var(--ppt-color-danger-hover); }
        .grid {
          display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
          gap: 16px;
        }
      `}</style>
    </div>
  );
}

export default function RealtorAnalyticsPage() {
  return (
    <div className="page">
      <Header />
      <main>
        <ProtectedRoute>
          <AnalyticsContent />
        </ProtectedRoute>
      </main>
      <Footer />
      <style jsx>{`
        .page { min-height: 100vh; display: flex; flex-direction: column; background: var(--ppt-bg-app); }
        main { flex: 1; }
      `}</style>
    </div>
  );
}
