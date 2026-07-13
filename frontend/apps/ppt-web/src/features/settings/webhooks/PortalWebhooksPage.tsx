/**
 * Portal Webhooks management/status page (Gap 83-3 / Epic 105 — Real Estate
 * Portal Webhooks).
 *
 * Inbound portal webhooks (`POST /api/v1/webhooks/portals/*`) record views and
 * inquiries from external real-estate portals (Reality Portal, Sreality,
 * Bezrealitky, Nehnutelnosti) and update per-listing syndication state. Until
 * now there was no manager-facing surface to see any of that — the webhooks
 * were inbound-only with zero visibility.
 *
 * This page renders that visibility surface from the org-scoped syndication
 * read endpoints (via the `@ppt/api-client` `syndication` module):
 *   GET /api/v1/listings/syndication/dashboard  — per-listing status + org stats
 *
 * It is read-only: the webhook receivers themselves are unauthenticated
 * HMAC-verified endpoints owned by the portals, so there is nothing to mutate
 * here — the manager sees delivery outcomes (views/inquiries, sync health,
 * last activity, last error) per portal.
 */

import { useSyndicationDashboard } from '@ppt/api-client';
import type React from 'react';
import { useTranslation } from 'react-i18next';
import { useAuth } from '../../../contexts';

/** Human-readable portal labels for the known real-estate portals. */
const PORTAL_LABELS: Record<string, string> = {
  reality_portal: 'Reality Portal',
  sreality: 'Sreality',
  bezrealitky: 'Bezrealitky',
  nehnutelnosti: 'Nehnutelnosti',
};

function portalLabel(portal: string): string {
  return PORTAL_LABELS[portal] ?? portal;
}

function formatDate(value: string | null | undefined): string {
  if (!value) return '—';
  const d = new Date(value);
  return Number.isNaN(d.getTime()) ? '—' : d.toLocaleString();
}

/** Tailwind classes + label for a listing's overall syndication health badge. */
function healthBadge(
  status: string,
  t: (key: string, opts?: Record<string, unknown>) => string
): { cls: string; label: string } {
  switch (status) {
    case 'healthy':
      return {
        cls: 'bg-green-100 text-green-700',
        label: t('settings.portalWebhooks.health.healthy', { defaultValue: 'Healthy' }),
      };
    case 'degraded':
      return {
        cls: 'bg-yellow-100 text-yellow-800',
        label: t('settings.portalWebhooks.health.degraded', { defaultValue: 'Degraded' }),
      };
    case 'unhealthy':
      return {
        cls: 'bg-red-100 text-red-700',
        label: t('settings.portalWebhooks.health.unhealthy', { defaultValue: 'Unhealthy' }),
      };
    default:
      return {
        cls: 'bg-gray-100 text-gray-600',
        label: t('settings.portalWebhooks.health.notConfigured', {
          defaultValue: 'Not configured',
        }),
      };
  }
}

const StatCard: React.FC<{ label: string; value: number | string }> = ({ label, value }) => (
  <div className="rounded-lg border border-gray-200 bg-white p-4">
    <div className="text-2xl font-bold text-gray-900">{value}</div>
    <div className="text-xs text-gray-500">{label}</div>
  </div>
);

