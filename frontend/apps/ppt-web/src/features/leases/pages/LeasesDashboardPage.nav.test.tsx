/// <reference types="vitest/globals" />
/**
 * LeasesDashboardPage nav (nav-fixes): lease templates and violations had no
 * inbound link; the dashboard header now links to both.
 */
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { LeasesDashboardPage } from './LeasesDashboardPage';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (k: string) => k, i18n: { language: 'en' } }),
}));

const statistics = {
  totalLeases: 0,
  activeLeases: 0,
  expiringLeases: 0,
  expiredLeases: 0,
  pendingApplications: 0,
  occupancyRate: 0,
  totalMonthlyRent: 0,
  overduePayments: 0,
  currency: 'EUR',
};
const expirationOverview = {
  expiringIn30Days: [],
  expiringIn60Days: [],
  expiringIn90Days: [],
  expired: [],
};

describe('LeasesDashboardPage header nav', () => {
  it('navigates to lease templates and violations', async () => {
    const onTemplates = vi.fn();
    const onViolations = vi.fn();
    render(
      <LeasesDashboardPage
        statistics={statistics}
        expirationOverview={expirationOverview}
        onNavigateToLease={vi.fn()}
        onNavigateToLeases={vi.fn()}
        onNavigateToApplications={vi.fn()}
        onNavigateToCreateLease={vi.fn()}
        onNavigateToTemplates={onTemplates}
        onNavigateToViolations={onViolations}
      />
    );
    await userEvent.click(screen.getByRole('button', { name: 'leases.nav.templates' }));
    await userEvent.click(screen.getByRole('button', { name: 'leases.nav.violations' }));
    expect(onTemplates).toHaveBeenCalledTimes(1);
    expect(onViolations).toHaveBeenCalledTimes(1);
  });
});
