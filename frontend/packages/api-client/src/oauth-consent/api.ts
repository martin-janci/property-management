/**
 * OAuth Consent (Authorization Endpoint) API Client
 *
 * API functions backing the OAuth 2.0 authorization-code consent screen
 * (Epic 10A). Wires to the public authorization endpoint on api-server:
 *
 *   GET  /api/v1/oauth/authorize — validate request, return consent page data
 *   POST /api/v1/oauth/authorize — submit the user's approve/deny decision
 *
 * Both endpoints require a valid Bearer token (the user's session); that is
 * attached automatically by `authenticatedFetchJson` via the registered token
 * provider, so the consent screen reuses the same auth plumbing as the rest of
 * the client rather than re-implementing header logic (issue #486).
 *
 * The GET response is JSON. The POST mirrors the backend's `Form<ConsentForm>`
 * extractor, so it is sent as `application/x-www-form-urlencoded`.
 */

import { authenticatedFetchJson } from '../lib/fetch';
import type {
  AuthorizeRequestParams,
  AuthorizeResponse,
  ConsentDecision,
  ConsentPageData,
} from './types';

const _win = typeof window !== 'undefined' ? (window as unknown as Record<string, unknown>) : {};
const API_BASE = `${_win.__API_BASE_URL__ ? String(_win.__API_BASE_URL__) : ''}/api/v1/oauth/authorize`;

/**
 * Build the query string for the authorize GET, omitting unset optionals.
 * Field names map to the backend's snake_case `AuthorizeQuery` extractor.
 */
function buildAuthorizeQuery(params: AuthorizeRequestParams): string {
  const qs = new URLSearchParams();
  qs.set('response_type', params.responseType);
  qs.set('client_id', params.clientId);
  qs.set('redirect_uri', params.redirectUri);
  if (params.scope) qs.set('scope', params.scope);
  if (params.state) qs.set('state', params.state);
  if (params.codeChallenge) qs.set('code_challenge', params.codeChallenge);
  if (params.codeChallengeMethod) qs.set('code_challenge_method', params.codeChallengeMethod);
  return qs.toString();
}

/**
 * Validate an incoming authorization request and fetch the data needed to
 * render the consent screen (client name, description, requested scopes).
 *
 * GET /api/v1/oauth/authorize
 *
 * Throws if the request is invalid (unknown client, bad redirect URI, missing
 * PKCE challenge, etc.) so the caller can show an error instead of a form.
 */
export async function fetchConsentPageData(
  params: AuthorizeRequestParams,
  signal?: AbortSignal
): Promise<ConsentPageData> {
  return authenticatedFetchJson<ConsentPageData>(`${API_BASE}?${buildAuthorizeQuery(params)}`, {
    method: 'GET',
    signal,
  });
}

/**
 * Submit the user's consent decision.
 *
 * POST /api/v1/oauth/authorize (application/x-www-form-urlencoded)
 *
 * On `approve` the server returns an authorization `code` (and echoes the
 * `state`) which the caller redirects back to the client. On `deny` the server
 * responds 403 and this function throws — the caller should redirect back with
 * an `error=access_denied` instead.
 */
export async function submitConsent(
  params: AuthorizeRequestParams,
  decision: ConsentDecision
): Promise<AuthorizeResponse> {
  const body = new URLSearchParams();
  body.set('client_id', params.clientId);
  body.set('redirect_uri', params.redirectUri);
  body.set('scope', params.scope ?? '');
  if (params.state) body.set('state', params.state);
  if (params.codeChallenge) body.set('code_challenge', params.codeChallenge);
  if (params.codeChallengeMethod) body.set('code_challenge_method', params.codeChallengeMethod);
  body.set('consent', decision);

  return authenticatedFetchJson<AuthorizeResponse>(API_BASE, {
    method: 'POST',
    // Override the default application/json — the backend uses a Form extractor.
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    body: body.toString(),
  });
}
