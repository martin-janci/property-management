/**
 * OutageList Component Tests (Epic 121, Story 3)
 *
 * Tests for the OutageList component including:
 * - Rendering list of outages
 * - Empty and loading states
 * - Filter controls
 * - Pagination
 */

/// <reference types="vitest/globals" />
import type { OutageSummary } from '@ppt/api-client';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { OutageList } from './OutageList';

// Mock react-i18next
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => {
      const translations: Record<string, string> = {
        'outages.title': 'Outages',
        'outages.createNew': 'Create New',
        'outages.allStatuses': 'All Statuses',
        'outages.allCommodities': 'All Commodities',
        'outages.allSeverities': 'All Severities',
        'outages.noResults': 'No outages found',
        'outages.status.planned': 'Planned',
        'outages.status.ongoing': 'Ongoing',
        'outages.status.resolved': 'Resolved',
        'outages.status.cancelled': 'Cancelled',
        'outages.commodity.electricity': 'Electricity',
        'outages.commodity.water': 'Water',
        'outages.commodity.gas': 'Gas',
        'outages.commodity.heating': 'Heating',
        'outages.commodity.internet': 'Internet',
        'outages.commodity.other': 'Other',
        'outages.severity.critical': 'Critical',
        'outages.severity.major': 'Major',
        'outages.severity.minor': 'Minor',
        'outages.severity.informational': 'Informational',
        'common.showing': 'Showing',
        'common.to': 'to',
        'common.of': 'of',
        'common.previous': 'Previous',
        'common.next': 'Next',
        'common.view': 'View',
        'common.edit': 'Edit',
        'outages.building': 'building',
        'outages.buildings': 'buildings',
        'outages.scheduled': 'Scheduled',
      };
      return translations[key] || key;
    },
    i18n: { language: 'en' },
  }),
}));

const createOutage = (id: string, title: string): OutageSummary => ({
  id,
  title,
  commodity: 'electricity',
  severity: 'minor',
  status: 'planned',
  affectedBuildingsCount: 2,
  scheduledStart: '2026-01-15T09:00:00Z',
  createdAt: '2026-01-10T10:00:00Z',
});

const defaultProps = {
  outages: [],
  total: 0,
  page: 1,
  pageSize: 10,
  onPageChange: vi.fn(),
  onStatusFilter: vi.fn(),
  onCommodityFilter: vi.fn(),
  onSeverityFilter: vi.fn(),
  onView: vi.fn(),
  onEdit: vi.fn(),
  onCreate: vi.fn(),
};

