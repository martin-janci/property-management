/// <reference types="vitest/globals" />
/**
 * PendingBookingsPage manager-gating tests.
 *
 * Regression cover for the hardcoded-`isManager` bug: the page previously
 * passed `isManager={true}` to <BookingList> unconditionally, so the
 * approve/reject manager actions rendered for every authenticated user
 * regardless of role. These tests pin that `isManager` is now derived from
 * the real auth role via `isManagerRole(user?.role)`.
 */
import { render, waitFor } from '@testing-library/react';
import { PendingBookingsPage } from './PendingBookingsPage';

// ── api-client ──
const mockListPendingBookings = vi.fn();
vi.mock('@ppt/api-client', () => ({
  listPendingBookings: (...args: unknown[]) => mockListPendingBookings(...args),
  approveBooking: vi.fn(),
  rejectBooking: vi.fn(),
}));

// ── toast ──
vi.mock('../../../components', () => ({
  useToast: () => ({ showToast: vi.fn() }),
}));

// ── router ──
vi.mock('react-router-dom', () => ({
  useNavigate: () => vi.fn(),
  useParams: () => ({ buildingId: 'b-1' }),
}));

// ── auth: role is driven per-test ──
let mockRole: string | undefined;
vi.mock('../../../contexts', () => ({
  useAuth: () => ({ user: { role: mockRole } }),
}));

// ── capture the isManager prop handed to <BookingList> ──
const capturedIsManager: Array<boolean | undefined> = [];
vi.mock('../components', () => ({
  BookingList: ({ isManager }: { isManager?: boolean }) => {
    capturedIsManager.push(isManager);
    return <div data-testid="booking-list" />;
  },
  RejectBookingDialog: () => null,
}));

beforeEach(() => {
  vi.clearAllMocks();
  capturedIsManager.length = 0;
  mockListPendingBookings.mockResolvedValue({ items: [], total: 0 });
});

describe('PendingBookingsPage manager gating', () => {
  it('passes isManager=true for a manager-equivalent role', async () => {
    mockRole = 'manager';
    render(<PendingBookingsPage />);

    await waitFor(() => expect(capturedIsManager.length).toBeGreaterThan(0));
    expect(capturedIsManager.at(-1)).toBe(true);
  });

  it('passes isManager=false for a non-manager role (resident)', async () => {
    mockRole = 'resident';
    render(<PendingBookingsPage />);

    await waitFor(() => expect(capturedIsManager.length).toBeGreaterThan(0));
    expect(capturedIsManager.at(-1)).toBe(false);
  });
});
