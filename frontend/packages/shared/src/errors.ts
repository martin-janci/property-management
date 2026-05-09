/**
 * API Error handling utilities for Property Management apps.
 * Provides framework-agnostic error parsing and user-friendly messages.
 */

// ============================================
// Types
// ============================================

/**
 * Backend error response format.
 * Matches the ErrorResponse structure from api-server.
 */
export interface ErrorResponse {
  requestId?: string;
  error: string;
  message: string;
  details?: ValidationDetail[];
}

/**
 * Validation error detail with field path.
 */
export interface ValidationDetail {
  field: string;
  message: string;
}

/**
 * Parsed API error result with user-friendly messages.
 */
export interface ParsedApiError {
  /** User-friendly error title */
  title: string;
  /** Detailed error message */
  message: string;
  /** Error code from backend */
  code: string;
  /** Request ID for error reporting */
  requestId?: string;
  /** Validation errors by field path */
  validationErrors?: Map<string, string>;
  /** HTTP status code */
  statusCode?: number;
  /** Whether this is a network error */
  isNetworkError: boolean;
  /** Whether this is a rate limit error */
  isRateLimitError: boolean;
  /** Whether this error is retryable */
  isRetryable: boolean;
  /** Retry-After value in seconds (for rate limiting) */
  retryAfterSeconds?: number;
}

// ============================================
// Error Messages
// ============================================

/**
 * Error code to user-friendly message mapping.
 */
export const ERROR_MESSAGES: Record<string, { title: string; message: string }> = {
  // Authentication errors
  AUTHENTICATION_ERROR: {
    title: 'Authentication Required',
    message: 'Please log in to continue.',
  },
  INVALID_CREDENTIALS: {
    title: 'Invalid Credentials',
    message: 'The email or password you entered is incorrect.',
  },
  SESSION_EXPIRED: {
    title: 'Session Expired',
    message: 'Your session has expired. Please log in again.',
  },
  UNAUTHORIZED: {
    title: 'Access Denied',
    message: 'You do not have permission to perform this action.',
  },

  // Validation errors
  VALIDATION_ERROR: {
    title: 'Validation Error',
    message: 'Please check the form for errors.',
  },
  INVALID_INPUT: {
    title: 'Invalid Input',
    message: 'The provided data is invalid.',
  },

  // Resource errors
  NOT_FOUND: {
    title: 'Not Found',
    message: 'The requested resource could not be found.',
  },
  RESOURCE_NOT_FOUND: {
    title: 'Resource Not Found',
    message: 'The item you are looking for does not exist or has been deleted.',
  },
  CONFLICT: {
    title: 'Conflict',
    message: 'This operation conflicts with existing data.',
  },
  DUPLICATE_ENTRY: {
    title: 'Duplicate Entry',
    message: 'An item with the same value already exists.',
  },

  // Rate limiting
  RATE_LIMITED: {
    title: 'Too Many Requests',
    message: 'You have made too many requests. Please wait before trying again.',
  },
  RATE_LIMIT_EXCEEDED: {
    title: 'Rate Limit Exceeded',
    message: 'Please slow down and try again in a moment.',
  },

  // Server errors
  INTERNAL_ERROR: {
    title: 'Server Error',
    message: 'An unexpected error occurred. Please try again later.',
  },
  SERVICE_UNAVAILABLE: {
    title: 'Service Unavailable',
    message: 'The service is temporarily unavailable. Please try again later.',
  },
  TIMEOUT: {
    title: 'Request Timeout',
    message: 'The request took too long. Please try again.',
  },

  // Network errors
  NETWORK_ERROR: {
    title: 'Network Error',
    message: 'Unable to connect to the server. Please check your internet connection.',
  },
  OFFLINE: {
    title: 'You are Offline',
    message: 'Please check your internet connection and try again.',
  },
};

/**
 * Default error message for unknown error codes.
 */
export const DEFAULT_ERROR = {
  title: 'Error',
  message: 'An unexpected error occurred.',
};

/** HTTP status codes that should trigger a retry */
const RETRYABLE_STATUS_CODES = [408, 429, 500, 502, 503, 504];

// ============================================
// Error Parsing
// ============================================

/**
 * Parse an API error response into a user-friendly format.
 *
 * @param error - The error object (can be fetch error, axios error, or plain object)
 * @returns Parsed error with user-friendly messages
 */
