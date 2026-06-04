/**
 * Authentication API Client
 *
 * API functions for authentication operations (UC-14).
 * Handles login, logout, token refresh, and registration.
 */

import type { ApiConfig } from '../index';
import type {
  AuthErrorCode,
  AuthErrorResponse,
  ChangePasswordRequest,
  ListSessionsResponse,
  LoginRequest,
  LoginResponse,
  LogoutRequest,
  RefreshTokenRequest,
  RefreshTokenResponse,
  RegisterRequest,
  RegisterResponse,
  RequestPasswordResetRequest,
  ResetPasswordRequest,
  RevokeAllSessionsResponse,
  RevokeSessionResponse,
  SsoCallbackRequest,
  SsoCallbackResponse,
  UpdateProfileRequest,
} from './types';

/**
 * Custom error class for authentication errors.
 */
export class AuthError extends Error {
  constructor(
    message: string,
    public readonly code: AuthErrorCode,
    public readonly details?: Record<string, unknown>
  ) {
    super(message);
    this.name = 'AuthError';
  }
}

const buildHeaders = (config: ApiConfig, includeAuth = false): HeadersInit => ({
  'Content-Type': 'application/json',
  ...(includeAuth && config.accessToken && { Authorization: `Bearer ${config.accessToken}` }),
  ...(config.tenantId && { 'X-Tenant-ID': config.tenantId }),
});

const handleResponse = async <T>(response: Response): Promise<T> => {
  if (!response.ok) {
    const errorData: AuthErrorResponse = await response.json().catch(() => ({
      code: 'UNKNOWN_ERROR' as AuthErrorCode,
      message: 'Authentication request failed',
    }));

    // Map HTTP status codes to error codes if not provided
    let code = errorData.code;
    if (!code) {
      if (response.status === 401) {
        code = 'INVALID_CREDENTIALS';
      } else if (response.status === 403) {
        code = 'ACCOUNT_DISABLED';
      } else if (response.status === 423) {
        code = 'ACCOUNT_LOCKED';
      } else {
        code = 'UNKNOWN_ERROR';
      }
    }

    throw new AuthError(errorData.message || 'Authentication failed', code, errorData.details);
  }
  return response.json();
};

/**
 * Creates an authentication API client.
 *
 * @param config - API configuration including base URL
 * @returns Authentication API methods
 */
