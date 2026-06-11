/**
 * OAuth Consent Module
 *
 * API functions, hooks, and types for the OAuth 2.0 authorization-code consent
 * screen (Epic 10A). Backs the `/api/v1/oauth/authorize` endpoint pair:
 *   GET  /api/v1/oauth/authorize — validate request, return consent page data
 *   POST /api/v1/oauth/authorize — submit the user's approve/deny decision
 *
 * Distinct from `oauth-grants` (managing already-approved apps) and the admin
 * `oauth-clients` surface (registering clients).
 */

export { fetchConsentPageData, submitConsent } from './api';
export { oauthConsentKeys, useConsentPageData, useSubmitConsent } from './hooks';
export type {
  AuthorizeRequestParams,
  AuthorizeResponse,
  ConsentDecision,
  ConsentPageData,
  ScopeDisplay,
} from './types';
