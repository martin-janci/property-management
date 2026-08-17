/**
 * API Error Handler for ppt-web.
 *
 * Parses backend ErrorResponse format and maps error codes to user-friendly messages.
 * Supports validation errors with field paths for inline form feedback.
 *
 * User-facing copy (error titles/messages) is resolved through the shared
 * react-i18next instance under the `errors.catalogue.*` keys, so the strings
 * follow the user's selected language. Developer-facing values — error `code`s,
 * `requestId`s and raw backend `message`s — are passed through untranslated.
 */

import i18n from '../i18n';

/**
 * Backend error response format.
 */
export interface ErrorResponse {
  requestId: string;
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
 * Parsed API error result.
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
  /** Retry-After value in seconds (for rate limiting) */
  retryAfterSeconds?: number;
}

/**
 * Resolved, user-facing copy for an error code.
 */
interface ErrorCopy {
  title: string;
  message: string;
}

/**
 * Error codes that have a dedicated translation entry under
 * `errors.catalogue.<CODE>`. Any code outside this set falls back to the
 * default copy (`errors.catalogue.default`).
 */
const KNOWN_ERROR_CODES = new Set<string>([
  // Authentication errors
  'AUTHENTICATION_ERROR',
  'INVALID_CREDENTIALS',
  'SESSION_EXPIRED',
  'UNAUTHORIZED',
  // Validation errors
  'VALIDATION_ERROR',
  'INVALID_INPUT',
  // Resource errors
  'NOT_FOUND',
  'RESOURCE_NOT_FOUND',
  'CONFLICT',
  'DUPLICATE_ENTRY',
  // Rate limiting
  'RATE_LIMITED',
  'RATE_LIMIT_EXCEEDED',
  // Server errors
  'INTERNAL_ERROR',
  'SERVICE_UNAVAILABLE',
  'TIMEOUT',
  // Network errors
  'NETWORK_ERROR',
  'OFFLINE',
]);

/**
 * Default, user-facing copy for unknown error codes, resolved from the active
 * i18n bundle.
 */
function defaultErrorCopy(): ErrorCopy {
  return {
    title: i18n.t('errors.catalogue.default.title'),
    message: i18n.t('errors.catalogue.default.message'),
  };
}

/**
 * Resolve the user-facing title/message for an error code from the active i18n
 * bundle. Unknown codes fall back to {@link defaultErrorCopy}.
 */
function errorCopy(code: string): ErrorCopy {
  if (!KNOWN_ERROR_CODES.has(code)) {
    return defaultErrorCopy();
  }
  return {
    title: i18n.t(`errors.catalogue.${code}.title`),
    message: i18n.t(`errors.catalogue.${code}.message`),
  };
}

/**
 * Parse an API error response into a user-friendly format.
 *
 * @param error - The error object (can be axios error, fetch error, or plain object)
 * @returns Parsed error with user-friendly messages
 */
export function parseApiError(error: unknown): ParsedApiError {
  // Handle network errors
  if (isNetworkError(error)) {
    const copy = errorCopy('NETWORK_ERROR');
    return {
      title: copy.title,
      message: copy.message,
      code: 'NETWORK_ERROR',
      isNetworkError: true,
      isRateLimitError: false,
    };
  }

  // Handle axios-style errors
  if (isAxiosError(error)) {
    const response = error.response;
    const statusCode = response?.status;
    const data = response?.data as ErrorResponse | undefined;

    // Check for rate limiting
    if (statusCode === 429) {
      const retryAfter = parseRetryAfter(response?.headers?.['retry-after']);
      const copy = errorCopy('RATE_LIMITED');
      return {
        title: copy.title,
        message: retryAfter
          ? i18n.t('errors.catalogue.rateLimitRetry', { seconds: retryAfter })
          : copy.message,
        code: 'RATE_LIMITED',
        requestId: data?.requestId,
        statusCode: 429,
        isNetworkError: false,
        isRateLimitError: true,
        retryAfterSeconds: retryAfter ?? undefined,
      };
    }

    if (data && typeof data === 'object') {
      return parseErrorResponse(data, statusCode);
    }

    // Handle HTTP status codes without structured error response
    return parseHttpStatusError(statusCode);
  }

  // Handle fetch-style errors
  if (isFetchError(error)) {
    const copy = errorCopy('NETWORK_ERROR');
    return {
      title: copy.title,
      message: copy.message,
      code: 'NETWORK_ERROR',
      isNetworkError: true,
      isRateLimitError: false,
    };
  }

  // Handle plain ErrorResponse objects
  if (isErrorResponse(error)) {
    return parseErrorResponse(error);
  }

  // Handle Error instances
  if (error instanceof Error) {
    const copy = defaultErrorCopy();
    return {
      title: copy.title,
      message: error.message || copy.message,
      code: 'UNKNOWN_ERROR',
      isNetworkError: false,
      isRateLimitError: false,
    };
  }

  // Fallback for unknown error types
  const copy = defaultErrorCopy();
  return {
    title: copy.title,
    message: copy.message,
    code: 'UNKNOWN_ERROR',
    isNetworkError: false,
    isRateLimitError: false,
  };
}

