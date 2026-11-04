/**
 * TaxReportPage (daňový výkaz) states: buildings loading / none, preview
 * flow with data and with no guests, saving, stored reports list.
 */
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { GuestReportPreviewData, GuestReportSummaryItem } from '../types';
import { TaxReportPage } from './TaxReportPage';

const preview: GuestReportPreviewData = {
  buildingId: 'b1',
  buildingName: 'Astra',
  periodStart: '2026-10-01',
  periodEnd: '2026-10-31',
  totalGuests: 5,
  bookingsCount: 2,
  byNationality: [
    { nationality: 'SK', countryName: 'Slovakia', count: 3, percentage: 60 },
    { nationality: 'UNK', countryName: 'Unknown', count: 2, percentage: 40 },
  ],
};

const savedReport: GuestReportSummaryItem = {
  id: 'r1',
  buildingId: 'b1',
  buildingName: 'Astra',
  periodType: 'monthly',
  periodStart: '2026-09-01',
  periodEnd: '2026-09-30',
  authorityCode: 'SK_UHUL',
  authorityName: 'UHUL Slovakia',
  totalGuests: 7,
  status: 'generated',
};

function setup(overrides: Partial<Parameters<typeof TaxReportPage>[0]> = {}) {
  const props = {
    buildings: [{ id: 'b1', name: 'Astra' }],
    reports: [] as GuestReportSummaryItem[],
    onPreview: vi.fn().mockResolvedValue(preview),
    onSaveReport: vi.fn().mockResolvedValue(undefined),
    onSubmitReport: vi.fn(),
    onBack: vi.fn(),
    now: new Date(2026, 10, 4),
    ...overrides,
  };
  render(<TaxReportPage {...props} />);
  return props;
}

describe('TaxReportPage', () => {
  it('shows spinners while buildings and reports load', () => {
    setup({ buildings: [], isLoadingBuildings: true, isLoadingReports: true });
    expect(screen.getByTestId('buildings-loading')).toBeInTheDocument();
    expect(screen.getByTestId('reports-loading')).toBeInTheDocument();
  });

  it('shows empty states without buildings or saved reports', () => {
    setup({ buildings: [] });
    expect(screen.getByText('No buildings yet')).toBeInTheDocument();
    expect(screen.getByTestId('no-saved-reports')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Show preview' })).not.toBeInTheDocument();
  });

  it('previews the previous month for the chosen building', async () => {
    const props = setup();
    const button = screen.getByRole('button', { name: 'Show preview' });
    expect(button).toBeDisabled();
    fireEvent.change(screen.getByLabelText('Building'), { target: { value: 'b1' } });
    fireEvent.click(button);
    await screen.findByTestId('tax-report-preview');
    expect(props.onPreview).toHaveBeenCalledWith({
      buildingId: 'b1',
      authorityCode: 'SK_UHUL',
      periodType: 'monthly',
      periodStart: '2026-10-01',
      periodEnd: '2026-10-31',
    });
    expect(screen.getByTestId('total-guests')).toHaveTextContent('5');
    expect(screen.getByTestId('total-bookings')).toHaveTextContent('2');
    expect(screen.getByText('Slovakia')).toBeInTheDocument();
    expect(screen.getByText('Not stated')).toBeInTheDocument();
    expect(screen.getByText('60%')).toBeInTheDocument();
  });

  it('switches to a quarter period', async () => {
    const props = setup();
    fireEvent.change(screen.getByLabelText('Building'), { target: { value: 'b1' } });
    fireEvent.click(screen.getByLabelText('Quarter', { selector: 'input' }));
    fireEvent.change(screen.getByLabelText('Quarter', { selector: 'select' }), {
      target: { value: '1' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Show preview' }));
    await waitFor(() => expect(props.onPreview).toHaveBeenCalled());
    expect(props.onPreview.mock.calls[0][0]).toMatchObject({
      periodType: 'quarterly',
      periodStart: '2026-01-01',
      periodEnd: '2026-03-31',
    });
  });

  it('shows the no-guests state for an empty period', async () => {
    setup({
      onPreview: vi
        .fn()
        .mockResolvedValue({ ...preview, totalGuests: 0, bookingsCount: 0, byNationality: [] }),
    });
    fireEvent.change(screen.getByLabelText('Building'), { target: { value: 'b1' } });
    fireEvent.click(screen.getByRole('button', { name: 'Show preview' }));
    expect(await screen.findByTestId('no-guests')).toBeInTheDocument();
  });

  it('saves the previewed report and reports failures', async () => {
    const onSaveReport = vi.fn().mockRejectedValueOnce(new Error('x')).mockResolvedValue(undefined);
    setup({ onSaveReport });
    fireEvent.change(screen.getByLabelText('Building'), { target: { value: 'b1' } });
    fireEvent.click(screen.getByRole('button', { name: 'Show preview' }));
    await screen.findByTestId('tax-report-preview');
    fireEvent.click(screen.getByRole('button', { name: 'Save report' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not save the report');
    fireEvent.click(screen.getByRole('button', { name: 'Save report' }));
    expect(await screen.findByText('Report saved.')).toBeInTheDocument();
    expect(onSaveReport).toHaveBeenLastCalledWith(
      expect.objectContaining({ buildingId: 'b1', authorityCode: 'SK_UHUL' })
    );
  });

  it('shows a preview error', async () => {
    setup({ onPreview: vi.fn().mockRejectedValue(new Error('boom')) });
    fireEvent.change(screen.getByLabelText('Building'), { target: { value: 'b1' } });
    fireEvent.click(screen.getByRole('button', { name: 'Show preview' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Could not create the preview');
  });

  it('lists stored reports and marks one as submitted', () => {
    const props = setup({
      reports: [
        savedReport,
        {
          ...savedReport,
          id: 'r2',
          status: 'submitted',
          submittedAt: '2026-10-05T10:00:00Z',
          fileUrl: 'https://files.example/r2.pdf',
        },
      ],
    });
    const rows = screen.getAllByTestId('saved-report-row');
    expect(rows).toHaveLength(2);
    expect(within(rows[0]).getByText('Prepared')).toBeInTheDocument();
    expect(
      within(rows[0]).getByText('Slovakia – municipality (accommodation tax)')
    ).toBeInTheDocument();
    fireEvent.click(within(rows[0]).getByRole('button', { name: 'Mark as submitted' }));
    expect(props.onSubmitReport).toHaveBeenCalledWith('r1');
    expect(within(rows[1]).queryByRole('button', { name: 'Mark as submitted' })).toBeNull();
    expect(within(rows[1]).getByRole('link', { name: 'Download' })).toHaveAttribute(
      'href',
      'https://files.example/r2.pdf'
    );
  });
});
