/**
 * BookingDetailPage: edit dialog, cancel / delete-guest confirmations and the
 * add / edit guest dialog (all previously dead or unconfirmed actions).
 */
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { emptyBookingForm } from '../mappers';
import type { BookingWithGuests, RentalGuest } from '../types';
import { BookingDetailPage } from './BookingDetailPage';

const guest: RentalGuest = {
  id: 'g1',
  bookingId: 'bk1',
  firstName: 'Jana',
  lastName: 'Nováková',
  dateOfBirth: '1990-05-01',
  nationality: 'SK',
  documentType: 'id_card',
  documentNumber: 'EA123456',
  registrationStatus: 'pending',
  isPrimary: true,
  createdAt: '2026-10-01T00:00:00Z',
};

const booking: BookingWithGuests = {
  id: 'bk1',
  unitId: 'u1',
  unitName: '1A',
  buildingId: 'b1',
  buildingName: 'Astra',
  guestName: 'Jana Nováková',
  checkIn: '2099-11-03',
  checkOut: '2099-11-05',
  status: 'confirmed',
  source: 'direct',
  guestCount: 2,
  internalNotes: 'Key in the box',
  createdAt: '2026-10-01T00:00:00Z',
  updatedAt: '2026-10-01T00:00:00Z',
  guests: [guest],
};

const editForm = {
  ...emptyBookingForm('u1'),
  guestName: 'Jana Nováková',
  guestCount: 2,
  checkIn: '2099-11-03',
  checkOut: '2099-11-05',
  internalNotes: 'Key in the box',
};

function setup(overrides: Partial<Parameters<typeof BookingDetailPage>[0]> = {}) {
  const props = {
    booking,
    editForm,
    onBack: vi.fn(),
    onSaveBooking: vi.fn().mockResolvedValue(undefined),
    onCancel: vi.fn(),
    onCheckIn: vi.fn(),
    onCheckOut: vi.fn(),
    onAddGuest: vi.fn().mockResolvedValue(undefined),
    onEditGuest: vi.fn().mockResolvedValue(undefined),
    onRegisterGuest: vi.fn(),
    onDeleteGuest: vi.fn(),
    ...overrides,
  };
  render(<BookingDetailPage {...props} />);
  return props;
}

describe('BookingDetailPage', () => {
  it('shows the unit/building and the internal note', () => {
    setup();
    expect(screen.getByText('1A - Astra')).toBeInTheDocument();
    expect(screen.getByText('Key in the box')).toBeInTheDocument();
    expect(screen.getByText('1 guest pending registration')).toBeInTheDocument();
  });

  it('edits the booking through the dialog', async () => {
    const props = setup();
    fireEvent.click(screen.getByRole('button', { name: 'Edit booking' }));
    const dialog = screen.getByRole('dialog');
    expect(within(dialog).getByRole('heading', { name: 'Edit booking' })).toBeInTheDocument();
    expect(within(dialog).queryByLabelText(/Unit/)).not.toBeInTheDocument();

    fireEvent.change(within(dialog).getByLabelText('Number of guests'), {
      target: { value: '3' },
    });
    fireEvent.click(within(dialog).getByRole('button', { name: 'Save' }));
    await waitFor(() =>
      expect(props.onSaveBooking).toHaveBeenCalledWith(expect.objectContaining({ guestCount: 3 }))
    );
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  });

  it('opens the edit dialog right away when asked to', () => {
    setup({ initialEditing: true });
    expect(screen.getByRole('dialog')).toBeInTheDocument();
  });

  it('hides Edit without edit values', () => {
    setup({ editForm: undefined });
    expect(screen.queryByRole('button', { name: 'Edit booking' })).not.toBeInTheDocument();
  });

  it('cancels the booking only after confirming', () => {
    const props = setup();
    fireEvent.click(screen.getByRole('button', { name: 'Cancel booking' }));
    expect(props.onCancel).not.toHaveBeenCalled();
    fireEvent.click(
      within(screen.getByRole('alertdialog')).getByRole('button', { name: 'Cancel booking' })
    );
    expect(props.onCancel).toHaveBeenCalledWith('bk1');
  });

  it('deletes a guest only after confirming', () => {
    const props = setup();
    fireEvent.click(screen.getByRole('button', { name: 'Delete' }));
    const confirm = screen.getByRole('alertdialog');
    expect(within(confirm).getByText(/Jana Nováková will be removed/)).toBeInTheDocument();
    expect(props.onDeleteGuest).not.toHaveBeenCalled();
    fireEvent.click(within(confirm).getByRole('button', { name: 'Delete' }));
    expect(props.onDeleteGuest).toHaveBeenCalledWith('g1');
  });

  it('adds a guest to this booking from a dialog', () => {
    setup();
    fireEvent.click(screen.getByRole('button', { name: '+ Add Guest' }));
    const dialog = screen.getByRole('dialog');
    expect(within(dialog).getByRole('heading', { name: 'Add Guest' })).toBeInTheDocument();
    expect(within(dialog).getByLabelText(/First Name/)).toHaveValue('');
  });

  it('edits an existing guest in a dialog prefilled with their data', async () => {
    const props = setup();
    const card = screen.getByText('Jana Nováková', { selector: 'h4' }).closest('div.border');
    fireEvent.click(within(card as HTMLElement).getByRole('button', { name: 'Edit' }));
    const dialog = screen.getByRole('dialog');
    expect(within(dialog).getByLabelText(/First Name/)).toHaveValue('Jana');
    fireEvent.change(within(dialog).getByLabelText(/First Name/), { target: { value: 'Janka' } });
    fireEvent.submit(
      within(dialog)
        .getByLabelText(/First Name/)
        .closest('form') as HTMLFormElement
    );
    await waitFor(() =>
      expect(props.onEditGuest).toHaveBeenCalledWith(
        'g1',
        expect.objectContaining({ firstName: 'Janka' })
      )
    );
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  });
});