/**
 * Parse a structured ErrorResponse into ParsedApiError.
 */
function parseErrorResponse(data: ErrorResponse, statusCode?: number): ParsedApiError {
  const errorInfo = errorCopy(data.error);
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
  switch (statusCode) {
    case 400:
      return {
        ...errorCopy('INVALID_INPUT'),
        code: 'INVALID_INPUT',
        statusCode,
        isNetworkError: false,
        isRateLimitError: false,
      };
    case 401:
      return {
        ...errorCopy('AUTHENTICATION_ERROR'),
        code: 'AUTHENTICATION_ERROR',
        statusCode,
        isNetworkError: false,
        isRateLimitError: false,
      };
    case 403:
      return {
        ...errorCopy('UNAUTHORIZED'),
        code: 'UNAUTHORIZED',
        statusCode,
        isNetworkError: false,
        isRateLimitError: false,
      };
    case 404:
      return {
        ...errorCopy('NOT_FOUND'),
        code: 'NOT_FOUND',
        statusCode,
        isNetworkError: false,
        isRateLimitError: false,
      };
    case 409:
      return {
        ...errorCopy('CONFLICT'),
        code: 'CONFLICT',
        statusCode,
        isNetworkError: false,
        isRateLimitError: false,
      };
    case 429:
      return {
        ...errorCopy('RATE_LIMITED'),
        code: 'RATE_LIMITED',
        statusCode,
        isNetworkError: false,
        isRateLimitError: true,
      };
    case 500:
    case 502:
    case 503:
      return {
        ...errorCopy('INTERNAL_ERROR'),
        code: 'INTERNAL_ERROR',
        statusCode,
        isNetworkError: false,
        isRateLimitError: false,
      };
    case 504:
      return {
        ...errorCopy('TIMEOUT'),
        code: 'TIMEOUT',
        statusCode,
        isNetworkError: false,
        isRateLimitError: false,
      };
    default:
      return {
        ...defaultErrorCopy(),
        code: 'UNKNOWN_ERROR',
        statusCode,
        isNetworkError: false,
        isRateLimitError: false,
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

// Type guards

interface AxiosError {
  response?: {
    status?: number;
    data?: unknown;
    headers?: Record<string, string | number | undefined>;
  };
  message?: string;
  code?: string;
}

function isAxiosError(error: unknown): error is AxiosError {
  return typeof error === 'object' && error !== null && ('response' in error || 'code' in error);
}

function isNetworkError(error: unknown): boolean {
  if (!error || typeof error !== 'object') {
    return false;
  }

  // Axios network error
  if ('code' in error && error.code === 'ERR_NETWORK') {
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

function isFetchError(error: unknown): error is TypeError {
  return error instanceof TypeError && error.message.includes('fetch');
}

function isErrorResponse(error: unknown): error is ErrorResponse {
  return (
    typeof error === 'object' &&
    error !== null &&
    'error' in error &&
    'message' in error &&
    typeof (error as ErrorResponse).error === 'string' &&
    typeof (error as ErrorResponse).message === 'string'
  );
}

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
function formatFieldName(fieldPath: string): string {
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
