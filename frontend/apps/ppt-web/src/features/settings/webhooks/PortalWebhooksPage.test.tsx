/**
 * Regression test for the Portal Webhooks status surface (Gap 83-3 / Epic 105 —
 * Real Estate Portal Webhooks).
 *
 * Inbound portal webhooks were previously inbound-only with no manager
 * visibility. This page renders the org-scoped syndication dashboard (the
 * downstream product of those webhooks) via the `@ppt/api-client` `syndication`
 * module. Neither the page/route nor the `useSyndicationDashboard` hook existed
 * on `dev`, so this test fails without the wiring in this PR.
 */

/// <reference types="vitest/globals" />
import { render, screen, within } from '@testing-library/react';
import { PortalWebhooksPage } from './PortalWebhooksPage';

let dashboard: {
  data?:
    | {
        listings: Array<{
          listing_id: string;
          listing_title: string;
          listing_status: string;
          overall_status: string;
          total_views: number;
          total_inquiries: number;
          portal_statuses: Array<{
            portal: string;
            status: string;
            views: number;
            inquiries: number;
            last_activity_at: string | null;
            last_error: string | null;
          }>;
        }>;
        total: number;
        page: number;
        limit: number;
        organization_stats: {
          total_active: number;
          total_pending: number;
          total_failed: number;
          total_views: number;
          total_inquiries: number;
          by_portal: Array<{
            portal: string;
            active_count: number;
            pending_count: number;
            failed_count: number;
            views: number;
            inquiries: number;
          }>;
        };
      }
    | undefined;
  isLoading: boolean;
  isError: boolean;
} = { data: undefined, isLoading: true, isError: false };

vi.mock('@ppt/api-client', () => ({
  useSyndicationDashboard: () => dashboard,
}));

vi.mock('../../../contexts', () => ({
  useAuth: () => ({ user: { organizationId: 'org-1' } }),
}));

function loadedDashboard() {
  return {
    data: {
      listings: [
        {
          listing_id: 'l1',
          listing_title: 'Sunny 2BR Apartment',
          listing_status: 'active',
          overall_status: 'degraded',
          total_views: 42,
          total_inquiries: 5,
          portal_statuses: [
            {
              portal: 'sreality',
              status: 'synced',
              views: 30,
              inquiries: 4,
              last_activity_at: '2026-07-10T10:00:00Z',
              last_error: null,
            },
            {
              portal: 'bezrealitky',
              status: 'failed',
              views: 12,
              inquiries: 1,
              last_activity_at: '2026-07-09T09:00:00Z',
              last_error: 'signature mismatch',
            },
          ],
        },
      ],
      total: 1,
      page: 1,
      limit: 20,
      organization_stats: {
        total_active: 3,
        total_pending: 1,
        total_failed: 1,
        total_views: 42,
        total_inquiries: 5,
        by_portal: [
          {
            portal: 'sreality',
            active_count: 2,
            pending_count: 0,
            failed_count: 0,
            views: 30,
            inquiries: 4,
          },
          {
            portal: 'bezrealitky',
            active_count: 1,
            pending_count: 1,
            failed_count: 1,
            views: 12,
            inquiries: 1,
          },
        ],
      },
    },
    isLoading: false,
    isError: false,
  };
}

describe('PortalWebhooksPage (Gap 83-3)', () => {
  beforeEach(() => {
    dashboard = { data: undefined, isLoading: true, isError: false };
  });

  it('renders the portal webhooks heading', () => {
    render(<PortalWebhooksPage />);
    expect(screen.getByRole('heading', { level: 1, name: /portal webhooks/i })).toBeInTheDocument();
  });

  it('shows a loading state while fetching', () => {
    render(<PortalWebhooksPage />);
    expect(screen.getByText(/loading portal activity/i)).toBeInTheDocument();
  });

  it('shows an error state when the dashboard query fails', () => {
    dashboard = { data: undefined, isLoading: false, isError: true };
    render(<PortalWebhooksPage />);
    expect(screen.getByRole('alert')).toHaveTextContent(/failed to load portal webhook activity/i);
  });

  it('renders the per-portal breakdown table with view/inquiry counts', () => {
    dashboard = loadedDashboard();
    render(<PortalWebhooksPage />);
    const table = screen.getByRole('table');
    // Both known portals appear as rows with humanized labels.
    expect(within(table).getByText('Sreality')).toBeInTheDocument();
    expect(within(table).getByText('Bezrealitky')).toBeInTheDocument();
  });

  it('renders per-listing syndication status, health badge and last error', () => {
    dashboard = loadedDashboard();
    render(<PortalWebhooksPage />);
    expect(
      screen.getByRole('heading', { level: 3, name: /sunny 2br apartment/i })
    ).toBeInTheDocument();
    // Degraded health badge is surfaced.
    expect(screen.getByText(/degraded/i)).toBeInTheDocument();
    // The failed portal's last error is shown to the manager.
    expect(screen.getByText(/signature mismatch/i)).toBeInTheDocument();
  });

  it('does not render a pager when all listings fit on one page', () => {
    // total (1) <= limit (20) — single page, no pager (issue #2322).
    dashboard = loadedDashboard();
    render(<PortalWebhooksPage />);
    expect(screen.queryByRole('button', { name: /next/i })).not.toBeInTheDocument();
  });

  it('renders a prev/next pager with a range summary when listings span pages', () => {
    const base = loadedDashboard();
    // 45 listings at 20/page → 3 pages; page 1 shows "1–20 of 45".
    dashboard = { ...base, data: { ...base.data, total: 45, page: 1, limit: 20 } };
    render(<PortalWebhooksPage />);
    expect(screen.getByText(/showing 1–20 of 45 listings/i)).toBeInTheDocument();
    // On the first page, "Previous" is disabled and "Next" is enabled.
    expect(screen.getByRole('button', { name: /previous/i })).toBeDisabled();
    expect(screen.getByRole('button', { name: /next/i })).toBeEnabled();
  });
});
