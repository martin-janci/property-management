/**
 * AgencyDashboard Component
 *
 * Dashboard for agency owners to view performance metrics (Epic 45, Story 45.1).
 */

'use client';

import type { AgencyPerformance, AgencyStats } from '@ppt/reality-api-client';
import {
  useAgencyPerformance,
  useAgencyStats,
  useMyAgency,
  useRealtors,
} from '@ppt/reality-api-client';
import Link from 'next/link';
import { useTranslations } from 'next-intl';
import { useState } from 'react';
import {
  AgencyLoadError,
  isNotFoundError,
  NoAgencyMessage,
  SectionError,
} from './AgencyErrorStates';

type PeriodType = '7d' | '30d' | '90d' | '12m';

export function AgencyDashboard() {
  const t = useTranslations('agency');
  const [period, setPeriod] = useState<PeriodType>('30d');
  const {
    data: agency,
    isLoading: agencyLoading,
    isError: agencyError,
    error: agencyErrorObj,
    refetch: refetchAgency,
  } = useMyAgency();
  const {
    data: stats,
    isLoading: statsLoading,
    isError: statsError,
  } = useAgencyStats(agency?.id || '', period);
  const { data: performance, isError: performanceError } = useAgencyPerformance(
    agency?.id || '',
    undefined,
    undefined,
    'week'
  );
  const { data: realtors, isError: realtorsError } = useRealtors(agency?.id || '');

  if (agencyLoading) {
    return <DashboardSkeleton />;
  }

  // Distinguish a failed fetch from a genuine "no agency" state. A 500/network
  // error settles as `agency === undefined` and would fall through to
  // <NoAgencyMessage />, showing an actual agency owner the misleading
  // "No Agency Found / Create Agency" screen (Issue #2277). Conversely a 404
  // *is* the genuine "no agency" case (Issue #2343) — GET /agencies/me returns
  // 404 when the caller has no agency — so it must reach <NoAgencyMessage />,
  // not the error/retry screen.
  if (agencyError && !isNotFoundError(agencyErrorObj)) {
    return <AgencyLoadError onRetry={() => refetchAgency()} />;
  }

  if (!agency) {
    return <NoAgencyMessage />;
  }

  return (
    <div className="dashboard">
      {/* Header */}
      <div className="header">
        <div className="header-content">
          <h1 className="title">{agency.name}</h1>
          <p className="subtitle">{t('dashboard')}</p>
        </div>
        <div className="period-selector">
          {(['7d', '30d', '90d', '12m'] as PeriodType[]).map((p) => (
            <button
              key={p}
              type="button"
              className={`period-button ${period === p ? 'active' : ''}`}
              onClick={() => setPeriod(p)}
            >
              {p === '7d'
                ? t('period7d')
                : p === '30d'
                  ? t('period30d')
                  : p === '90d'
                    ? t('period90d')
                    : t('period1y')}
            </button>
          ))}
        </div>
      </div>

      {/* Stats Cards */}
      {statsError ? (
        <SectionError message={t('sectionLoadError')} />
      ) : statsLoading ? (
        <StatsCardsSkeleton />
      ) : stats ? (
        <StatsCards stats={stats} />
      ) : null}

      {/* Main Content Grid */}
      <div className="content-grid">
        {/* Performance Chart */}
        <div className="section chart-section">
          <h2 className="section-title">{t('performanceOverview')}</h2>
          {performanceError ? (
            <SectionError message={t('sectionLoadError')} />
          ) : (
            performance && <PerformanceChart data={performance} />
          )}
        </div>

        {/* Realtor Leaderboard */}
        <div className="section leaderboard-section">
          <div className="section-header">
            <h2 className="section-title">{t('topRealtors')}</h2>
            <Link href="/agency/realtors" className="view-all">
              {t('viewAll')}
            </Link>
          </div>
          {realtorsError ? (
            <SectionError message={t('sectionLoadError')} />
          ) : (
            <RealtorLeaderboard
              realtors={
                realtors
                  ?.filter((r) => r.status === 'active')
                  .sort((a, b) => b.totalSales - a.totalSales)
                  .slice(0, 5) || []
              }
            />
          )}
        </div>
      </div>

      {/* Quick Actions */}
      <div className="quick-actions">
        <h2 className="section-title">{t('quickActions')}</h2>
        <div className="actions-grid">
          <Link href="/agency/realtors" className="action-card">
            <svg
              width="24"
              height="24"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
              <circle cx="9" cy="7" r="4" />
              <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
              <path d="M16 3.13a4 4 0 0 1 0 7.75" />
            </svg>
            <span>{t('manageRealtors')}</span>
          </Link>
          <Link href="/agency/listings" className="action-card">
            <svg
              width="24"
              height="24"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
              <polyline points="9 22 9 12 15 12 15 22" />
            </svg>
            <span>{t('allListings')}</span>
          </Link>
          <Link href="/agency/inquiries" className="action-card">
            <svg
              width="24"
              height="24"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
            </svg>
            <span>{t('inquiries')}</span>
          </Link>
          <Link href="/agency/branding" className="action-card">
            <svg
              width="24"
              height="24"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <circle cx="13.5" cy="6.5" r="0.5" />
              <circle cx="17.5" cy="10.5" r="0.5" />
              <circle cx="8.5" cy="7.5" r="0.5" />
              <circle cx="6.5" cy="12.5" r="0.5" />
              <path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.555-2.503 5.555-5.555C21.965 6.012 17.461 2 12 2z" />
            </svg>
            <span>{t('branding')}</span>
          </Link>
          <Link href="/agency/realtors?action=invite" className="action-card primary">
            <svg
              width="24"
              height="24"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <path d="M16 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
              <circle cx="8.5" cy="7" r="4" />
              <line x1="20" y1="8" x2="20" y2="14" />
              <line x1="23" y1="11" x2="17" y2="11" />
            </svg>
            <span>{t('inviteRealtor')}</span>
          </Link>
        </div>
      </div>

      <style jsx>{`
        .dashboard {
          padding: 24px;
          max-width: 1400px;
          margin: 0 auto;
        }

        .header {
          display: flex;
          justify-content: space-between;
          align-items: flex-start;
          margin-bottom: 32px;
          flex-wrap: wrap;
          gap: 16px;
        }

        .title {
          font-size: 2rem;
          font-weight: bold;
          color: var(--ppt-fg-primary);
          margin: 0;
        }

        .subtitle {
          font-size: 1rem;
          color: var(--ppt-fg-muted);
          margin: 4px 0 0;
        }

        .period-selector {
          display: flex;
          gap: 8px;
          background: var(--ppt-bg-subtle);
          padding: 4px;
          border-radius: 8px;
        }

        .period-button {
          padding: 8px 16px;
          border: none;
          background: transparent;
          border-radius: 6px;
          font-size: 14px;
          font-weight: 500;
          color: var(--ppt-fg-muted);
          cursor: pointer;
          transition: all 0.2s;
        }

        .period-button.active {
          background: var(--ppt-bg-surface);
          color: var(--ppt-color-primary);
          box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
        }

        .content-grid {
          display: grid;
          gap: 24px;
          margin-bottom: 32px;
        }

        @media (min-width: 1024px) {
          .content-grid {
            grid-template-columns: 2fr 1fr;
          }
        }

        .section {
          background: var(--ppt-bg-surface);
          border-radius: 12px;
          padding: 24px;
          box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
        }

        .section-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 16px;
        }

        .section-title {
          font-size: 1.125rem;
          font-weight: 600;
          color: var(--ppt-fg-primary);
          margin: 0 0 16px;
        }

        .section-header .section-title {
          margin: 0;
        }

        .view-all {
          font-size: 14px;
          color: var(--ppt-color-primary);
          text-decoration: none;
        }

        .view-all:hover {
          text-decoration: underline;
        }

        .quick-actions {
          background: var(--ppt-bg-surface);
          border-radius: 12px;
          padding: 24px;
          box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
        }

        .actions-grid {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
          gap: 16px;
        }

        .action-card {
          display: flex;
          align-items: center;
          gap: 12px;
          padding: 16px 20px;
          background: var(--ppt-bg-app);
          border: 1px solid var(--ppt-border-default);
          border-radius: 8px;
          color: var(--ppt-fg-secondary);
          text-decoration: none;
          font-weight: 500;
          transition: all 0.2s;
        }

        .action-card:hover {
          background: var(--ppt-bg-subtle);
          border-color: var(--ppt-border-strong);
        }

        .action-card.primary {
          background: var(--ppt-color-primary);
          border-color: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent);
        }

        .action-card.primary:hover {
          background: var(--ppt-color-primary-hover);
        }
      `}</style>
    </div>
  );
}

