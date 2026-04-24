'use client';

/**
 * Realtor analytics page (UC-51.10).
 *
 * Displays headline performance metrics for the signed-in realtor.
 */

import { ProtectedRoute } from '@/components/auth';
import { Footer, Header } from '@/components/ui';
import {
  RealtorApiError,
  type RealtorAnalytics,
  getMyRealtorAnalytics,
} from '@/lib/realtor-api';
import { useEffect, useState } from 'react';

const PERIODS: ReadonlyArray<{ value: string; label: string }> = [
  { value: '7d', label: 'Last 7 days' },
  { value: '30d', label: 'Last 30 days' },
  { value: '90d', label: 'Last 90 days' },
  { value: '1y', label: 'Last year' },
];

function StatCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="stat">
      <span className="stat-label">{label}</span>
      <span className="stat-value">{value}</span>
      <style jsx>{`
        .stat {
          background: #fff; padding: 20px; border-radius: 12px;
          box-shadow: 0 1px 3px rgba(0,0,0,.1); display: flex; flex-direction: column; gap: 8px;
        }
        .stat-label { font-size: 12px; text-transform: uppercase; letter-spacing: .5px; color: #6b7280; font-weight: 600; }
        .stat-value { font-size: 1.875rem; font-weight: 700; color: #111827; }
      `}</style>
    </div>
  );
}

function AnalyticsContent() {
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
        if (!cancelled)
          setError(err instanceof RealtorApiError ? err.message : 'Could not load analytics.');
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [period]);

  return (
    <div className="container">
      <header className="head">
        <div>
          <h1 className="title">Analytics</h1>
          <p className="subtitle">Performance of your listings over time.</p>
        </div>
        <select
          value={period}
          onChange={(e) => setPeriod(e.target.value)}
          className="period"
        >
          {PERIODS.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      </header>

      {isLoading && <p className="state">Loading metrics…</p>}
      {error && <p className="state error">{error}</p>}

      {analytics && (
        <div className="grid">
          <StatCard label="Total listings" value={analytics.totalListings.toLocaleString()} />
          <StatCard label="Active listings" value={analytics.activeListings.toLocaleString()} />
          <StatCard label="Total views" value={analytics.totalViews.toLocaleString()} />
          <StatCard label="Inquiries" value={analytics.totalInquiries.toLocaleString()} />
          <StatCard
            label="Conversion rate"
            value={`${(analytics.conversionRate * 100).toFixed(1)}%`}
          />
        </div>
      )}

      <style jsx>{`
        .container { max-width: 960px; margin: 0 auto; padding: 32px 16px; }
        .head { display: flex; justify-content: space-between; align-items: flex-end; gap: 16px; margin-bottom: 24px; flex-wrap: wrap; }
        .title { font-size: 2rem; font-weight: 700; color: #111827; margin: 0 0 4px; }
        .subtitle { color: #6b7280; margin: 0; }
        .period {
          padding: 8px 12px; border: 1px solid #d1d5db; border-radius: 8px; background: #fff;
          font-size: 14px; color: #111827;
        }
        .state { padding: 32px; text-align: center; color: #6b7280; }
        .error { color: #dc2626; }
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
        .page { min-height: 100vh; display: flex; flex-direction: column; background: #f9fafb; }
        main { flex: 1; }
      `}</style>
    </div>
  );
}
