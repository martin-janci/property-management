/**
 * BookingsPage: "New booking" opens the create dialog (validated, submits the
 * form values) and cancelling a booking goes through a confirmation dialog.
 */
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { RentalBooking, RentalUnitOption } from '../types';
import { BookingsPage } from './BookingsPage';

const units: RentalUnitOption[] = [
  { id: 'u1', name: '1A', buildingId: 'b1', buildingName: 'Astra' },
];

const booking: RentalBooking = {
  id: 'bk1',
  unitId: 'u1',
  unitName: '1A',
  buildingId: '',
  guestName: 'Jana Nováková',
  checkIn: '2099-11-03',
  checkOut: '2099-11-05',
  status: 'confirmed',
  source: 'direct',
  guestCount: 2,
  createdAt: '',
  updatedAt: '',
};

function setup(overrides: Partial<Parameters<typeof BookingsPage>[0]> = {}) {
  const props = {
    bookings: [booking],
    total: 1,
    buildings: [{ id: 'b1', name: 'Astra' }],
    units,
    onFilterChange: vi.fn(),
    onViewBooking: vi.fn(),
    onEditBooking: vi.fn(),
    onCancelBooking: vi.fn(),
    onCheckIn: vi.fn(),
    onCheckOut: vi.fn(),
    onCreateBooking: vi.fn().mockResolvedValue(undefined),
    onBack: vi.fn(),
    ...overrides,
  };
  render(<BookingsPage {...props} />);
  return props;
}

describe('BookingsPage', () => {
  it('shows an alert with retry instead of the empty state when loading fails', () => {
    const onRetry = vi.fn();
    setup({ bookings: [], total: 0, isError: true, onRetry });
    expect(screen.getByRole('alert')).toHaveTextContent('Failed to load bookings');
    expect(screen.queryByText('No bookings found')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Retry' }));
    expect(onRetry).toHaveBeenCalledTimes(1);
  });

  it('pluralizes the result count', () => {
    setup();
    expect(screen.getByText('1 booking found')).toBeInTheDocument();
  });

  it('validates and submits the create-booking dialog', async () => {
    const props = setup();
    fireEvent.click(screen.getByRole('button', { name: 'Create Booking' }));
    const dialog = screen.getByRole('dialog');

    fireEvent.click(within(dialog).getByRole('button', { name: 'Create booking' }));
    expect(within(dialog).getByText('Select a unit.')).toBeInTheDocument();
    expect(within(dialog).getByText("Enter the guest's name.")).toBeInTheDocument();
    expect(props.onCreateBooking).not.toHaveBeenCalled();

    fireEvent.change(within(dialog).getByLabelText(/Unit/), { target: { value: 'u1' } });
    fireEvent.change(within(dialog).getByLabelText(/Guest name/), {
      target: { value: 'Peter Horváth' },
    });
    fireEvent.change(within(dialog).getByLabelText(/Check-in/), {
      target: { value: '2099-12-01' },
    });
    fireEvent.change(within(dialog).getByLabelText(/Check-out/), {
      target: { value: '2099-12-04' },
    });
    fireEvent.click(within(dialog).getByRole('button', { name: 'Create booking' }));

    await waitFor(() => expect(props.onCreateBooking).toHaveBeenCalledTimes(1));
    expect(props.onCreateBooking.mock.calls[0][0]).toMatchObject({
      unitId: 'u1',
      guestName: 'Peter Horváth',
      checkIn: '2099-12-01',
      checkOut: '2099-12-04',
      source: 'direct',
    });
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  });

  it('shows the unavailable message when the server answers 409', async () => {
    const conflict = Object.assign(new Error('Unit is not available'), { status: 409 });
    setup({ onCreateBooking: vi.fn().mockRejectedValue(conflict) });
    fireEvent.click(screen.getByRole('button', { name: 'Create Booking' }));
    const dialog = screen.getByRole('dialog');
    fireEvent.change(within(dialog).getByLabelText(/Unit/), { target: { value: 'u1' } });
    fireEvent.change(within(dialog).getByLabelText(/Guest name/), { target: { value: 'X' } });
    fireEvent.change(within(dialog).getByLabelText(/Check-in/), {
      target: { value: '2099-12-01' },
    });
    fireEvent.change(within(dialog).getByLabelText(/Check-out/), {
      target: { value: '2099-12-02' },
    });
    fireEvent.click(within(dialog).getByRole('button', { name: 'Create booking' }));
    expect(
      await within(dialog).findByText(/The unit is not available for these dates/)
    ).toBeInTheDocument();
  });

  it('cancels a booking only after confirming', () => {
    const props = setup();
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    const confirm = screen.getByRole('alertdialog');
    expect(within(confirm).getByText(/Jana Nováková/)).toBeInTheDocument();
    expect(props.onCancelBooking).not.toHaveBeenCalled();

    fireEvent.click(within(confirm).getByRole('button', { name: 'Keep booking' }));
    expect(props.onCancelBooking).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    fireEvent.click(
      within(screen.getByRole('alertdialog')).getByRole('button', { name: 'Cancel booking' })
    );
    expect(props.onCancelBooking).toHaveBeenCalledWith('bk1');
  });
});