function StatsCards({ stats }: { stats: AgencyStats }) {
  const t = useTranslations('agency');
  const cards = [
    {
      label: t('statActiveListings'),
      value: stats.activeListings,
      total: stats.totalListings,
      icon: (
        <svg
          width="20"
          height="20"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          aria-hidden="true"
        >
          <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
        </svg>
      ),
      color: 'var(--ppt-color-primary)',
    },
    {
      label: t('statTotalViews'),
      value: formatNumber(stats.totalViews),
      icon: (
        <svg
          width="20"
          height="20"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          aria-hidden="true"
        >
          <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
          <circle cx="12" cy="12" r="3" />
        </svg>
      ),
      color: 'var(--ppt-color-success)',
    },
    {
      label: t('statInquiries'),
      value: stats.totalInquiries,
      icon: (
        <svg
          width="20"
          height="20"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          aria-hidden="true"
        >
          <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z" />
        </svg>
      ),
      color: 'var(--ppt-color-warning)',
    },
    {
      label: t('statConversionRate'),
      value: `${stats.conversionRate.toFixed(1)}%`,
      icon: (
        <svg
          width="20"
          height="20"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          aria-hidden="true"
        >
          <polyline points="23 6 13.5 15.5 8.5 10.5 1 18" />
          <polyline points="17 6 23 6 23 12" />
        </svg>
      ),
      color: '#8b5cf6',
    },
    {
      label: t('statRealtors'),
      value: stats.totalRealtors,
      icon: (
        <svg
          width="20"
          height="20"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          aria-hidden="true"
        >
          <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
          <circle cx="9" cy="7" r="4" />
          <path d="M23 21v-2a4 4 0 0 0-3-3.87" />
          <path d="M16 3.13a4 4 0 0 1 0 7.75" />
        </svg>
      ),
      color: '#ec4899',
    },
    {
      label: t('statAvgDaysOnMarket'),
      value: Math.round(stats.averageDaysOnMarket),
      icon: (
        <svg
          width="20"
          height="20"
          aria-hidden="true"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
        >
          <circle cx="12" cy="12" r="10" />
          <polyline points="12 6 12 12 16 14" />
        </svg>
      ),
      color: '#06b6d4',
    },
  ];

  return (
    <div className="stats-cards">
      {cards.map((card) => (
        <div key={card.label} className="stat-card">
          <div
            className="stat-icon"
            style={{ backgroundColor: `${card.color}15`, color: card.color }}
          >
            {card.icon}
          </div>
          <div className="stat-content">
            <span className="stat-value">{card.value}</span>
            {'total' in card && card.total && <span className="stat-total">/ {card.total}</span>}
            <span className="stat-label">{card.label}</span>
          </div>
        </div>
      ))}
      <style jsx>{`
        .stats-cards {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
          gap: 16px;
          margin-bottom: 32px;
        }

        .stat-card {
          display: flex;
          align-items: center;
          gap: 16px;
          padding: 20px;
          background: var(--ppt-bg-surface);
          border-radius: 12px;
          box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
        }

        .stat-icon {
          width: 48px;
          height: 48px;
          border-radius: 12px;
          display: flex;
          align-items: center;
          justify-content: center;
          flex-shrink: 0;
        }

        .stat-content {
          display: flex;
          flex-direction: column;
        }

        .stat-value {
          font-size: 1.5rem;
          font-weight: 700;
          color: var(--ppt-fg-primary);
          display: inline;
        }

        .stat-total {
          font-size: 1rem;
          color: var(--ppt-fg-subtle);
          font-weight: 500;
        }

        .stat-label {
          font-size: 0.875rem;
          color: var(--ppt-fg-muted);
          margin-top: 2px;
        }
      `}</style>
    </div>
  );
}