export const createAuthApi = (config: ApiConfig) => {
  const baseUrl = `${config.baseUrl}/api/v1/auth`;

  return {
    /**
     * Log in with email and password.
     *
     * @param credentials - Login credentials
     * @returns Login response with tokens and user info
     * @throws AuthError on authentication failure
     */
    login: async (credentials: LoginRequest): Promise<LoginResponse> => {
      const response = await fetch(`${baseUrl}/login`, {
        method: 'POST',
        headers: buildHeaders(config),
        body: JSON.stringify(credentials),
      });
      return handleResponse(response);
    },

    /**
     * Log out and invalidate the refresh token.
     *
     * @param request - Logout request with refresh token
     */
    logout: async (request: LogoutRequest): Promise<void> => {
      const response = await fetch(`${baseUrl}/logout`, {
        method: 'POST',
        headers: buildHeaders(config, true),
        body: JSON.stringify(request),
      });
      if (!response.ok) {
        // Ignore logout errors - tokens may already be invalid
        console.warn('Logout request failed, tokens may already be invalid');
      }
    },

    /**
     * Refresh the access token using a refresh token.
     *
     * @param request - Refresh token request
     * @returns New token pair
     * @throws AuthError on refresh failure
     */
    refreshToken: async (request: RefreshTokenRequest): Promise<RefreshTokenResponse> => {
      const response = await fetch(`${baseUrl}/refresh`, {
        method: 'POST',
        headers: buildHeaders(config),
        body: JSON.stringify(request),
      });
      return handleResponse(response);
    },

    /**
     * Register a new user account.
     *
     * @param data - Registration data
     * @returns Registration response
     * @throws AuthError on registration failure
     */
    register: async (data: RegisterRequest): Promise<RegisterResponse> => {
      const response = await fetch(`${baseUrl}/register`, {
        method: 'POST',
        headers: buildHeaders(config),
        body: JSON.stringify(data),
      });
      return handleResponse(response);
    },

    /**
     * Request a password reset email.
     *
     * @param request - Password reset request with email
     */
    requestPasswordReset: async (request: RequestPasswordResetRequest): Promise<void> => {
      const response = await fetch(`${baseUrl}/password-reset/request`, {
        method: 'POST',
        headers: buildHeaders(config),
        body: JSON.stringify(request),
      });
      if (!response.ok) {
        const error = await handleResponse<never>(response);
        throw error;
      }
    },

    /**
     * Reset password with a reset token.
     *
     * @param request - Password reset confirmation with token and new password
     */
    resetPassword: async (request: ResetPasswordRequest): Promise<void> => {
      const response = await fetch(`${baseUrl}/password-reset/confirm`, {
        method: 'POST',
        headers: buildHeaders(config),
        body: JSON.stringify(request),
      });
      if (!response.ok) {
        const error = await handleResponse<never>(response);
        throw error;
      }
    },

    /**
     * Change password for authenticated user.
     *
     * @param request - Current and new password
     */
    changePassword: async (request: ChangePasswordRequest): Promise<void> => {
      const response = await fetch(`${baseUrl}/password/change`, {
        method: 'POST',
        headers: buildHeaders(config, true),
        body: JSON.stringify(request),
      });
      if (!response.ok) {
        const error = await handleResponse<never>(response);
        throw error;
      }
    },

    /**
     * Verify email with verification token.
     *
     * @param token - Email verification token
     */
    verifyEmail: async (token: string): Promise<void> => {
      const response = await fetch(`${baseUrl}/verify-email?token=${encodeURIComponent(token)}`, {
        method: 'POST',
        headers: buildHeaders(config),
      });
      if (!response.ok) {
        const error = await handleResponse<never>(response);
        throw error;
      }
    },

    /**
     * Get current user profile (requires authentication).
     *
     * @returns Current user information
     */
    getCurrentUser: async (): Promise<LoginResponse['user']> => {
      const response = await fetch(`${baseUrl}/me`, {
        method: 'GET',
        headers: buildHeaders(config, true),
      });
      return handleResponse(response);
    },

    /**
     * Update the current authenticated user's profile.
     *
     * Endpoint: PATCH /api/v1/auth/me
     *
     * The backend only persists `displayName`, `phone`, and `avatarUrl`;
     * other fields are ignored. Returns the full updated user.
     */
    updateProfile: async (request: UpdateProfileRequest): Promise<LoginResponse['user']> => {
      const response = await fetch(`${baseUrl}/me`, {
        method: 'PATCH',
        headers: buildHeaders(config, true),
        body: JSON.stringify(request),
      });
      return handleResponse(response);
    },

    /**
     * List the current user's active sessions.
     *
     * Endpoint: GET /api/v1/auth/sessions
     */
    listSessions: async (): Promise<ListSessionsResponse> => {
      const response = await fetch(`${baseUrl}/sessions`, {
        method: 'GET',
        headers: buildHeaders(config, true),
      });
      return handleResponse(response);
    },

    /**
     * Revoke a specific session by ID.
     *
     * Endpoint: POST /api/v1/auth/sessions/revoke
     */
    revokeSession: async (sessionId: string): Promise<RevokeSessionResponse> => {
      const response = await fetch(`${baseUrl}/sessions/revoke`, {
        method: 'POST',
        headers: buildHeaders(config, true),
        body: JSON.stringify({ session_id: sessionId }),
      });
      return handleResponse(response);
    },

    /**
     * Revoke all sessions other than the current one.
     *
     * Endpoint: POST /api/v1/auth/sessions/revoke-all
     */
    revokeAllOtherSessions: async (): Promise<RevokeAllSessionsResponse> => {
      const response = await fetch(`${baseUrl}/sessions/revoke-all`, {
        method: 'POST',
        headers: buildHeaders(config, true),
      });
      return handleResponse(response);
    },

    /**
     * Exchange an SSO/OAuth authorization code for PPT JWT tokens.
     *
     * Endpoint: POST /api/v1/auth/sso/callback
     */
    exchangeSsoCode: async (request: SsoCallbackRequest): Promise<SsoCallbackResponse> => {
      const response = await fetch(`${baseUrl}/sso/callback`, {
        method: 'POST',
        headers: buildHeaders(config),
        body: JSON.stringify(request),
      });
      return handleResponse(response);
    },
  };
};

export type AuthApi = ReturnType<typeof createAuthApi>;
