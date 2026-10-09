/**
 * Authentication Module
 *
 * Provides authentication API and secure token management for API clients.
 */

// Auth API client and types
export { type AuthApi, AuthError, createAuthApi } from './api';
// Single source of truth for the authenticated header set, shared by the
// generated-client interceptor and the raw-fetch primitive (#3006).
export { applyAuthHeaders } from './apply-headers';
export { type AuthInterceptorClient, registerAuthInterceptors } from './interceptors';
// Active-org provider + centralized auth interceptor (#1522)
export { clearOrgProvider, getOrg, type OrgProvider, setOrgProvider } from './org-provider';
// Token provider for secure token management
export {
  clearTokenProvider,
  getToken,
  hasTokenProvider,
  setTokenProvider,
  type TokenProvider,
} from './token-provider';
export type {
  AuthErrorCode,
  AuthErrorResponse,
  AuthUser,
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
  RevokeSessionRequest,
  RevokeSessionResponse,
  SessionInfo,
  SsoCallbackRequest,
  SsoCallbackResponse,
  TenantMembership,
  TenantRole,
  UpdateProfileRequest,
} from './types';