describe('OutageList Component', () => {
  describe('Header', () => {
    it('renders title', () => {
      render(<OutageList {...defaultProps} />);
      expect(screen.getByText('Outages')).toBeInTheDocument();
    });

    it('renders create new button', () => {
      render(<OutageList {...defaultProps} />);
      expect(screen.getByText('Create New')).toBeInTheDocument();
    });

    it('calls onCreate when create button clicked', async () => {
      const user = userEvent.setup();
      const onCreate = vi.fn();
      render(<OutageList {...defaultProps} onCreate={onCreate} />);

      await user.click(screen.getByText('Create New'));

      expect(onCreate).toHaveBeenCalled();
    });
  });

  describe('Empty State', () => {
    it('renders empty message when no outages', () => {
      render(<OutageList {...defaultProps} outages={[]} />);
      expect(screen.getByText('No outages found')).toBeInTheDocument();
    });
  });

  describe('Loading State', () => {
    it('shows loading spinner when isLoading is true', () => {
      render(<OutageList {...defaultProps} isLoading />);
      // Look for the spinner element
      const spinner = document.querySelector('.animate-spin');
      expect(spinner).toBeInTheDocument();
    });

    it('does not show empty message when loading', () => {
      render(<OutageList {...defaultProps} isLoading />);
      expect(screen.queryByText('No outages found')).not.toBeInTheDocument();
    });
  });

  describe('Outage List', () => {
    it('renders all outages', () => {
      const outages = [
        createOutage('1', 'Outage One'),
        createOutage('2', 'Outage Two'),
        createOutage('3', 'Outage Three'),
      ];
      render(<OutageList {...defaultProps} outages={outages} total={3} />);

      expect(screen.getByText('Outage One')).toBeInTheDocument();
      expect(screen.getByText('Outage Two')).toBeInTheDocument();
      expect(screen.getByText('Outage Three')).toBeInTheDocument();
    });
  });

  describe('Filters', () => {
    it('renders status filter dropdown', () => {
      render(<OutageList {...defaultProps} />);
      expect(screen.getByDisplayValue('All Statuses')).toBeInTheDocument();
    });

    it('renders commodity filter dropdown', () => {
      render(<OutageList {...defaultProps} />);
      expect(screen.getByDisplayValue('All Commodities')).toBeInTheDocument();
    });

    it('renders severity filter dropdown', () => {
      render(<OutageList {...defaultProps} />);
      expect(screen.getByDisplayValue('All Severities')).toBeInTheDocument();
    });

    it('calls onStatusFilter when status changed', async () => {
      const user = userEvent.setup();
      const onStatusFilter = vi.fn();
      render(<OutageList {...defaultProps} onStatusFilter={onStatusFilter} />);

      const statusSelect = screen.getByDisplayValue('All Statuses');
      await user.selectOptions(statusSelect, 'planned');

      expect(onStatusFilter).toHaveBeenCalledWith('planned');
    });

    it('calls onCommodityFilter when commodity changed', async () => {
      const user = userEvent.setup();
      const onCommodityFilter = vi.fn();
      render(<OutageList {...defaultProps} onCommodityFilter={onCommodityFilter} />);

      const commoditySelect = screen.getByDisplayValue('All Commodities');
      await user.selectOptions(commoditySelect, 'water');

      expect(onCommodityFilter).toHaveBeenCalledWith('water');
    });

    it('calls onSeverityFilter when severity changed', async () => {
      const user = userEvent.setup();
      const onSeverityFilter = vi.fn();
      render(<OutageList {...defaultProps} onSeverityFilter={onSeverityFilter} />);

      const severitySelect = screen.getByDisplayValue('All Severities');
      await user.selectOptions(severitySelect, 'critical');

      expect(onSeverityFilter).toHaveBeenCalledWith('critical');
    });

    it('calls filter with undefined when cleared', async () => {
      const user = userEvent.setup();
      const onStatusFilter = vi.fn();
      render(<OutageList {...defaultProps} onStatusFilter={onStatusFilter} />);

      const statusSelect = screen.getByDisplayValue('All Statuses');
      // First select a value, then clear it
      await user.selectOptions(statusSelect, 'planned');
      await user.selectOptions(statusSelect, '');

      expect(onStatusFilter).toHaveBeenLastCalledWith(undefined);
    });
  });

  describe('Pagination', () => {
    it('does not show pagination when total pages is 1', () => {
      render(<OutageList {...defaultProps} total={5} pageSize={10} />);
      expect(screen.queryByText('Previous')).not.toBeInTheDocument();
      expect(screen.queryByText('Next')).not.toBeInTheDocument();
    });

    it('shows pagination when total pages is more than 1', () => {
      render(<OutageList {...defaultProps} total={25} pageSize={10} />);
      expect(screen.getByText('Previous')).toBeInTheDocument();
      expect(screen.getByText('Next')).toBeInTheDocument();
    });

    it('shows correct showing count', () => {
      const outages = [createOutage('1', 'Test')];
      render(<OutageList {...defaultProps} outages={outages} total={25} page={1} pageSize={10} />);
      expect(screen.getByText(/Showing.*1.*to.*10.*of.*25/)).toBeInTheDocument();
    });

    it('disables previous button on first page', () => {
      render(<OutageList {...defaultProps} total={25} page={1} pageSize={10} />);
      const prevButton = screen.getByText('Previous');
      expect(prevButton).toBeDisabled();
    });

    it('disables next button on last page', () => {
      render(<OutageList {...defaultProps} total={25} page={3} pageSize={10} />);
      const nextButton = screen.getByText('Next');
      expect(nextButton).toBeDisabled();
    });

    it('calls onPageChange with next page when next clicked', async () => {
      const user = userEvent.setup();
      const onPageChange = vi.fn();
      render(
        <OutageList
          {...defaultProps}
          total={25}
          page={1}
          pageSize={10}
          onPageChange={onPageChange}
        />
      );

      await user.click(screen.getByText('Next'));

      expect(onPageChange).toHaveBeenCalledWith(2);
    });

    it('calls onPageChange with previous page when previous clicked', async () => {
      const user = userEvent.setup();
      const onPageChange = vi.fn();
      render(
        <OutageList
          {...defaultProps}
          total={25}
          page={2}
          pageSize={10}
          onPageChange={onPageChange}
        />
      );

      await user.click(screen.getByText('Previous'));

      expect(onPageChange).toHaveBeenCalledWith(1);
    });
  });
});
