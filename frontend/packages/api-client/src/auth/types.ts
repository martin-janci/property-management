/**
 * Authentication Types
 *
 * Type definitions for the Authentication API (UC-14).
 */

/** User role within a tenant (mirrors Shared.TenantRole in OpenAPI spec) */
export type TenantRole =
  | 'super_admin'
  | 'org_admin'
  | 'manager'
  | 'technical_manager'
  | 'owner'
  | 'owner_delegate'
  | 'tenant'
  | 'resident'
  | 'property_manager'
  | 'real_estate_agent'
  | 'guest';

/** User's membership in a tenant (mirrors Auth.TenantMembership in OpenAPI spec) */
export interface TenantMembership {
  tenantId: string;
  tenantName: string;
  role: TenantRole;
}

/** User information returned from authentication */
export interface AuthUser {
  id: string;
  email: string;
  firstName?: string;
  lastName?: string;
  /** The user role in the active tenant. Populated from tenants[0].role when not embedded directly. */
  role?: TenantRole;
  organizationId?: string;
  organizationName?: string;
}

/** Login request credentials */
export interface LoginRequest {
  email: string;
  password: string;
}

/** Login response */
export interface LoginResponse {
  accessToken: string;
  refreshToken: string;
  /** Seconds until the access token expires */
  expiresIn?: number;
  user: AuthUser;
  /** Tenant memberships available to this user (used to derive user.role) */
  tenants?: TenantMembership[];
}

/** Token refresh request */
export interface RefreshTokenRequest {
  refreshToken: string;
}

/** Token refresh response */
export interface RefreshTokenResponse {
  accessToken: string;
  refreshToken: string;
}

/** Logout request */
export interface LogoutRequest {
  refreshToken: string;
}

/** Registration request */
export interface RegisterRequest {
  email: string;
  password: string;
  firstName: string;
  lastName: string;
  organizationId?: string;
}

/** Registration response */
export interface RegisterResponse {
  id: string;
  email: string;
  firstName: string;
  lastName: string;
  message: string;
}

/** Password reset request */
export interface RequestPasswordResetRequest {
  email: string;
}

/** Password reset confirmation */
export interface ResetPasswordRequest {
  token: string;
  newPassword: string;
}

/** Change password request (authenticated) */
export interface ChangePasswordRequest {
  currentPassword: string;
  newPassword: string;
}

// ============================================================================
// Profile (PATCH /me)
// ============================================================================

/**
 * Request to update the current user's profile (`PATCH /api/v1/auth/me`).
 *
 * **Whitelist only**: the backend (`UpdateMeRequest` in `routes/auth.rs`) only
 * persists `displayName`, `phone`, and `avatarUrl`. Identity fields
 * (id, email, role, status) are intentionally not accepted here.
 */
export interface UpdateProfileRequest {
  displayName?: string;
  phone?: string;
  avatarUrl?: string;
}

// ============================================================================
// Session Management (GET/POST /auth/sessions)
// ============================================================================

/**
 * A single active session for the current user.
 *
 * Mirrors the backend `SessionInfo` DTO (`routes/auth.rs`), which serializes
 * with `camelCase` field names.
 */
export interface SessionInfo {
  /** Session ID */
  id: string;
  /** Device info, if available */
  deviceInfo?: string | null;
  /** IP address the session was created from */
  ipAddress?: string | null;
  /** User agent string */
  userAgent?: string | null;
  /** When the session was created (RFC3339) */
  createdAt: string;
  /** When the session was last used (RFC3339) */
  lastUsedAt: string;
  /** Whether this is the session making the request */
  isCurrent: boolean;
}

/** Response for `GET /api/v1/auth/sessions`. */
export interface ListSessionsResponse {
  sessions: SessionInfo[];
}

/**
 * Request body for `POST /api/v1/auth/sessions/revoke`.
 *
 * The backend `RevokeSessionRequest` has no `rename_all` attribute, so the
 * field is snake_case: `session_id`.
 */
export interface RevokeSessionRequest {
  session_id: string;
}

/** Response for `POST /api/v1/auth/sessions/revoke`. */
export interface RevokeSessionResponse {
  message: string;
}

/** Response for `POST /api/v1/auth/sessions/revoke-all`. */
export interface RevokeAllSessionsResponse {
  message: string;
  revokedCount: number;
}

/** Auth error codes */
export type AuthErrorCode =
  | 'INVALID_CREDENTIALS'
  | 'ACCOUNT_LOCKED'
  | 'ACCOUNT_DISABLED'
  | 'SESSION_EXPIRED'
  | 'TOKEN_INVALID'
  | 'TOKEN_EXPIRED'
  | 'EMAIL_NOT_VERIFIED'
  | 'EMAIL_ALREADY_EXISTS'
  | 'WEAK_PASSWORD'
  | 'NETWORK_ERROR'
  | 'UNKNOWN_ERROR';

/** Auth error response from API */
export interface AuthErrorResponse {
  code: AuthErrorCode;
  message: string;
  details?: Record<string, unknown>;
}

// ============================================================================
// SSO / OAuth Callback
// ============================================================================

/**
 * Request to exchange an OAuth authorization code for PPT JWT tokens.
 *
 * Sent by the frontend /auth/callback page after the SSO provider redirects
 * back with `?code=<code>&state=<state>`.
 */
export interface SsoCallbackRequest {
  /** OAuth authorization code from the SSO provider redirect. */
  code: string;
  /** CSRF / state value from the redirect (echoed back to the server). */
  state: string;
  /** Redirect URI that was registered with the SSO provider (must match). */
  redirectUri: string;
}

/**
 * Response from the SSO callback endpoint.
 *
 * On success the backend has exchanged the code with the SSO provider,
 * resolved / created the local PPT user, and returns the same JWT pair as
 * /api/v1/auth/login.
 */
export type SsoCallbackResponse = LoginResponse;
