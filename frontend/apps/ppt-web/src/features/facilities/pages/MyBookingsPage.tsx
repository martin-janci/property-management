/**
 * My Bookings Page (Epic 56: Facility Booking).
 *
 * Shows the current user's facility bookings.
 */

import type { BookingStatus, BookingWithDetails } from '@ppt/api-client';
import { cancelBooking, getMyBookings } from '@ppt/api-client';
import { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useToast } from '../../../components';
import { BookingList, CancelBookingDialog } from '../components';

const PAGE_SIZE = 10;

export function MyBookingsPage() {
  const navigate = useNavigate();
  const { showToast } = useToast();

  const [bookings, setBookings] = useState<BookingWithDetails[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [isLoading, setIsLoading] = useState(true);
  const [statusFilter, setStatusFilter] = useState<BookingStatus | undefined>();

  const [cancelDialogBooking, setCancelDialogBooking] = useState<BookingWithDetails | null>(null);
  const [isCancelling, setIsCancelling] = useState(false);

  const fetchBookings = async (currentPage: number = page, status?: BookingStatus) => {
    setIsLoading(true);
    try {
      const offset = (currentPage - 1) * PAGE_SIZE;
      const response = await getMyBookings({
        status,
        limit: PAGE_SIZE,
        offset,
      });
      setBookings(response.items);
      setTotal(response.total);
    } catch (error) {
      showToast({
        type: 'error',
        title: 'Failed to load bookings',
        message: error instanceof Error ? error.message : 'An unexpected error occurred.',
      });
    } finally {
      setIsLoading(false);
    }
  };

  // biome-ignore lint/correctness/useExhaustiveDependencies: intentional trigger on page/statusFilter change
  useEffect(() => {
    fetchBookings(page, statusFilter);
  }, [page, statusFilter]);

  const handleStatusFilter = (status?: BookingStatus) => {
    setStatusFilter(status);
    setPage(1); // Reset to first page when filter changes
  };

  const handleView = (id: string) => {
    navigate(`/bookings/${id}`);
  };

  const handleCancelClick = (id: string) => {
    const booking = bookings.find((b) => b.id === id);
    if (booking) {
      setCancelDialogBooking(booking);
    }
  };

  const handleCancelConfirm = async (reason?: string) => {
    if (!cancelDialogBooking) return;

    setIsCancelling(true);
    try {
      await cancelBooking(cancelDialogBooking.id, reason ? { reason } : undefined);
      // Refresh bookings with current page and filter
      await fetchBookings(page, statusFilter);
      setCancelDialogBooking(null);
    } catch (error) {
      showToast({
        type: 'error',
        title: 'Failed to cancel booking',
        message: error instanceof Error ? error.message : 'An unexpected error occurred.',
      });
    } finally {
      setIsCancelling(false);
    }
  };

  const handlePageChange = (newPage: number) => {
    setPage(newPage);
  };

  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      <BookingList
        bookings={bookings}
        total={total}
        page={page}
        pageSize={PAGE_SIZE}
        isLoading={isLoading}
        isManager={false}
        title="My Bookings"
        onPageChange={handlePageChange}
        onStatusFilter={handleStatusFilter}
        onView={handleView}
        onCancel={handleCancelClick}
      />

      {cancelDialogBooking && (
        <CancelBookingDialog
          bookingId={cancelDialogBooking.id}
          facilityName={cancelDialogBooking.facility_name}
          startTime={cancelDialogBooking.start_time}
          onConfirm={handleCancelConfirm}
          onCancel={() => setCancelDialogBooking(null)}
          isSubmitting={isCancelling}
        />
      )}
    </div>
  );
}
