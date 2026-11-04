/**
 * CalendarPage states: units loading / none / select prompt, events loading /
 * error / empty / data, and the block create + remove flows.
 */
import { fireEvent, render, screen, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { CalendarEvent, RentalUnitOption } from '../types';
import { CalendarPage } from './CalendarPage';

const units: RentalUnitOption[] = [
  { id: 'u1', name: '1A', buildingId: 'b1', buildingName: 'Astra' },
  { id: 'u2', name: '2B', buildingId: 'b1', buildingName: 'Astra' },
];

const events: CalendarEvent[] = [
  {
    id: 'blk-booking',
    unitId: 'u1',
    type: 'booking',
    title: 'Jana Nováková',
    startDate: '2026-11-03',
    endDate: '2026-11-05',
  },
  {
    id: 'blk-maint',
    unitId: 'u1',
    type: 'maintenance',
    blockReason: 'maintenance',
    title: 'Maintenance',
    startDate: '2026-11-10',
    endDate: '2026-11-12',
  },
];

function setup(overrides: Partial<Parameters<typeof CalendarPage>[0]> = {}) {
  const props = {
    units,
    events: [] as CalendarEvent[],
    selectedUnitId: 'u1',
    month: new Date(2026, 10, 4),
    onUnitChange: vi.fn(),
    onMonthChange: vi.fn(),
    onEventClick: vi.fn(),
    onAddBlock: vi.fn(),
    onDeleteBlock: vi.fn(),
    onBack: vi.fn(),
    ...overrides,
  };
  render(<CalendarPage {...props} />);
  return props;
}

describe('CalendarPage', () => {
  it('shows a spinner while units load', () => {
    setup({ units: [], isLoadingUnits: true });
    expect(screen.getByTestId('units-loading')).toBeInTheDocument();
  });

  it('shows an empty state when the organisation has no units', () => {
    setup({ units: [], selectedUnitId: '' });
    expect(screen.getByText('No units yet')).toBeInTheDocument();
    expect(screen.queryByRole('combobox')).not.toBeInTheDocument();
  });

  it('asks to pick a unit and lists units with their building', () => {
    const props = setup({ selectedUnitId: '' });
    expect(screen.getByText('Select a unit to view its calendar')).toBeInTheDocument();
    const select = screen.getByLabelText('Select unit');
    expect(within(select).getByRole('option', { name: '1A – Astra' })).toBeInTheDocument();
    fireEvent.change(select, { target: { value: 'u2' } });
    expect(props.onUnitChange).toHaveBeenCalledWith('u2');
  });

  it('shows the calendar spinner while events load', () => {
    setup({ isLoading: true });
    expect(screen.getByTestId('calendar-loading')).toBeInTheDocument();
  });

  it('shows an error with retry', () => {
    const onRetry = vi.fn();
    setup({ isError: true, onRetry });
    expect(screen.getByRole('alert')).toHaveTextContent('Could not load the calendar.');
    fireEvent.click(screen.getByRole('button', { name: 'Retry' }));
    expect(onRetry).toHaveBeenCalled();
  });

  it('shows the empty-month hint without events', () => {
    setup();
    expect(screen.getByText('No reservations or blocks in this month.')).toBeInTheDocument();
    expect(screen.getAllByTestId('calendar-day')).toHaveLength(42);
  });

  it('renders reservations over their nights and translated block labels', () => {
    const props = setup({ events });
    expect(screen.getByTestId('booking-count')).toHaveTextContent('1 reservation');
    expect(screen.getByTestId('block-count')).toHaveTextContent('1 block');
    // Nov 3 + Nov 4 (check-out Nov 5 is free)
    expect(screen.getAllByRole('button', { name: 'Jana Nováková' })).toHaveLength(2);
    expect(screen.getAllByRole('button', { name: 'Maintenance' }).length).toBeGreaterThan(0);
    fireEvent.click(screen.getAllByRole('button', { name: 'Jana Nováková' })[0]);
    expect(props.onEventClick).toHaveBeenCalledWith(events[0]);
  });

  it('removes a manual block from its detail dialog', () => {
    const props = setup({ events });
    fireEvent.click(screen.getAllByRole('button', { name: 'Maintenance' })[0]);
    const dialog = screen.getByRole('dialog');
    fireEvent.click(within(dialog).getByRole('button', { name: 'Remove block' }));
    expect(props.onDeleteBlock).toHaveBeenCalledWith('blk-maint');
    expect(props.onEventClick).not.toHaveBeenCalled();
  });

  it('creates a block with an inclusive range, reason and note', () => {
    const props = setup();
    fireEvent.click(screen.getByRole('button', { name: 'Block dates' }));
    const dialog = screen.getByRole('dialog');
    fireEvent.change(within(dialog).getByLabelText('From'), { target: { value: '2026-11-20' } });
    fireEvent.change(within(dialog).getByLabelText('To (including)'), {
      target: { value: '2026-11-22' },
    });
    expect(screen.getByTestId('block-days')).toHaveTextContent('3 days');
    fireEvent.change(within(dialog).getByLabelText('Reason'), { target: { value: 'maintenance' } });
    fireEvent.change(within(dialog).getByLabelText(/Note/), { target: { value: ' boiler ' } });
    fireEvent.click(within(dialog).getByRole('button', { name: 'Block dates' }));
    expect(props.onAddBlock).toHaveBeenCalledWith({
      unitId: 'u1',
      startDate: '2026-11-20',
      endDate: '2026-11-22',
      reason: 'maintenance',
      notes: 'boiler',
    });
  });

  it('rejects an end date before the start date', () => {
    const props = setup();
    fireEvent.click(screen.getByRole('button', { name: 'Block dates' }));
    const dialog = screen.getByRole('dialog');
    fireEvent.change(within(dialog).getByLabelText('From'), { target: { value: '2026-11-20' } });
    fireEvent.change(within(dialog).getByLabelText('To (including)'), {
      target: { value: '2026-11-18' },
    });
    expect(screen.getByTestId('block-days')).toHaveTextContent('must not be before');
    const confirm = within(dialog).getByRole('button', { name: 'Block dates' });
    expect(confirm).toBeDisabled();
    fireEvent.click(confirm);
    expect(props.onAddBlock).not.toHaveBeenCalled();
  });

  it('navigates months', () => {
    const props = setup();
    fireEvent.click(screen.getByRole('button', { name: 'Next month' }));
    const next = props.onMonthChange.mock.calls[0][0] as Date;
    expect([next.getFullYear(), next.getMonth()]).toEqual([2026, 11]);
  });
});
