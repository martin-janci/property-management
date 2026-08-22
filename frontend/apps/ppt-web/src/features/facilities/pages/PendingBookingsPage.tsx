/**
 * Pending Bookings Page (Epic 56: Facility Booking).
 *
 * Shows pending bookings for manager approval.
 */

import type { BookingWithDetails } from '@ppt/api-client';
import { approveBooking, listPendingBookings, rejectBooking } from '@ppt/api-client';
import { useEffect, useState } from 'react';
import { useNavigate, useParams } from 'react-router-dom';
import { useToast } from '../../../components';
import { BookingList, RejectBookingDialog } from '../components';

const PAGE_SIZE = 10;

export function PendingBookingsPage() {
  const { buildingId } = useParams<{ buildingId: string }>();
  const navigate = useNavigate();
  const { showToast } = useToast();

  const [bookings, setBookings] = useState<BookingWithDetails[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [isLoading, setIsLoading] = useState(true);

  const [rejectDialogBooking, setRejectDialogBooking] = useState<BookingWithDetails | null>(null);
  const [isProcessing, setIsProcessing] = useState(false);

  const refreshBookings = async (currentPage: number = page) => {
    if (!buildingId) return;

    setIsLoading(true);
    try {
      const offset = (currentPage - 1) * PAGE_SIZE;
      const response = await listPendingBookings(buildingId, {
        limit: PAGE_SIZE,
        offset,
      });
      setBookings(response.items);
      setTotal(response.total);
    } catch (error) {
      showToast({
        type: 'error',
        title: 'Failed to load pending bookings',
        message: error instanceof Error ? error.message : 'An unexpected error occurred.',
      });
    } finally {
      setIsLoading(false);
    }
  };

  // biome-ignore lint/correctness/useExhaustiveDependencies: intentional trigger on buildingId/page change
  useEffect(() => {
    refreshBookings(page);
  }, [buildingId, page]);

  const handleView = (id: string) => {
    navigate(`/bookings/${id}`);
  };

  const handleApprove = async (id: string) => {
    setIsProcessing(true);
    try {
      await approveBooking(id);
      await refreshBookings();
    } catch (error) {
      showToast({
        type: 'error',
        title: 'Failed to approve booking',
        message: error instanceof Error ? error.message : 'An unexpected error occurred.',
      });
    } finally {
      setIsProcessing(false);
    }
  };

  const handleRejectClick = (id: string) => {
    const booking = bookings.find((b) => b.id === id);
    if (booking) {
      setRejectDialogBooking(booking);
    }
  };

  const handleRejectConfirm = async (reason: string) => {
    if (!rejectDialogBooking) return;

    setIsProcessing(true);
    try {
      await rejectBooking(rejectDialogBooking.id, { reason });
      await refreshBookings();
      setRejectDialogBooking(null);
    } catch (error) {
      showToast({
        type: 'error',
        title: 'Failed to reject booking',
        message: error instanceof Error ? error.message : 'An unexpected error occurred.',
      });
    } finally {
      setIsProcessing(false);
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
        isLoading={isLoading || isProcessing}
        isManager={true}
        title="Pending Booking Approvals"
        onPageChange={handlePageChange}
        onView={handleView}
        onApprove={handleApprove}
        onReject={handleRejectClick}
      />

      {bookings.length === 0 && !isLoading && (
        <div className="mt-4 p-4 bg-green-50 border border-green-200 rounded-md">
          <p className="text-green-700">All caught up! No pending bookings to review.</p>
        </div>
      )}

      {rejectDialogBooking && (
        <RejectBookingDialog
          bookingId={rejectDialogBooking.id}
          facilityName={rejectDialogBooking.facility_name}
          onConfirm={handleRejectConfirm}
          onCancel={() => setRejectDialogBooking(null)}
          isSubmitting={isProcessing}
        />
      )}
    </div>
  );
}