function PerformanceChart({ data }: { data: AgencyPerformance[] }) {
  const t = useTranslations('agency');
  const maxViews = Math.max(...data.map((d) => d.views), 1);
  const maxInquiries = Math.max(...data.map((d) => d.inquiries), 1);

  return (
    <div className="chart">
      <div className="chart-bars">
        {data.map((item, index) => (
          <div key={`${item.period}-${index}`} className="bar-group">
            <div className="bars">
              <div
                className="bar views"
                style={{ height: `${(item.views / maxViews) * 100}%` }}
                title={`${t('chartViews')}: ${item.views}`}
              />
              <div
                className="bar inquiries"
                style={{ height: `${(item.inquiries / maxInquiries) * 100}%` }}
                title={`${t('chartInquiries')}: ${item.inquiries}`}
              />
            </div>
            <span className="bar-label">{item.period}</span>
          </div>
        ))}
      </div>
      <div className="chart-legend">
        <span className="legend-item">
          <span className="legend-color views" />
          {t('chartViews')}
        </span>
        <span className="legend-item">
          <span className="legend-color inquiries" />
          {t('chartInquiries')}
        </span>
      </div>
      <style jsx>{`
        .chart {
          padding-top: 16px;
        }

        .chart-bars {
          display: flex;
          align-items: flex-end;
          gap: 8px;
          height: 200px;
          padding-bottom: 30px;
        }

        .bar-group {
          flex: 1;
          display: flex;
          flex-direction: column;
          align-items: center;
          height: 100%;
        }

        .bars {
          flex: 1;
          display: flex;
          align-items: flex-end;
          gap: 4px;
          width: 100%;
        }

        .bar {
          flex: 1;
          border-radius: 4px 4px 0 0;
          min-height: 4px;
          transition: height 0.3s;
        }

        .bar.views {
          background: var(--ppt-color-primary);
        }

        .bar.inquiries {
          background: var(--ppt-color-success);
        }

        .bar-label {
          font-size: 11px;
          color: var(--ppt-fg-subtle);
          margin-top: 8px;
          white-space: nowrap;
        }

        .chart-legend {
          display: flex;
          justify-content: center;
          gap: 24px;
          margin-top: 16px;
        }

        .legend-item {
          display: flex;
          align-items: center;
          gap: 8px;
          font-size: 14px;
          color: var(--ppt-fg-muted);
        }

        .legend-color {
          width: 12px;
          height: 12px;
          border-radius: 3px;
        }

        .legend-color.views {
          background: var(--ppt-color-primary);
        }

        .legend-color.inquiries {
          background: var(--ppt-color-success);
        }
      `}</style>
    </div>
  );
}

