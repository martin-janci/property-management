/**
 * API Client Configuration
 *
 * Axios instance with interceptors for JWT token injection,
 * error transformation, and retry logic (Story 79.1).
 */

import axios, {
  type AxiosError,
  type AxiosInstance,
  type AxiosResponse,
  type InternalAxiosRequestConfig,
} from 'axios';

// ============================================================================
// Types
// ============================================================================

/**
 * Backend error response structure.
 * Matches the ErrorResponse format from api-server.
 */
export interface ApiErrorResponse {
  requestId?: string;
  error: string;
  message: string;
  details?: Record<string, unknown>;
}

/**
 * Transformed API error for client-side handling.
 */
export interface ApiError extends Error {
  status: number;
  requestId?: string;
  code: string;
  details?: Record<string, unknown>;
  isRetryable: boolean;
}

/**
 * Token getter function type.
 * Will be provided by AuthContext integration.
 */
export type TokenGetter = () => string | null | Promise<string | null>;

/**
 * API client configuration options.
 */
export interface ApiClientConfig {
  baseURL?: string;
  timeout?: number;
  getToken?: TokenGetter;
  onUnauthorized?: () => void;
}

// ============================================================================
// Constants
// ============================================================================

/** Default API base URL from environment or fallback.
 * The '/api/v1' relative fallback is handled by Vite's dev-server proxy
 * (see vite.config.ts). In production VITE_API_URL must be set. */
const DEFAULT_BASE_URL = import.meta.env.VITE_API_URL || '/api/v1';

/** Default request timeout in milliseconds (30 seconds) */
const DEFAULT_TIMEOUT = 30000;

/** Maximum number of retries for transient failures */
const MAX_RETRIES = 3;

/** Initial delay between retries in milliseconds */
const INITIAL_RETRY_DELAY = 1000;

/** HTTP status codes that should trigger a retry */
const RETRYABLE_STATUS_CODES = [408, 429, 500, 502, 503, 504];

// ============================================================================
// Helpers
// ============================================================================

/**
 * Check if an error is retryable based on status code or network error.
 */
function isRetryableError(error: AxiosError): boolean {
  // Network errors (no response) are retryable
  if (!error.response) {
    return true;
  }

  // Check if status code is in retryable list
  return RETRYABLE_STATUS_CODES.includes(error.response.status);
}

/**
 * Calculate delay for exponential backoff with jitter.
 */
function calculateRetryDelay(retryCount: number): number {
  const exponentialDelay = INITIAL_RETRY_DELAY * 2 ** retryCount;
  // Add random jitter (0-50% of delay) to prevent thundering herd
  const jitter = Math.random() * 0.5 * exponentialDelay;
  return exponentialDelay + jitter;
}

/**
 * Sleep for a given number of milliseconds.
 */
function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Transform axios error to ApiError.
 */
function transformError(error: AxiosError<ApiErrorResponse>): ApiError {
  const status = error.response?.status || 0;
  const data = error.response?.data;

  const apiError = new Error(
    data?.message || error.message || 'An unexpected error occurred'
  ) as ApiError;

  apiError.name = 'ApiError';
  apiError.status = status;
  apiError.requestId = data?.requestId;
  apiError.code = data?.error || 'UNKNOWN_ERROR';
  apiError.details = data?.details;
  apiError.isRetryable = isRetryableError(error);

  return apiError;
}

// ============================================================================
// API Client Factory
// ============================================================================

/** Store for the token getter function */
let tokenGetter: TokenGetter | undefined;

/** Store for unauthorized callback */
let onUnauthorizedCallback: (() => void) | undefined;

/**
 * Create and configure the axios instance.
 */
function createAxiosInstance(config: ApiClientConfig = {}): AxiosInstance {
  const instance = axios.create({
    baseURL: config.baseURL || DEFAULT_BASE_URL,
    timeout: config.timeout || DEFAULT_TIMEOUT,
    headers: {
      'Content-Type': 'application/json',
    },
  });

  // Store callbacks
  tokenGetter = config.getToken;
  onUnauthorizedCallback = config.onUnauthorized;

  // Request interceptor: Add JWT token to requests
  instance.interceptors.request.use(
    async (requestConfig: InternalAxiosRequestConfig) => {
      if (tokenGetter) {
        const token = await tokenGetter();
        if (token) {
          requestConfig.headers.Authorization = `Bearer ${token}`;
        }
      }
      return requestConfig;
    },
    (error: unknown) => Promise.reject(error)
  );

  // Response interceptor: Transform errors and handle retries
  instance.interceptors.response.use(
    (response: AxiosResponse) => response,
    async (error: AxiosError<ApiErrorResponse>) => {
      const config = error.config;

      // Handle unauthorized errors
      if (error.response?.status === 401 && onUnauthorizedCallback) {
        onUnauthorizedCallback();
      }

      // Check if we should retry
      if (config && isRetryableError(error)) {
        // Initialize retry count
        const retryCount =
          (config as InternalAxiosRequestConfig & { __retryCount?: number }).__retryCount || 0;

        if (retryCount < MAX_RETRIES) {
          // Update retry count
          (config as InternalAxiosRequestConfig & { __retryCount?: number }).__retryCount =
            retryCount + 1;

          // Calculate delay with exponential backoff
          const delay = calculateRetryDelay(retryCount);

          // Log retry attempt (in development)
          if (import.meta.env.DEV) {
            console.warn(
              `[API] Retrying request (${retryCount + 1}/${MAX_RETRIES}) after ${Math.round(delay)}ms:`,
              config.url
            );
          }

          // Wait before retrying
          await sleep(delay);

          // Retry the request
          return instance.request(config);
        }
      }

      // Transform and reject with ApiError
      return Promise.reject(transformError(error));
    }
  );

  return instance;
}

// ============================================================================
// Singleton Instance
// ============================================================================

/** The configured axios instance */
let apiInstance: AxiosInstance | null = null;

/**
 * Get the API client instance.
 * Creates the instance on first call with default configuration.
 */
export function getApiClient(): AxiosInstance {
  if (!apiInstance) {
    apiInstance = createAxiosInstance();
  }
  return apiInstance;
}

/**
 * Configure the API client with custom options.
 * Should be called once during app initialization, typically in AuthProvider.
 *
 * @param config - Configuration options for the API client
 * @returns The configured axios instance
 *
 * @example
 * ```typescript
 * // In AuthProvider
 * configureApiClient({
 *   getToken: () => localStorage.getItem('accessToken'),
 *   onUnauthorized: () => {
 *     // Redirect to login or refresh token
 *   },
 * });
 * ```
 */
export function configureApiClient(config: ApiClientConfig): AxiosInstance {
  apiInstance = createAxiosInstance(config);
  return apiInstance;
}

/**
 * Reset the API client instance.
 * Useful for testing or when user logs out.
 */
export function resetApiClient(): void {
  apiInstance = null;
  tokenGetter = undefined;
  onUnauthorizedCallback = undefined;
}

// ============================================================================
// Convenience Exports
// ============================================================================

/**
 * Default export: the API client instance.
 * Use getApiClient() for type-safe access.
 */
export default getApiClient();

/**
 * Type guard to check if an error is an ApiError.
 */
export function isApiError(error: unknown): error is ApiError {
  return error instanceof Error && 'status' in error && 'code' in error && 'isRetryable' in error;
}

/**
 * Extract a user-friendly message from an error.
 */
export function getErrorMessage(error: unknown): string {
  if (isApiError(error)) {
    return error.message;
  }
  if (error instanceof Error) {
    return error.message;
  }
  return 'An unexpected error occurred';
}
