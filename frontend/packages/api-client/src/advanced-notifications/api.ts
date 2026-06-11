/**
 * Advanced Notifications API (Epic 40)
 *
 * Uses the centralised authenticated fetch helper so auth token handling is
 * consistent with the rest of the api-client (see lib/fetch.ts, #486).
 */

import { authenticatedFetchJson } from '../lib/fetch';
import type {
  AdvancedPreferencesResponse,
  CategoryPreferencesResponse,
  DigestResponse,
  GroupingResponse,
  NotificationCategory,
  QuietHoursResponse,
  UpdateCategoryPreferenceRequest,
  UpdateDigestRequest,
  UpdateGroupingRequest,
  UpdateQuietHoursRequest,
} from './types';

const API_BASE = '/api/v1/users/me/notification-preferences';

// ============================================================================
// Story 40.1: Category Preferences API
// ============================================================================

/**
 * Fetch category-based notification preferences.
 */
export async function getCategoryPreferences(): Promise<CategoryPreferencesResponse> {
  return authenticatedFetchJson<CategoryPreferencesResponse>(`${API_BASE}/categories`);
}

/**
 * Update preferences for a specific category.
 */
export async function updateCategoryPreference(
  category: NotificationCategory,
  request: UpdateCategoryPreferenceRequest
): Promise<CategoryPreferencesResponse> {
  return authenticatedFetchJson<CategoryPreferencesResponse>(`${API_BASE}/categories/${category}`, {
    method: 'PATCH',
    body: JSON.stringify(request),
  });
}

// ============================================================================
// Story 40.2: Quiet Hours API
// ============================================================================

/**
 * Fetch quiet hours configuration.
 */
export async function getQuietHours(): Promise<QuietHoursResponse> {
  return authenticatedFetchJson<QuietHoursResponse>(`${API_BASE}/quiet-hours`);
}

/**
 * Update quiet hours configuration.
 */
export async function updateQuietHours(
  request: UpdateQuietHoursRequest
): Promise<QuietHoursResponse> {
  return authenticatedFetchJson<QuietHoursResponse>(`${API_BASE}/quiet-hours`, {
    method: 'PATCH',
    body: JSON.stringify(request),
  });
}

// ============================================================================
// Story 40.3: Digest Preferences API
// ============================================================================

/**
 * Fetch digest configuration.
 */
export async function getDigestPreferences(): Promise<DigestResponse> {
  return authenticatedFetchJson<DigestResponse>(`${API_BASE}/digest`);
}

/**
 * Update digest preferences.
 */
export async function updateDigestPreferences(
  request: UpdateDigestRequest
): Promise<DigestResponse> {
  return authenticatedFetchJson<DigestResponse>(`${API_BASE}/digest`, {
    method: 'PATCH',
    body: JSON.stringify(request),
  });
}

// ============================================================================
// Story 40.4: Grouping Preferences API
// ============================================================================

/**
 * Fetch notification grouping configuration.
 */
export async function getGroupingPreferences(): Promise<GroupingResponse> {
  return authenticatedFetchJson<GroupingResponse>(`${API_BASE}/grouping`);
}

/**
 * Update notification grouping preferences.
 */
export async function updateGroupingPreferences(
  request: UpdateGroupingRequest
): Promise<GroupingResponse> {
  return authenticatedFetchJson<GroupingResponse>(`${API_BASE}/grouping`, {
    method: 'PATCH',
    body: JSON.stringify(request),
  });
}

// ============================================================================
// Combined API
// ============================================================================

/**
 * Fetch all advanced notification preferences at once.
 */
export async function getAdvancedPreferences(): Promise<AdvancedPreferencesResponse> {
  return authenticatedFetchJson<AdvancedPreferencesResponse>(`${API_BASE}/advanced`);
}
