/**
 * OutageCard Component Tests (Epic 121, Story 3)
 *
 * Tests for the OutageCard component including:
 * - Rendering different outage severities and statuses
 * - Commodity icons display
 * - Action button visibility based on status
 * - User interactions (view, edit)
 */

/// <reference types="vitest/globals" />
import type { OutageSummary } from '@ppt/api-client';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { OutageCard } from './OutageCard';

// Mock react-i18next
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const translations: Record<string, string> = {
        'outages.severity.critical': 'Critical',
        'outages.severity.major': 'Major',
        'outages.severity.minor': 'Minor',
        'outages.severity.informational': 'Informational',
        'outages.status.planned': 'Planned',
        'outages.status.ongoing': 'Ongoing',
        'outages.status.resolved': 'Resolved',
        'outages.status.cancelled': 'Cancelled',
        'outages.building': 'building',
        'outages.buildings': 'buildings',
        'outages.scheduled': 'Scheduled',
        'common.view': 'View',
        'common.edit': 'Edit',
      };
      return translations[key] || key;
    },
    i18n: { language: 'en' },
  }),
}));

const createOutage = (overrides: Partial<OutageSummary> = {}): OutageSummary => ({
  id: 'outage-1',
  title: 'Test Outage',
  commodity: 'electricity',
  severity: 'minor',
  status: 'planned',
  affectedBuildingsCount: 3,
  scheduledStart: '2026-01-15T09:00:00Z',
  scheduledEnd: '2026-01-15T17:00:00Z',
  createdAt: '2026-01-10T10:00:00Z',
  ...overrides,
});

describe('OutageCard Component', () => {
  describe('Rendering', () => {
    it('renders outage title', () => {
      render(<OutageCard outage={createOutage({ title: 'Power Outage Downtown' })} />);
      expect(screen.getByText('Power Outage Downtown')).toBeInTheDocument();
    });

    it('renders severity badge', () => {
      render(<OutageCard outage={createOutage({ severity: 'critical' })} />);
      expect(screen.getByText('Critical')).toBeInTheDocument();
    });

    it('renders status badge', () => {
      render(<OutageCard outage={createOutage({ status: 'ongoing' })} />);
      expect(screen.getByText('Ongoing')).toBeInTheDocument();
    });

    it('renders affected buildings count', () => {
      render(<OutageCard outage={createOutage({ affectedBuildingsCount: 5 })} />);
      expect(screen.getByText('5 buildings')).toBeInTheDocument();
    });

    it('renders singular building when count is 1', () => {
      render(<OutageCard outage={createOutage({ affectedBuildingsCount: 1 })} />);
      expect(screen.getByText('1 building')).toBeInTheDocument();
    });

    it('renders scheduled time', () => {
      render(<OutageCard outage={createOutage()} />);
      expect(screen.getByText('Scheduled:')).toBeInTheDocument();
    });
  });

  describe('Commodity Icons', () => {
    it('renders electricity icon', () => {
      render(<OutageCard outage={createOutage({ commodity: 'electricity' })} />);
      expect(screen.getByRole('img', { name: 'electricity' })).toBeInTheDocument();
    });

    it('renders water icon', () => {
      render(<OutageCard outage={createOutage({ commodity: 'water' })} />);
      expect(screen.getByRole('img', { name: 'water' })).toBeInTheDocument();
    });

    it('renders gas icon', () => {
      render(<OutageCard outage={createOutage({ commodity: 'gas' })} />);
      expect(screen.getByRole('img', { name: 'gas' })).toBeInTheDocument();
    });

    it('renders heating icon', () => {
      render(<OutageCard outage={createOutage({ commodity: 'heating' })} />);
      expect(screen.getByRole('img', { name: 'heating' })).toBeInTheDocument();
    });

    it('renders internet icon', () => {
      render(<OutageCard outage={createOutage({ commodity: 'internet' })} />);
      expect(screen.getByRole('img', { name: 'internet' })).toBeInTheDocument();
    });

    it('renders other icon', () => {
      render(<OutageCard outage={createOutage({ commodity: 'other' })} />);
      expect(screen.getByRole('img', { name: 'other' })).toBeInTheDocument();
    });
  });

  describe('Action Buttons', () => {
    it('shows view button when onView callback provided', () => {
      const onView = vi.fn();
      render(<OutageCard outage={createOutage()} onView={onView} />);
      expect(screen.getByText('View')).toBeInTheDocument();
    });

    it('does not show view button when onView not provided', () => {
      render(<OutageCard outage={createOutage()} />);
      expect(screen.queryByText('View')).not.toBeInTheDocument();
    });

    it('shows edit button for planned outages', () => {
      const onEdit = vi.fn();
      render(<OutageCard outage={createOutage({ status: 'planned' })} onEdit={onEdit} />);
      expect(screen.getByText('Edit')).toBeInTheDocument();
    });

    it('shows edit button for ongoing outages', () => {
      const onEdit = vi.fn();
      render(<OutageCard outage={createOutage({ status: 'ongoing' })} onEdit={onEdit} />);
      expect(screen.getByText('Edit')).toBeInTheDocument();
    });

    it('does not show edit button for resolved outages', () => {
      const onEdit = vi.fn();
      render(<OutageCard outage={createOutage({ status: 'resolved' })} onEdit={onEdit} />);
      expect(screen.queryByText('Edit')).not.toBeInTheDocument();
    });

    it('does not show edit button for cancelled outages', () => {
      const onEdit = vi.fn();
      render(<OutageCard outage={createOutage({ status: 'cancelled' })} onEdit={onEdit} />);
      expect(screen.queryByText('Edit')).not.toBeInTheDocument();
    });
  });

  describe('User Interactions', () => {
    it('calls onView with outage id when view button clicked', async () => {
      const user = userEvent.setup();
      const onView = vi.fn();
      render(<OutageCard outage={createOutage({ id: 'outage-123' })} onView={onView} />);

      await user.click(screen.getByText('View'));

      expect(onView).toHaveBeenCalledWith('outage-123');
    });

    it('calls onEdit with outage id when edit button clicked', async () => {
      const user = userEvent.setup();
      const onEdit = vi.fn();
      render(<OutageCard outage={createOutage({ id: 'outage-456' })} onEdit={onEdit} />);

      await user.click(screen.getByText('Edit'));

      expect(onEdit).toHaveBeenCalledWith('outage-456');
    });
  });

  describe('Severity Styles', () => {
    it('applies critical severity style', () => {
      render(<OutageCard outage={createOutage({ severity: 'critical' })} />);
      const severityText = screen.getByText('Critical');
      expect(severityText).toHaveClass('text-red-600');
    });

    it('applies major severity style', () => {
      render(<OutageCard outage={createOutage({ severity: 'major' })} />);
      const severityText = screen.getByText('Major');
      expect(severityText).toHaveClass('text-orange-500');
    });

    it('applies minor severity style', () => {
      render(<OutageCard outage={createOutage({ severity: 'minor' })} />);
      const severityText = screen.getByText('Minor');
      expect(severityText).toHaveClass('text-blue-500');
    });

    it('applies informational severity style', () => {
      render(<OutageCard outage={createOutage({ severity: 'informational' })} />);
      const severityText = screen.getByText('Informational');
      expect(severityText).toHaveClass('text-gray-500');
    });
  });
});