export function parseApiError(error: unknown): ParsedApiError {
  // Handle network errors
  if (isNetworkError(error)) {
    return {
      title: ERROR_MESSAGES.NETWORK_ERROR.title,
      message: ERROR_MESSAGES.NETWORK_ERROR.message,
      code: 'NETWORK_ERROR',
      isNetworkError: true,
      isRateLimitError: false,
      isRetryable: true,
    };
  }

  // Handle response-like errors (axios, fetch wrapper)
  if (hasResponse(error)) {
    const response = error.response;
    const statusCode = response?.status;
    const data = response?.data as ErrorResponse | undefined;

    // Check for rate limiting
    if (statusCode === 429) {
      const retryAfter = parseRetryAfter(response?.headers?.['retry-after']);
      return {
        title: ERROR_MESSAGES.RATE_LIMITED.title,
        message: retryAfter
          ? `Please wait ${retryAfter} seconds before trying again.`
          : ERROR_MESSAGES.RATE_LIMITED.message,
        code: 'RATE_LIMITED',
        requestId: data?.requestId,
        statusCode: 429,
        isNetworkError: false,
        isRateLimitError: true,
        isRetryable: true,
        retryAfterSeconds: retryAfter ?? undefined,
      };
    }

    if (data && typeof data === 'object' && 'error' in data) {
      return parseErrorResponse(data, statusCode);
    }

    // Handle HTTP status codes without structured error response
    return parseHttpStatusError(statusCode);
  }

  // Handle plain ErrorResponse objects
  if (isErrorResponse(error)) {
    return parseErrorResponse(error);
  }

  // Handle Error instances
  if (error instanceof Error) {
    return {
      title: DEFAULT_ERROR.title,
      message: error.message || DEFAULT_ERROR.message,
      code: 'UNKNOWN_ERROR',
      isNetworkError: false,
      isRateLimitError: false,
      isRetryable: false,
    };
  }

  // Fallback for unknown error types
  return {
    title: DEFAULT_ERROR.title,
    message: DEFAULT_ERROR.message,
    code: 'UNKNOWN_ERROR',
    isNetworkError: false,
    isRateLimitError: false,
    isRetryable: false,
  };
}

/**
 * Parse a structured ErrorResponse into ParsedApiError.
 */
function parseErrorResponse(data: ErrorResponse, statusCode?: number): ParsedApiError {
  const errorInfo = ERROR_MESSAGES[data.error] ?? DEFAULT_ERROR;
  const validationErrors = parseValidationDetails(data.details);

  return {
    title: errorInfo.title,
    message: data.message || errorInfo.message,
    code: data.error,
    requestId: data.requestId,
    validationErrors: validationErrors.size > 0 ? validationErrors : undefined,
    statusCode,
    isNetworkError: false,
    isRateLimitError: statusCode === 429,
    isRetryable: statusCode ? RETRYABLE_STATUS_CODES.includes(statusCode) : false,
  };
}

/**
 * Parse validation details into a Map of field path to error message.
 */
function parseValidationDetails(details?: ValidationDetail[]): Map<string, string> {
  const errors = new Map<string, string>();

  if (!details || !Array.isArray(details)) {
    return errors;
  }

  for (const detail of details) {
    if (detail.field && detail.message) {
      errors.set(detail.field, detail.message);
    }
  }

  return errors;
}

/**
 * Parse HTTP status code into a user-friendly error.
 */
function parseHttpStatusError(statusCode?: number): ParsedApiError {
  const isRetryable = statusCode ? RETRYABLE_STATUS_CODES.includes(statusCode) : false;

  switch (statusCode) {
    case 400:
      return {
        ...ERROR_MESSAGES.INVALID_INPUT,
        code: 'INVALID_INPUT',
        statusCode,
        isNetworkError: false,
        isRateLimitError: false,
        isRetryable: false,
      };
    case 401:
      return {
        ...ERROR_MESSAGES.AUTHENTICATION_ERROR,
        code: 'AUTHENTICATION_ERROR',
        statusCode,
        isNetworkError: false,
        isRateLimitError: false,
        isRetryable: false,
      };
    case 403:
      return {
        ...ERROR_MESSAGES.UNAUTHORIZED,
        code: 'UNAUTHORIZED',
        statusCode,
        isNetworkError: false,
        isRateLimitError: false,
        isRetryable: false,
      };
    case 404:
      return {
        ...ERROR_MESSAGES.NOT_FOUND,
        code: 'NOT_FOUND',
        statusCode,
        isNetworkError: false,
        isRateLimitError: false,
        isRetryable: false,
      };
    case 409:
      return {
        ...ERROR_MESSAGES.CONFLICT,
        code: 'CONFLICT',
        statusCode,
        isNetworkError: false,
        isRateLimitError: false,
        isRetryable: false,
      };
    case 429:
      return {
        ...ERROR_MESSAGES.RATE_LIMITED,
        code: 'RATE_LIMITED',
        statusCode,
        isNetworkError: false,
        isRateLimitError: true,
        isRetryable: true,
      };
    case 500:
    case 502:
    case 503:
      return {
        ...ERROR_MESSAGES.INTERNAL_ERROR,
        code: 'INTERNAL_ERROR',
        statusCode,
        isNetworkError: false,
        isRateLimitError: false,
        isRetryable: true,
      };
    case 504:
      return {
        ...ERROR_MESSAGES.TIMEOUT,
        code: 'TIMEOUT',
        statusCode,
        isNetworkError: false,
        isRateLimitError: false,
        isRetryable: true,
      };
    default:
      return {
        ...DEFAULT_ERROR,
        code: 'UNKNOWN_ERROR',
        statusCode,
        isNetworkError: false,
        isRateLimitError: false,
        isRetryable,
      };
  }
}

