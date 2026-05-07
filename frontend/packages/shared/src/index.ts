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
// Formatting Utilities
// ============================================
export {
  // Locale
  getUserLocale,
  // Currency
  formatCurrency,
  formatCompactCurrency,
  type CurrencyFormatOptions,
  // Date/Time
  formatDate,
  formatDetailedDate,
  formatDateTime,
  formatTime,
  formatISODate,
  formatRelativeTime,
  type DateInput,
  type DateFormatOptions,
  // Numbers
  formatNumber,
  formatCompactNumber,
  formatPercent,
  type NumberFormatOptions,
  // Phone & Address
  formatPhone,
  formatAddress,
  type AddressComponents,
  // File Size
  formatFileSize,
} from './formatting';

// ============================================
// Validation Utilities
// ============================================
export {
  // Types
  type ValidationResult,
  type Validator,
  // Email
  isValidEmail,
  validateEmail,
  // Phone
  isValidPhone,
  validatePhone,
  // Password
  validatePassword,
  getPasswordStrength,
  type PasswordRequirements,
  // URL
  isValidUrl,
  validateUrl,
  // Numbers
  validateNumber,
  type NumberRangeOptions,
  // Strings
  validateString,
  validateRequired,
  type StringLengthOptions,
  // Dates
  validateFutureDate,
  validatePastDate,
  validateDateRange,
  // Composite
  combineValidators,
  // Specialized
  isValidPostalCode,
  isValidCompanyId,
  isValidVatId,
  isValidIban,
} from './validation';

// ============================================
// Error Handling
// ============================================
export {
  // Types
  type ErrorResponse,
  type ValidationDetail,
  type ParsedApiError,
  // Constants
  ERROR_MESSAGES,
  DEFAULT_ERROR,
  // Parsing
  parseApiError,
  // Type Guards
  isNetworkError,
  isErrorResponse,
  isRetryableError,
  // Utilities
  formatValidationErrors,
  formatFieldName,
  getFieldError,
  getErrorMessage,
} from './errors';

// ============================================
// Authentication Utilities
// ============================================
export {
  // Types
  type JwtPayload,
  type TokenPair,
  type BasicUserInfo,
  type Role,
  // JWT Utilities
  decodeJwt,
  isTokenExpired,
  getTokenExpiration,
  getTokenRemainingTime,
  getUserFromToken,
  // Token Storage
  createTokenStorage,
  // Auth Headers
  createAuthHeader,
  extractTokenFromHeader,
  // Roles
  ROLES,
  hasRolePermission,
  hasAnyRole,
  // Session
  setReturnUrl,
  getAndClearReturnUrl,
} from './auth';

// ============================================
// Accessibility (existing)
// ============================================
export * from './accessibility';

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
