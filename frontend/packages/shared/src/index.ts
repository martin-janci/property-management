/**
 * @ppt/shared - Shared utilities for Property Management apps.
 *
 * This package provides common utilities, types, and helpers used across
 * ppt-web, reality-web, and mobile applications.
 */

// ============================================
// API Client Re-exports
// ============================================
export * from '@ppt/api-client';
// ============================================
// Accessibility (existing)
// ============================================
export * from './accessibility';
// ============================================
// Authentication Utilities
// ============================================
export {
  type BasicUserInfo,
  // Auth Headers
  createAuthHeader,
  // Token Storage
  createTokenStorage,
  // JWT Utilities
  decodeJwt,
  extractTokenFromHeader,
  getAndClearReturnUrl,
  getAndClearSsoState,
  getSsoState,
  getTokenExpiration,
  getTokenRemainingTime,
  getUserFromToken,
  hasAnyRole,
  hasRolePermission,
  isTokenExpired,
  // Types
  type JwtPayload,
  // Roles
  ROLES,
  type Role,
  sanitizeReturnUrl,
  // Session
  setReturnUrl,
  setSsoState,
  type TokenPair,
} from './auth';

// ============================================
// Error Handling
// ============================================
export {
  DEFAULT_ERROR,
  // Constants
  ERROR_MESSAGES,
  // Types
  type ErrorResponse,
  formatFieldName,
  // Utilities
  formatValidationErrors,
  getErrorMessage,
  getFieldError,
  isErrorResponse,
  // Type Guards
  isNetworkError,
  isRetryableError,
  type ParsedApiError,
  // Parsing
  parseApiError,
  type ValidationDetail,
} from './errors';
// ============================================
// Formatting Utilities
// ============================================
export {
  type AddressComponents,
  type CurrencyFormatOptions,
  type DateFormatOptions,
  type DateInput,
  formatAddress,
  formatCompactCurrency,
  formatCompactNumber,
  // Currency
  formatCurrency,
  // Date/Time
  formatDate,
  formatDateTime,
  formatDetailedDate,
  // File Size
  formatFileSize,
  formatISODate,
  // Numbers
  formatNumber,
  formatPercent,
  // Phone & Address
  formatPhone,
  formatRelativeTime,
  formatTime,
  // Locale
  getUserLocale,
  type NumberFormatOptions,
} from './formatting';
// ============================================
// Layout Preview Bridge Protocol
// ============================================
export {
  // Functions
  connectPreviewChild,
  connectPreviewParent,
  // Guards
  isPreviewChildMessage,
  isPreviewParentMessage,
  LAYOUT_PREVIEW_KIND,
  LAYOUT_PREVIEW_PROTOCOL,
  // Types
  type PreviewChildMessage,
  type PreviewParentMessage,
  type ResolvedScreenLike,
  type ResolvedSectionLike,
  readPreviewParams,
} from './layout-preview';
// ============================================
// Validation Utilities
// ============================================
export {
  // Composite
  combineValidators,
  getPasswordStrength,
  // Image URL allowlist
  isSafeImageUrl,
  isValidCompanyId,
  // Email
  isValidEmail,
  isValidIban,
  // Phone
  isValidPhone,
  // Specialized
  isValidPostalCode,
  // URL
  isValidUrl,
  isValidVatId,
  type NumberRangeOptions,
  type PasswordRequirements,
  type StringLengthOptions,
  // Types
  type ValidationResult,
  type Validator,
  validateDateRange,
  validateEmail,
  // Dates
  validateFutureDate,
  // Numbers
  validateNumber,
  // Password
  validatePassword,
  validatePastDate,
  validatePhone,
  validateRequired,
  // Strings
  validateString,
  validateUrl,
} from './validation';

// ============================================
// Shared Types
// ============================================

/**
 * Tenant context for multi-tenant operations.
 */
export interface TenantContext {
  tenantId: string;
  tenantName: string;
  role: string;
}

/**
 * Basic user type for UI display.
 */
export interface User {
  id: string;
  email: string;
  displayName: string;
  avatarUrl?: string;
}

// ============================================
// Shared Constants
// ============================================

/** Current API version */
export const API_VERSION = 'v1';

/** Package version */
export const VERSION = '0.2.355';
