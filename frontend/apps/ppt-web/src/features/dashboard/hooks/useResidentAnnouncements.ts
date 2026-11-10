/**
 * Newest published announcements visible to the signed-in user.
 *
 * Uses `GET /api/v1/announcements/published`, which every member may call and
 * which the backend filters by the announcement's targeting. The manager list
 * (`GET /announcements?status=published`) returns 403 for owners and tenants,
 * so the resident dashboard never showed announcements (round-3 audit #17).
 *
 * The query key matches the shared published-list cache
 * (`[...announcementKeys.published(), { page, pageSize }]`).
 */
import {
  ApiError,
  announcementKeys,
  authenticatedFetch,
  type BackendAnnouncementListResponse,
  buildAnnouncementQuery,
  toPaginatedResponse,
} from '@ppt/api-client';
import { useQuery } from '@tanstack/react-query';

export function useResidentAnnouncements(pageSize: number) {
  const paging = { page: undefined, pageSize };
  return useQuery({
    queryKey: [...announcementKeys.published(), paging] as const,
    queryFn: async () => {
      const response = await authenticatedFetch(
        `/api/v1/announcements/published${buildAnnouncementQuery(paging)}`
      );
      if (!response.ok) {
        throw new ApiError(response.status, `HTTP ${response.status}`);
      }
      return toPaginatedResponse(
        (await response.json()) as BackendAnnouncementListResponse,
        paging
      );
    },
  });
}
