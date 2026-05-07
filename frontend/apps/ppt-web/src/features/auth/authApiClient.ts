/**
 * Helper to construct an auth API client using the same conventions as
 * AuthContext (env base URL + bearer token from localStorage).
 *
 * @see Story 79.x - UC-14 Auth flow
 */

import { type AuthApi, createAuthApi } from '@ppt/api-client';

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? '';
const ACCESS_TOKEN_KEY = 'ppt_access_token';

function readAccessToken(): string | undefined {
  try {
    return localStorage.getItem(ACCESS_TOKEN_KEY) ?? undefined;
  } catch {
    return undefined;
  }
}

export function getAuthApi(): AuthApi {
  return createAuthApi({
    baseUrl: API_BASE_URL,
    accessToken: readAccessToken(),
  });
}
