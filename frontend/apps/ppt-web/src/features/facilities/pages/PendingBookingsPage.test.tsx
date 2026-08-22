/// <reference types="vitest/globals" />
/**
 * PendingBookingsPage error-surfacing tests.
 *
 * Regression cover for the silent-error bug: fetch / approve / reject failures
 * previously only hit `console.error`, so the manager saw no feedback. These
 * tests pin that each failure now raises an error toast via useToast().
 *
 * Covered:
 *  (a) list fetch failure   → error toast ("Failed to load pending bookings")
 *  (b) approve failure      → error toast ("Failed to approve booking", Error.message)
 *  (c) reject failure       → error toast ("Failed to reject booking")
 *  (d) non-Error rejection  → error toast falls back to generic message
 */
import { render, screen, waitFor } from '@testing-library/react';
import { PendingBookingsPage } from './PendingBookingsPage';

// ── api-client: list + approve + reject ──
const mockListPendingBookings = vi.fn();
const mockApproveBooking = vi.fn();
const mockRejectBooking = vi.fn();
vi.mock('@ppt/api-client', () => ({
  listPendingBookings: (...args: unknown[]) => mockListPendingBookings(...args),
  approveBooking: (...args: unknown[]) => mockApproveBooking(...args),
  rejectBooking: (...args: unknown[]) => mockRejectBooking(...args),
}));

// ── toast ──
const mockShowToast = vi.fn();
vi.mock('../../../components', () => ({
  useToast: () => ({ showToast: mockShowToast }),
}));

// ── router ──
vi.mock('react-router-dom', () => ({
  useNavigate: () => vi.fn(),
  useParams: () => ({ buildingId: 'b-1' }),
}));

// ── stub child components so we can drive the page handlers ──
vi.mock('../components', () => ({
  BookingList: ({
    onApprove,
    onReject,
  }: {
    onApprove?: (id: string) => void;
    onReject?: (id: string) => void;
  }) => (
    <div>
      <button type="button" onClick={() => onApprove?.('bk-1')}>
        approve
      </button>
      <button type="button" onClick={() => onReject?.('bk-1')}>
        reject
      </button>
    </div>
  ),
  RejectBookingDialog: ({ onConfirm }: { onConfirm: (reason: string) => void }) => (
    <button type="button" onClick={() => onConfirm('no')}>
      confirm-reject
    </button>
  ),
}));

const ONE_BOOKING = {
  items: [
    {
      id: 'bk-1',
      facility_name: 'Gym',
      start_time: '2026-09-01T10:00:00Z',
    },
  ],
  total: 1,
};

beforeEach(() => {
  vi.clearAllMocks();
  mockListPendingBookings.mockResolvedValue(ONE_BOOKING);
  mockApproveBooking.mockResolvedValue(undefined);
  mockRejectBooking.mockResolvedValue(undefined);
});

describe('PendingBookingsPage error surfacing', () => {
  it('(a) list fetch failure → error toast', async () => {
    mockListPendingBookings.mockRejectedValueOnce(new Error('list boom'));
    render(<PendingBookingsPage />);

    await waitFor(() => {
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'error',
          title: 'Failed to load pending bookings',
          message: 'list boom',
        })
      );
    });
  });

  it('(b) approve failure → error toast with Error.message', async () => {
    mockApproveBooking.mockRejectedValueOnce(new Error('approve boom'));
    render(<PendingBookingsPage />);

    await waitFor(() => expect(mockListPendingBookings).toHaveBeenCalled());
    screen.getByRole('button', { name: /^approve$/i }).click();

    await waitFor(() => {
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'error',
          title: 'Failed to approve booking',
          message: 'approve boom',
        })
      );
    });
  });

  it('(c) reject failure → error toast', async () => {
    mockRejectBooking.mockRejectedValueOnce(new Error('reject boom'));
    render(<PendingBookingsPage />);

    await waitFor(() => expect(mockListPendingBookings).toHaveBeenCalled());
    // open the reject dialog, then confirm
    screen.getByRole('button', { name: /^reject$/i }).click();
    (await screen.findByRole('button', { name: /confirm-reject/i })).click();

    await waitFor(() => {
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'error',
          title: 'Failed to reject booking',
          message: 'reject boom',
        })
      );
    });
  });

  it('(d) non-Error rejection → error toast falls back to generic message', async () => {
    mockApproveBooking.mockRejectedValueOnce('network down');
    render(<PendingBookingsPage />);

    await waitFor(() => expect(mockListPendingBookings).toHaveBeenCalled());
    screen.getByRole('button', { name: /^approve$/i }).click();

    await waitFor(() => {
      expect(mockShowToast).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'error',
          title: 'Failed to approve booking',
          message: 'An unexpected error occurred.',
        })
      );
    });
  });
});