function RealtorLeaderboard({
  realtors,
}: {
  realtors: Array<{
    id: string;
    name: string;
    photoUrl?: string;
    totalSales: number;
    activeListings: number;
    rating?: number;
  }>;
}) {
  const t = useTranslations('agency');
  return (
    <div className="leaderboard">
      {realtors.length === 0 ? (
        <p className="empty">{t('leaderboardEmpty')}</p>
      ) : (
        realtors.map((realtor, index) => (
          <div key={realtor.id} className="realtor-row">
            <span className="rank">{index + 1}</span>
            <div className="avatar">
              {realtor.photoUrl ? (
                <img src={realtor.photoUrl} alt={realtor.name} />
              ) : (
                <span>{realtor.name.charAt(0)}</span>
              )}
            </div>
            <div className="realtor-info">
              <span className="realtor-name">{realtor.name}</span>
              <span className="realtor-stats">
                {t('leaderboardStats', {
                  sales: realtor.totalSales,
                  active: realtor.activeListings,
                })}
              </span>
            </div>
            {realtor.rating && (
              <div className="rating">
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="var(--ppt-color-warning)"
                  aria-hidden="true"
                >
                  <polygon points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2" />
                </svg>
                <span>{realtor.rating.toFixed(1)}</span>
              </div>
            )}
          </div>
        ))
      )}
      <style jsx>{`
        .leaderboard {
          display: flex;
          flex-direction: column;
          gap: 12px;
        }

        .empty {
          color: var(--ppt-fg-subtle);
          font-size: 14px;
          text-align: center;
          padding: 24px;
        }

        .realtor-row {
          display: flex;
          align-items: center;
          gap: 12px;
          padding: 12px;
          background: var(--ppt-bg-app);
          border-radius: 8px;
        }

        .rank {
          width: 24px;
          height: 24px;
          display: flex;
          align-items: center;
          justify-content: center;
          background: var(--ppt-border-default);
          border-radius: 50%;
          font-size: 12px;
          font-weight: 600;
          color: var(--ppt-fg-muted);
        }

        .avatar {
          width: 40px;
          height: 40px;
          border-radius: 50%;
          background: var(--ppt-color-primary);
          display: flex;
          align-items: center;
          justify-content: center;
          color: var(--ppt-fg-on-accent);
          font-weight: 600;
          overflow: hidden;
        }

        .avatar img {
          width: 100%;
          height: 100%;
          object-fit: cover;
        }

        .realtor-info {
          flex: 1;
          display: flex;
          flex-direction: column;
        }

        .realtor-name {
          font-weight: 500;
          color: var(--ppt-fg-primary);
        }

        .realtor-stats {
          font-size: 12px;
          color: var(--ppt-fg-muted);
        }

        .rating {
          display: flex;
          align-items: center;
          gap: 4px;
          font-size: 14px;
          font-weight: 500;
          color: var(--ppt-fg-secondary);
        }
      `}</style>
    </div>
  );
}

