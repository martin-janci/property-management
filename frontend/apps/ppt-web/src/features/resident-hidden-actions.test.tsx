/// <reference types="vitest/globals" />
/**
 * Manager-only actions are not rendered when the route omits their handler
 * (round-3 audit #14). The route wrappers pass these callbacks only for
 * `isManagerRole` users, so owners and tenants get read-only pages instead of
 * buttons that end in 403 or "Access denied".
 */
import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';

vi.mock('@ppt/api-client', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@ppt/api-client')>();
  return {
    ...actual,
    useBuildings: () => ({ data: { items: [{ id: 'b-1', name: 'Dom 1' }] }, isLoading: false }),
    useVotes: () => ({ data: [], isLoading: false, error: null }),
  };
});

const role = vi.hoisted(() => ({ current: 'tenant' as string }));

vi.mock('../contexts', () => ({
  useAuth: () => ({ user: { role: role.current } }),
}));
vi.mock('../hooks', () => ({
  useOrganization: () => ({ organizationId: 'org-1' }),
}));
vi.mock('../lib/api', () => ({
  getApiClient: () => ({
    get: vi.fn().mockResolvedValue({
      data: [
        {
          id: 'a-1',
          title: 'Draft notice',
          excerpt: null,
          status: 'draft',
          pinned: false,
          view_count: 0,
          reaction_count: 0,
          comment_count: 0,
          created_at: '2026-10-01T00:00:00Z',
        },
      ],
    }),
  }),
}));

import { PaymentManagementPage } from './financial/pages/PaymentManagementPage';
import { NewsListPage } from './news/pages/NewsListPage';
import { OutageList } from './outages/components/OutageList';
import { UnitPersonMonthsPage } from './person-months/pages/UnitPersonMonthsPage';
import { VotingPage } from './voting/pages/VotingPage';

describe('VotingPage', () => {
  it('hides "+ New vote" and "Create the first vote" without onCreateVote', () => {
    render(<VotingPage onOpenVote={vi.fn()} />);
    expect(screen.queryByRole('button', { name: /new vote/i })).not.toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: /create the first vote/i })
    ).not.toBeInTheDocument();
  });

  it('shows the create action when the route passes onCreateVote', () => {
    const onCreateVote = vi.fn();
    render(<VotingPage onOpenVote={vi.fn()} onCreateVote={onCreateVote} />);
    fireEvent.click(screen.getByRole('button', { name: /new vote/i }));
    expect(onCreateVote).toHaveBeenCalledOnce();
  });
});

describe('OutageList', () => {
  const props = {
    outages: [],
    total: 0,
    page: 1,
    pageSize: 10,
    onPageChange: vi.fn(),
    onStatusFilter: vi.fn(),
    onCommodityFilter: vi.fn(),
    onSeverityFilter: vi.fn(),
    onView: vi.fn(),
  };

  it('hides "Create Outage" without onCreate', () => {
    render(<OutageList {...props} />);
    expect(screen.queryByRole('button', { name: 'Create Outage' })).not.toBeInTheDocument();
  });

  it('shows "Create Outage" with onCreate', () => {
    render(<OutageList {...props} onCreate={vi.fn()} />);
    expect(screen.getByRole('button', { name: 'Create Outage' })).toBeInTheDocument();
  });
});

describe('PaymentManagementPage', () => {
  const props = {
    buildings: [],
    payments: [],
    total: 0,
    unallocatedPayments: [],
    unpaidInvoices: [],
    metrics: { totalReceived: 0, pendingReconciliation: 0, currency: 'EUR' },
    onNavigateToDetail: vi.fn(),
    onMatch: vi.fn(),
    onAutoMatch: vi.fn(),
    onFilterChange: vi.fn(),
  };

  it('hides "Record Payment" without onNavigateToRecord', () => {
    render(<PaymentManagementPage {...props} />);
    expect(screen.queryByRole('button', { name: 'Record Payment' })).not.toBeInTheDocument();
  });

  it('shows "Record Payment" with onNavigateToRecord', () => {
    render(<PaymentManagementPage {...props} onNavigateToRecord={vi.fn()} />);
    expect(screen.getByRole('button', { name: 'Record Payment' })).toBeInTheDocument();
  });
});

describe('UnitPersonMonthsPage', () => {
  const props = {
    buildingName: 'Dom 1',
    unitDesignation: '3B',
    personMonths: [
      {
        id: 'pm-1',
        unitId: 'u-1',
        unitDesignation: '3B',
        year: 2026,
        month: 10,
        personCount: 2,
        updatedAt: '2026-10-01T00:00:00Z',
      },
    ],
    yearlySummary: { year: 2026, totalPersonMonths: 2, averagePersons: 2, monthlyData: [] },
    selectedYear: 2026,
    onYearChange: vi.fn(),
    onBack: vi.fn(),
  };

  it('is read-only without onNavigateToEdit', () => {
    render(
      <MemoryRouter>
        <UnitPersonMonthsPage {...props} />
      </MemoryRouter>
    );
    expect(screen.queryByRole('button', { name: 'Add Entry' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Edit' })).not.toBeInTheDocument();
  });

  it('offers Add Entry and Edit with onNavigateToEdit', () => {
    render(
      <MemoryRouter>
        <UnitPersonMonthsPage {...props} onNavigateToEdit={vi.fn()} />
      </MemoryRouter>
    );
    expect(screen.getByRole('button', { name: 'Add Entry' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Edit' })).toBeInTheDocument();
  });
});

describe('NewsListPage', () => {
  function renderNews(asRole: string) {
    role.current = asRole;
    render(
      <MemoryRouter>
        <NewsListPage />
      </MemoryRouter>
    );
  }

  it.each(['owner', 'tenant'])('gives "%s" read-only news', async (asRole) => {
    renderNews(asRole);
    await screen.findByText('Draft notice');
    expect(screen.queryByRole('link', { name: 'Create Article' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /^Edit article/ })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /^Delete article/ })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Pin article' })).not.toBeInTheDocument();
  });

  it('gives a manager the write actions', async () => {
    renderNews('manager');
    await screen.findByText('Draft notice');
    expect(screen.getByRole('link', { name: 'Create Article' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /^Edit article/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Pin article' })).toBeInTheDocument();
  });
});
