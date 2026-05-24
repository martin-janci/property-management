/**
 * OAuth User Grants Module
 *
 * API hooks and types for the OAuth user grants management feature
 * (Epic 10A, Story 10A-3). Exposes the two public-facing endpoints:
 *   GET    /api/v1/oauth/grants           — list authorized apps
 *   DELETE /api/v1/oauth/grants/{clientId} — revoke a specific app
 */

export { listUserGrants, revokeUserGrant } from './api';
export { oauthGrantKeys, useRevokeUserGrant, useUserGrants } from './hooks';
export type { UserGrant } from './types';