function DashboardSkeleton() {
  return (
    <div className="skeleton-dashboard">
      <div className="skeleton-header" />
      <div className="skeleton-stats">
        {[1, 2, 3, 4, 5, 6].map((i) => (
          <div key={`stat-skel-${i}`} className="skeleton-stat" />
        ))}
      </div>
      <style jsx>{`
        .skeleton-dashboard {
          padding: 24px;
          max-width: 1400px;
          margin: 0 auto;
        }
        .skeleton-header {
          height: 48px;
          width: 300px;
          background: var(--ppt-border-default);
          border-radius: 8px;
          margin-bottom: 32px;
        }
        .skeleton-stats {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
          gap: 16px;
        }
        .skeleton-stat {
          height: 100px;
          background: var(--ppt-border-default);
          border-radius: 12px;
        }
      `}</style>
    </div>
  );
}

function StatsCardsSkeleton() {
  return (
    <div className="skeleton-stats">
      {[1, 2, 3, 4, 5, 6].map((i) => (
        <div key={`stat-skel-${i}`} className="skeleton-stat" />
      ))}
      <style jsx>{`
        .skeleton-stats {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
          gap: 16px;
          margin-bottom: 32px;
        }
        .skeleton-stat {
          height: 100px;
          background: var(--ppt-border-default);
          border-radius: 12px;
        }
      `}</style>
    </div>
  );
}

function formatNumber(num: number): string {
  if (num >= 1000000) return `${(num / 1000000).toFixed(1)}M`;
  if (num >= 1000) return `${(num / 1000).toFixed(1)}K`;
  return num.toString();
}
