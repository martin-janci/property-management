/**
 * Reality Portal Inquiries Types
 *
 * TypeScript types for inquiries and viewing requests API (Epic 44).
 */

// Inquiry Status
export type InquiryStatus = 'pending' | 'responded' | 'scheduled' | 'completed' | 'cancelled';

// Inquiry Type
export type InquiryType = 'general' | 'viewing_request' | 'price_negotiation' | 'availability';

// Viewing Request
export interface ViewingSlot {
  date: string;
  timeSlot: string; // e.g., "10:00-11:00"
}

// Inquiry (contact form submission)
export interface Inquiry {
  id: string;
  listingId: string;
  listingTitle: string;
  listingPhoto?: string;
  type: InquiryType;
  status: InquiryStatus;
  message: string;
  name: string;
  email: string;
  phone?: string;
  preferredViewingSlots?: ViewingSlot[];
  scheduledViewing?: ViewingSlot;
  agentResponse?: string;
  respondedAt?: string;
  createdAt: string;
  updatedAt: string;
}

// Create Inquiry Request
export interface CreateInquiryRequest {
  listingId: string;
  type: InquiryType;
  message: string;
  name: string;
  email: string;
  phone?: string;
  preferredViewingSlots?: ViewingSlot[];
}

// Contact Message Response
// Returned by POST /api/v1/inquiries/contact/{listingId}. The reality-server
// handler answers 201 with a confirmation message and the new inquiry id in
// snake_case (no camelCase rename on `ContactMessageResponse`).
export interface ContactMessageResponse {
  message: string;
  inquiry_id: string;
}

// Paginated Response
export interface PaginatedInquiries {
  data: Inquiry[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

// Available Viewing Slots Response
export interface AvailableViewingSlotsResponse {
  listingId: string;
  slots: ViewingSlot[];
}
