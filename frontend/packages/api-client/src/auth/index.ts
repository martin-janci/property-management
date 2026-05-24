/**
 * Authentication Module
 *
 * Provides authentication API and secure token management for API clients.
 */

// Auth API client and types
export { type AuthApi, AuthError, createAuthApi } from './api';
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
  LoginRequest,
  LoginResponse,
  LogoutRequest,
  RefreshTokenRequest,
  RefreshTokenResponse,
  RegisterRequest,
  RegisterResponse,
  RequestPasswordResetRequest,
  ResetPasswordRequest,
  TenantMembership,
  TenantRole,
} from './types';
