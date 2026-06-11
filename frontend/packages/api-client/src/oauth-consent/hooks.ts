/**
 * OAuth Consent TanStack Query Hooks
 *
 * React hooks backing the OAuth 2.0 authorization-code consent screen
 * (Epic 10A). One query (validate request + load consent data) and one
 * mutation (submit approve/deny).
 */

import { useMutation, useQuery } from '@tanstack/react-query';
import { fetchConsentPageData, submitConsent } from './api';
import type { AuthorizeRequestParams, ConsentDecision } from './types';

// ============================================
// Query Key Factory
// ============================================

export const oauthConsentKeys = {
  all: ['oauth-consent'] as const,
  /** Keyed by client + redirect + scope so distinct requests don't collide. */
  consentData: (params: AuthorizeRequestParams) =>
    [
      ...oauthConsentKeys.all,
      'data',
      params.clientId,
      params.redirectUri,
      params.scope ?? '',
    ] as const,
};

// ============================================
// Hooks
// ============================================

/**
 * Validate an incoming authorization request and load the consent screen
 * data. Disabled until the minimum required params (response_type, client_id,
 * redirect_uri) are present, so the hook never fires on a malformed redirect.
 *
 * The consent screen is a one-shot interaction — there is no staleness window
 * worth keeping, so the result is not cached across navigations.
 */
export function useConsentPageData(params: AuthorizeRequestParams | null) {
  return useQuery({
    queryKey: params
      ? oauthConsentKeys.consentData(params)
      : [...oauthConsentKeys.all, 'data', 'disabled'],
    queryFn: ({ signal }) => fetchConsentPageData(params as AuthorizeRequestParams, signal),
    enabled:
      params !== null &&
      params.responseType.length > 0 &&
      params.clientId.length > 0 &&
      params.redirectUri.length > 0,
    retry: false,
    staleTime: 0,
    gcTime: 0,
  });
}

/**
 * Submit the user's consent decision (approve/deny).
 *
 * On `approve` the mutation resolves with the authorization code + state for
 * the caller to redirect back to the client. On `deny` (or any error) it
 * rejects, and the caller decides how to redirect.
 */
export function useSubmitConsent(params: AuthorizeRequestParams | null) {
  return useMutation({
    mutationFn: (decision: ConsentDecision) =>
      submitConsent(params as AuthorizeRequestParams, decision),
  });
}
