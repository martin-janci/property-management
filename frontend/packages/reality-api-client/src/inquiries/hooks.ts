/**
 * Reality Portal Inquiries Hooks
 *
 * React Query hooks for inquiries and viewing requests API (Epic 44).
 */

'use client';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { getApiBase, portalFetch } from '../config';
import type {
  AvailableViewingSlotsResponse,
  ContactMessageResponse,
  CreateInquiryRequest,
  Inquiry,
  InquiryStatus,
  PaginatedInquiries,
} from './types';

// Server wire shapes (reality-server `routes/inquiries.rs`). The app-facing
// `Inquiry` type predates the server, so rows are mapped below.
interface WireListingInquiry {
  id: string;
  listing_id: string;
  name: string;
  email: string;
  phone?: string | null;
  message: string;
  inquiry_type: string;
  preferred_time?: string | null;
  status: string;
  responded_at?: string | null;
  read_at?: string | null;
  created_at: string;
}

interface WireInquiryList {
  inquiries: WireListingInquiry[];
  total: number;
  page: number;
  limit: number;
}

interface WireInquiryMessage {
  id: string;
  sender_type: string;
  message: string;
  created_at: string;
}

interface WireInquiryDetail {
  inquiry: WireListingInquiry;
  messages: WireInquiryMessage[];
}

/** Server statuses: new, read, responded, scheduled, closed. */
function toStatus(raw: string, isViewing: boolean): InquiryStatus {
  switch (raw) {
    case 'responded':
      return 'responded';
    case 'scheduled':
      return 'scheduled';
    case 'closed':
      // A closed viewing request is one the buyer cancelled; a closed
      // question is simply finished.
      return isViewing ? 'cancelled' : 'completed';
    default:
      return 'pending';
  }
}

/** App status filter → server `status` query value (closest match). */
function toServerStatus(status: InquiryStatus): string {
  switch (status) {
    case 'pending':
      return 'new';
    case 'completed':
    case 'cancelled':
      return 'closed';
    default:
      return status;
  }
}

export function mapWireInquiry(
  row: WireListingInquiry,
  messages: WireInquiryMessage[] = []
): Inquiry {
  const isViewing = row.inquiry_type === 'viewing';
  const realtorReplies = messages.filter((m) => m.sender_type === 'realtor');
  const lastReply = realtorReplies[realtorReplies.length - 1];
  return {
    id: row.id,
    listingId: row.listing_id,
    // The inquiry row carries no listing snapshot; pages link by id.
    listingTitle: '',
    type: isViewing ? 'viewing_request' : 'general',
    status: toStatus(row.status, isViewing),
    message: row.message,
    name: row.name,
    email: row.email,
    phone: row.phone ?? undefined,
    preferredViewingSlots:
      isViewing && row.preferred_time ? [{ date: '', timeSlot: row.preferred_time }] : undefined,
    agentResponse: lastReply?.message,
    respondedAt: row.responded_at ?? lastReply?.created_at ?? undefined,
    createdAt: row.created_at,
    updatedAt: row.responded_at ?? row.read_at ?? row.created_at,
  };
}

function toPage(wire: WireInquiryList): PaginatedInquiries {
  const pageSize = Math.max(1, wire.limit);
  return {
    data: wire.inquiries.map((row) => mapWireInquiry(row)),
    total: wire.total,
    page: wire.page,
    pageSize,
    totalPages: Math.ceil(wire.total / pageSize),
  };
}

async function fetchInquiryPage(
  path: string,
  status: InquiryStatus | undefined,
  page: number,
  pageSize: number
): Promise<PaginatedInquiries> {
  const params = new URLSearchParams();
  params.set('page', String(page));
  params.set('limit', String(pageSize));
  if (status) params.set('status', toServerStatus(status));

  const response = await portalFetch(`${getApiBase()}${path}?${params}`, {
    credentials: 'include',
  });
  if (!response.ok) throw new Error('Failed to fetch inquiries');
  return toPage((await response.json()) as WireInquiryList);
}

// Inquiries the signed-in buyer sent (`GET /api/v1/inquiries/mine`).
export function useMyInquiries(status?: InquiryStatus, page = 1, pageSize = 20) {
  return useQuery({
    queryKey: ['my-inquiries', status, page, pageSize],
    queryFn: () => fetchInquiryPage('/api/v1/inquiries/mine', status, page, pageSize),
  });
}

// Inquiries addressed to the signed-in realtor (`GET /api/v1/inquiries`).
export function useReceivedInquiries(status?: InquiryStatus, page = 1, pageSize = 20) {
  return useQuery({
    queryKey: ['received-inquiries', status, page, pageSize],
    queryFn: () => fetchInquiryPage('/api/v1/inquiries', status, page, pageSize),
  });
}

// Single inquiry of the signed-in buyer, with the realtor's latest reply.
export function useInquiry(id: string) {
  return useQuery({
    queryKey: ['inquiry', id],
    queryFn: async (): Promise<Inquiry> => {
      const response = await portalFetch(
        `${getApiBase()}/api/v1/inquiries/mine/${encodeURIComponent(id)}`,
        { credentials: 'include' }
      );
      if (!response.ok) throw new Error('Failed to fetch inquiry');
      const wire = (await response.json()) as WireInquiryDetail;
      return mapWireInquiry(wire.inquiry, wire.messages);
    },
    enabled: !!id,
  });
}

// Create Inquiry (Contact Form)
//
// The public contact submission is listing-scoped on the server:
// `POST /api/v1/inquiries/contact/{listing_id}` with a `ContactMessageRequest`
// body (`name` / `email` / `phone` / `message`). When the buyer is signed in
// the bearer token links the inquiry to their account, so it shows up in
// `useMyInquiries`.
export function useCreateInquiry() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (data: CreateInquiryRequest): Promise<ContactMessageResponse> => {
      const response = await portalFetch(
        `${getApiBase()}/api/v1/inquiries/contact/${encodeURIComponent(data.listingId)}`,
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            name: data.name,
            email: data.email,
            phone: data.phone,
            message: data.message,
          }),
          credentials: 'include',
        }
      );
      if (!response.ok) throw new Error('Failed to create inquiry');
      return response.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['my-inquiries'] });
    },
  });
}

// Cancel one of the buyer's viewing requests
// (`POST /api/v1/inquiries/mine/{id}/cancel`; only viewing requests qualify).
export function useCancelInquiry() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (id: string): Promise<void> => {
      const response = await portalFetch(
        `${getApiBase()}/api/v1/inquiries/mine/${encodeURIComponent(id)}/cancel`,
        { method: 'POST', credentials: 'include' }
      );
      if (!response.ok) throw new Error('Failed to cancel inquiry');
    },
    onSuccess: (_data, id) => {
      queryClient.invalidateQueries({ queryKey: ['my-inquiries'] });
      queryClient.invalidateQueries({ queryKey: ['inquiry', id] });
    },
  });
}

// Get Available Viewing Slots for a Listing
export function useAvailableViewingSlots(listingId: string) {
  return useQuery({
    queryKey: ['available-viewing-slots', listingId],
    queryFn: async (): Promise<AvailableViewingSlotsResponse> => {
      const response = await portalFetch(
        `${getApiBase()}/api/v1/listings/${listingId}/viewing-slots`,
        {
          credentials: 'include',
        }
      );
      if (!response.ok) throw new Error('Failed to fetch available viewing slots');
      return response.json();
    },
    enabled: !!listingId,
    staleTime: 5 * 60 * 1000, // 5 minutes
  });
}
