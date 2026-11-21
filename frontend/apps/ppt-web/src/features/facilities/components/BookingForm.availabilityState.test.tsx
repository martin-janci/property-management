/// <reference types="vitest/globals" />
/**
 * Booking availability feedback (code-review: silent availability failure).
 *
 * The BookFacilityPage availability fetch used to swallow its error — on
 * failure every time stayed bookable with no hint to the user. The form now
 * surfaces a loading status while availability is being checked and an alert
 * when the check failed, so a user never books against unknown availability
 * thinking the slots are confirmed free.
 */
import { render, screen } from '@testing-library/react';
import type { Facility } from '@ppt/api-client';
import { BookingForm } from './BookingForm';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: 'en' },
  }),
}));

const facility = {
  id: 'f-1',
  name: 'Community Room',
  is_bookable: true,
} as unknown as Facility;

function renderForm(props: Partial<React.ComponentProps<typeof BookingForm>> = {}) {
  return render(
    <BookingForm
      facility={facility}
      selectedDate="2026-03-10"
      onDateChange={() => {}}
      onSubmit={() => {}}
      onCancel={() => {}}
      {...props}
    />,
  );
}

describe('BookingForm availability feedback', () => {
  it('shows an alert when the availability check failed', () => {
    renderForm({ availabilityError: 'facilities.bookings.loadAvailabilityFailed' });
    const alert = screen.getByRole('alert');
    expect(alert).toHaveTextContent('facilities.bookings.loadAvailabilityFailed');
  });

  it('shows a checking status while availability is loading', () => {
    renderForm({ isCheckingAvailability: true });
    expect(screen.getByRole('status')).toHaveTextContent(
      'facilities.bookings.form.checkingAvailability',
    );
  });

  it('shows neither status nor alert when availability loaded cleanly', () => {
    renderForm();
    expect(screen.queryByRole('alert')).toBeNull();
    expect(screen.queryByRole('status')).toBeNull();
  });
});
