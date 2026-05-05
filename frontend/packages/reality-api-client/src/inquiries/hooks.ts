/**
 * Reality Portal Inquiries Hooks
 *
 * React Query hooks for inquiries and viewing requests API (Epic 44).
 */

'use client';

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { getApiBase } from '../config';
import type {
  AvailableViewingSlotsResponse,
  CreateInquiryRequest,
  Inquiry,
  InquiryStatus,
  PaginatedInquiries,
} from './types';

// User's Inquiries
export function useMyInquiries(status?: InquiryStatus, page = 1, pageSize = 20) {
  return useQuery({
    queryKey: ['my-inquiries', status, page, pageSize],
    queryFn: async (): Promise<PaginatedInquiries> => {
      const params = new URLSearchParams();
      params.set('page', String(page));
      params.set('pageSize', String(pageSize));
      if (status) params.set('status', status);

      const response = await fetch(`${getApiBase()}/api/v1/inquiries?${params}`, {
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to fetch inquiries');
      return response.json();
    },
  });
}

// Single Inquiry
export function useInquiry(id: string) {
  return useQuery({
    queryKey: ['inquiry', id],
    queryFn: async (): Promise<Inquiry> => {
      const response = await fetch(`${getApiBase()}/api/v1/inquiries/${id}`, {
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to fetch inquiry');
      return response.json();
    },
    enabled: !!id,
  });
}

// Create Inquiry (Contact Form)
export function useCreateInquiry() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (data: CreateInquiryRequest): Promise<Inquiry> => {
      const response = await fetch(`${getApiBase()}/api/v1/inquiries`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data),
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to create inquiry');
      return response.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['my-inquiries'] });
    },
  });
}

// Cancel Inquiry
export function useCancelInquiry() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (id: string): Promise<void> => {
      const response = await fetch(`${getApiBase()}/api/v1/inquiries/${id}/cancel`, {
        method: 'POST',
        credentials: 'include',
      });
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
      const response = await fetch(`${getApiBase()}/api/v1/listings/${listingId}/viewing-slots`, {
        credentials: 'include',
      });
      if (!response.ok) throw new Error('Failed to fetch available viewing slots');
      return response.json();
    },
    enabled: !!listingId,
    staleTime: 5 * 60 * 1000, // 5 minutes
  });
}