export const PortalWebhooksPage: React.FC = () => {
  const { t } = useTranslation();
  const { user } = useAuth();
  const organizationId = user?.organizationId ?? '';

  const dashboard = useSyndicationDashboard();
  const stats = dashboard.data?.organization_stats;
  const listings = dashboard.data?.listings ?? [];

  return (
    <div className="max-w-4xl mx-auto p-6 space-y-6">
      <header>
        <h1 className="text-2xl font-bold">
          {t('settings.portalWebhooks.title', { defaultValue: 'Portal Webhooks' })}
        </h1>
        <p className="text-sm" style={{ color: 'var(--color-text-secondary)' }}>
          {t('settings.portalWebhooks.subtitle', {
            defaultValue:
              'Delivery status and activity from real-estate portal webhooks — views, inquiries and sync health per portal.',
          })}
        </p>
      </header>

      {!organizationId && (
        <div
          className="rounded-lg border border-yellow-300 bg-yellow-50 p-4 text-sm text-yellow-800"
          role="alert"
        >
          {t('settings.portalWebhooks.noOrganization', {
            defaultValue:
              'No organization is selected for your account, so portal webhook activity cannot be shown.',
          })}
        </div>
      )}

      {dashboard.isLoading && (
        <p className="text-sm text-gray-500">
          {t('settings.portalWebhooks.loading', { defaultValue: 'Loading portal activity…' })}
        </p>
      )}

      {dashboard.isError && (
        <div className="rounded-md bg-red-50 p-3 text-sm text-red-700" role="alert">
          {t('settings.portalWebhooks.loadError', {
            defaultValue: 'Failed to load portal webhook activity.',
          })}
        </div>
      )}

      {/* Organization-wide summary */}
      {stats && (
        <section className="space-y-4" aria-labelledby="pw-summary-heading">
          <h2 id="pw-summary-heading" className="text-lg font-semibold">
            {t('settings.portalWebhooks.summary.title', { defaultValue: 'Summary' })}
          </h2>
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-5">
            <StatCard
              label={t('settings.portalWebhooks.summary.active', { defaultValue: 'Active' })}
              value={stats.total_active}
            />
            <StatCard
              label={t('settings.portalWebhooks.summary.pending', { defaultValue: 'Pending' })}
              value={stats.total_pending}
            />
            <StatCard
              label={t('settings.portalWebhooks.summary.failed', { defaultValue: 'Failed' })}
              value={stats.total_failed}
            />
            <StatCard
              label={t('settings.portalWebhooks.summary.views', { defaultValue: 'Views' })}
              value={stats.total_views}
            />
            <StatCard
              label={t('settings.portalWebhooks.summary.inquiries', { defaultValue: 'Inquiries' })}
              value={stats.total_inquiries}
            />
          </div>

          {/* Per-portal breakdown */}
          <div className="overflow-x-auto rounded-lg border border-gray-200">
            <table className="min-w-full divide-y divide-gray-200 text-sm">
              <caption className="sr-only">
                {t('settings.portalWebhooks.byPortal.caption', {
                  defaultValue: 'Syndication activity by portal',
                })}
              </caption>
              <thead className="bg-gray-50 text-left text-xs uppercase text-gray-500">
                <tr>
                  <th scope="col" className="px-4 py-2">
                    {t('settings.portalWebhooks.byPortal.portal', { defaultValue: 'Portal' })}
                  </th>
                  <th scope="col" className="px-4 py-2">
                    {t('settings.portalWebhooks.summary.active', { defaultValue: 'Active' })}
                  </th>
                  <th scope="col" className="px-4 py-2">
                    {t('settings.portalWebhooks.summary.pending', { defaultValue: 'Pending' })}
                  </th>
                  <th scope="col" className="px-4 py-2">
                    {t('settings.portalWebhooks.summary.failed', { defaultValue: 'Failed' })}
                  </th>
                  <th scope="col" className="px-4 py-2">
                    {t('settings.portalWebhooks.summary.views', { defaultValue: 'Views' })}
                  </th>
                  <th scope="col" className="px-4 py-2">
                    {t('settings.portalWebhooks.summary.inquiries', { defaultValue: 'Inquiries' })}
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100 bg-white">
                {stats.by_portal.length === 0 ? (
                  <tr>
                    <td className="px-4 py-3 text-gray-500" colSpan={6}>
                      {t('settings.portalWebhooks.byPortal.empty', {
                        defaultValue: 'No portal activity recorded yet.',
                      })}
                    </td>
                  </tr>
                ) : (
                  stats.by_portal.map((p) => (
                    <tr key={p.portal}>
                      <td className="px-4 py-2 font-medium text-gray-900">
                        {portalLabel(p.portal)}
                      </td>
                      <td className="px-4 py-2 text-gray-700">{p.active_count}</td>
                      <td className="px-4 py-2 text-gray-700">{p.pending_count}</td>
                      <td className="px-4 py-2 text-gray-700">{p.failed_count}</td>
                      <td className="px-4 py-2 text-gray-700">{p.views}</td>
                      <td className="px-4 py-2 text-gray-700">{p.inquiries}</td>
                    </tr>
                  ))
                )}
              </tbody>
            </table>
          </div>
        </section>
      )}

      {/* Per-listing syndication status */}
      {dashboard.data && (
        <section className="space-y-4" aria-labelledby="pw-listings-heading">
          <h2 id="pw-listings-heading" className="text-lg font-semibold">
            {t('settings.portalWebhooks.listings.title', { defaultValue: 'Listings' })}
          </h2>
          {listings.length === 0 ? (
            <p className="text-sm text-gray-500">
              {t('settings.portalWebhooks.listings.empty', {
                defaultValue: 'No syndicated listings yet.',
              })}
            </p>
          ) : (
            <ul className="space-y-3">
              {listings.map((listing) => {
                const badge = healthBadge(listing.overall_status, t);
                return (
                  <li
                    key={listing.listing_id}
                    className="rounded-lg border border-gray-200 bg-white p-4 space-y-3"
                  >
                    <div className="flex items-start justify-between gap-4">
                      <div>
                        <h3 className="font-semibold text-gray-900">{listing.listing_title}</h3>
                        <p className="text-xs text-gray-500">
                          {t('settings.portalWebhooks.listings.activity', {
                            defaultValue: '{{views}} views · {{inquiries}} inquiries',
                            views: listing.total_views,
                            inquiries: listing.total_inquiries,
                          })}
                        </p>
                      </div>
                      <span
                        className={`shrink-0 rounded-full px-3 py-1 text-xs font-medium ${badge.cls}`}
                      >
                        {badge.label}
                      </span>
                    </div>
                    {listing.portal_statuses.length > 0 && (
                      <ul className="divide-y divide-gray-100 rounded-md border border-gray-100">
                        {listing.portal_statuses.map((ps) => (
                          <li
                            key={`${listing.listing_id}-${ps.portal}`}
                            className="flex flex-wrap items-center justify-between gap-2 px-3 py-2 text-sm"
                          >
                            <span className="font-medium text-gray-900">
                              {portalLabel(ps.portal)}
                            </span>
                            <span className="text-gray-500">
                              {t('settings.portalWebhooks.listings.portalActivity', {
                                defaultValue: '{{views}} views · {{inquiries}} inquiries',
                                views: ps.views,
                                inquiries: ps.inquiries,
                              })}
                            </span>
                            <span className="text-xs text-gray-400">
                              {t('settings.portalWebhooks.listings.lastActivity', {
                                defaultValue: 'Last activity: {{when}}',
                                when: formatDate(ps.last_activity_at),
                              })}
                            </span>
                            {ps.last_error && (
                              <span className="w-full text-xs text-red-600" role="alert">
                                {t('settings.portalWebhooks.listings.lastError', {
                                  defaultValue: 'Last error: {{error}}',
                                  error: ps.last_error,
                                })}
                              </span>
                            )}
                          </li>
                        ))}
                      </ul>
                    )}
                  </li>
                );
              })}
            </ul>
          )}
        </section>
      )}
    </div>
  );
};

export default PortalWebhooksPage;
