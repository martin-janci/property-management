/**
 * Reality Portal Listing-Report Hooks
 *
 * React Query mutation for submitting an abuse / problem report about a
 * listing (UC-23). Mirrors the shape of the inquiries hooks (anonymous,
 * listing-scoped POST) and posts to reality-server's already-live
 * `POST /api/v1/reports` endpoint.
 */

'use client';

import { useMutation } from '@tanstack/react-query';

import { getApiBase } from '../config';
import type { SubmitReportRequest, SubmitReportResponse } from './types';

/**
 * Error carrying the HTTP status so callers can distinguish a rate-limit
 * (429) or missing-listing (404) from a generic failure and surface the
 * right message to the user.
 */
export class SubmitReportError extends Error {
  readonly status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = 'SubmitReportError';
    this.status = status;
  }
}

// Submit a listing report (anonymous-friendly — session cookie is sent when
// present so an authenticated reporter gets the faster moderation SLA).
export function useSubmitReport() {
  return useMutation<SubmitReportResponse, SubmitReportError, SubmitReportRequest>({
    mutationFn: async (data: SubmitReportRequest): Promise<SubmitReportResponse> => {
      const response = await fetch(`${getApiBase()}/api/v1/reports`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data),
        credentials: 'include',
      });
      if (!response.ok) {
        throw new SubmitReportError(
          response.status,
          `Failed to submit report (${response.status})`
        );
      }
      return response.json();
    },
  });
}
