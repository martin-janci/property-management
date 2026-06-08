/**
 * OAuth Consent (Authorization Endpoint) Types
 *
 * Types for the OAuth 2.0 authorization-code consent flow (Epic 10A).
 * Mirror the `ConsentPageData` / `ScopeDisplay` / `AuthorizeResponse` Rust
 * structs in `db/models/oauth.rs` and `routes/oauth.rs` on api-server.
 *
 * These back the consent screen the user sees when a third-party application
 * redirects them to `/api/v1/oauth/authorize`. The admin Control Plane renders
 * this screen for its own (platform-principal) sessions.
 */

/**
 * A single requested scope, with a human-readable description for display
 * on the consent screen. Corresponds to `ScopeDisplay` in the backend.
 */
export interface ScopeDisplay {
  /** Machine scope name, e.g. `profile` or `read:buildings`. */
  name: string;
  /** Human-readable description shown next to the scope. */
  description: string;
}

/**
 * Everything the consent screen needs to render an authorization request.
 * Corresponds to `ConsentPageData` (camelCase) in the backend.
 */
export interface ConsentPageData {
  /** OAuth client identifier string. */
  clientId: string;
  /** Human-readable application name. */
  clientName: string;
  /** Optional developer-supplied description. */
  clientDescription: string | null;
  /** Scopes the application is requesting, with descriptions. */
  scopes: ScopeDisplay[];
  /** Validated redirect URI the code will be returned to. */
  redirectUri: string;
  /** Opaque CSRF state value echoed back to the client, if supplied. */
  state: string | null;
}

/**
 * The raw authorization-request parameters, as received on the inbound
 * redirect query string from the third-party client. These are passed
 * straight through to the backend (GET to validate, POST to approve/deny).
 */
export interface AuthorizeRequestParams {
  /** Must be `code` for the authorization-code flow. */
  responseType: string;
  clientId: string;
  redirectUri: string;
  /** Space-separated scope list, if any. */
  scope?: string;
  /** Opaque CSRF state value. */
  state?: string;
  /** PKCE code challenge. */
  codeChallenge?: string;
  /** PKCE code-challenge method (e.g. `S256`). */
  codeChallengeMethod?: string;
}

/**
 * Result of approving a consent request. Corresponds to `AuthorizeResponse`
 * (camelCase) in the backend — the authorization code to hand back to the
 * client (via its redirect URI) along with the echoed `state`.
 */
export interface AuthorizeResponse {
  /** The freshly minted authorization code. */
  code: string;
  /** Echoed CSRF state value, if one was supplied. */
  state: string | null;
}

/** Whether the user approved or denied the authorization request. */
export type ConsentDecision = 'approve' | 'deny';