/**
 * Parse Retry-After header value.
 */
function parseRetryAfter(value: string | number | undefined | null): number | null {
  if (value === undefined || value === null) {
    return null;
  }

  if (typeof value === 'number') {
    return value;
  }

  // Try parsing as seconds
  const seconds = Number.parseInt(value, 10);
  if (!Number.isNaN(seconds)) {
    return seconds;
  }

  // Try parsing as HTTP date
  const date = new Date(value);
  if (!Number.isNaN(date.getTime())) {
    const now = Date.now();
    const retryAt = date.getTime();
    return Math.max(0, Math.ceil((retryAt - now) / 1000));
  }

  return null;
}

// ============================================
// Type Guards
// ============================================

interface ResponseError {
  response?: {
    status?: number;
    data?: unknown;
    headers?: Record<string, string | number | undefined>;
  };
  message?: string;
  code?: string;
}

function hasResponse(error: unknown): error is ResponseError {
  return typeof error === 'object' && error !== null && 'response' in error;
}

/**
 * Check if an error is a network error.
 */
export function isNetworkError(error: unknown): boolean {
  if (!error || typeof error !== 'object') {
    return false;
  }

  // Axios network error
  if ('code' in error && error.code === 'ERR_NETWORK') {
    return true;
  }

  // Fetch TypeError
  if (error instanceof TypeError && error.message.includes('fetch')) {
    return true;
  }

  // Check message for network-related keywords
  if ('message' in error && typeof error.message === 'string') {
    const message = error.message.toLowerCase();
    return (
      message.includes('network') ||
      message.includes('failed to fetch') ||
      message.includes('network request failed') ||
      message.includes('networkerror')
    );
  }

  return false;
}

/**
 * Type guard to check if a value is an ErrorResponse.
 */
export function isErrorResponse(error: unknown): error is ErrorResponse {
  return (
    typeof error === 'object' &&
    error !== null &&
    'error' in error &&
    'message' in error &&
    typeof (error as ErrorResponse).error === 'string' &&
    typeof (error as ErrorResponse).message === 'string'
  );
}

// ============================================
// Utilities
// ============================================

/**
 * Format validation errors for display.
 *
 * @param validationErrors - Map of field path to error message
 * @returns Formatted string with all validation errors
 */
export function formatValidationErrors(validationErrors: Map<string, string>): string {
  const messages: string[] = [];
  for (const [field, message] of validationErrors) {
    messages.push(`${formatFieldName(field)}: ${message}`);
  }
  return messages.join('\n');
}

/**
 * Convert field path to human-readable name.
 *
 * @example
 * formatFieldName('user.email') // 'Email'
 * formatFieldName('address.street_name') // 'Street Name'
 */
export function formatFieldName(fieldPath: string): string {
  // Get last segment of path
  const parts = fieldPath.split('.');
  const field = parts[parts.length - 1];

  // Convert snake_case to Title Case
  return field
    .split('_')
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
}

/**
 * Get validation error for a specific field.
 *
 * @param validationErrors - Map of field path to error message
 * @param fieldPath - The field path to look up
 * @returns Error message for the field, or undefined if not found
 */
export function getFieldError(
  validationErrors: Map<string, string> | undefined,
  fieldPath: string
): string | undefined {
  return validationErrors?.get(fieldPath);
}

/**
 * Extract a user-friendly message from any error.
 */
export function getErrorMessage(error: unknown): string {
  if (error && typeof error === 'object' && 'message' in error) {
    return String(error.message);
  }
  if (error instanceof Error) {
    return error.message;
  }
  return 'An unexpected error occurred';
}

/**
 * Check if an error should be retried.
 */
export function isRetryableError(error: unknown): boolean {
  const parsed = parseApiError(error);
  return parsed.isRetryable;
}
