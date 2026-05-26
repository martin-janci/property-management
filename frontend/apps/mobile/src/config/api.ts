/**
 * API configuration for the mobile app.
 *
 * Epic 49 - Story 49.1: Home Screen Widgets
 * Epic 85 - Story 85.1: Environment Variable Setup
 */

import Constants from 'expo-constants';

/**
 * Environment types supported by the application.
 */
export type Environment = 'development' | 'staging' | 'production';

/**
 * Get configuration value from Expo extra (single source of truth).
 *
 * Values flow: .env.<appEnv> → app.config.ts (dotenv) → Constants.expoConfig.extra.
 * EXPO_PUBLIC_* fallback was removed (issue #523) — app.config.ts is now the
 * only path so iOS Info.plist and JS read the same value.
 */
function getConfigValue(key: string, defaultValue: string): string {
  const extra = Constants.expoConfig?.extra;
  if (extra && key in extra) {
    const raw = extra[key];
    if (raw !== undefined && raw !== null) {
      return String(raw);
    }
  }
  return defaultValue;
}

/**
 * Get the API base URL based on the environment.
 * Reads from environment configuration, with platform-specific defaults for development.
 */
export function getApiBaseUrl(): string {
  const configuredUrl = getConfigValue('API_BASE_URL', '');
  if (configuredUrl) {
    return configuredUrl;
  }

  // Development fallback with platform-specific localhost handling
  if (__DEV__) {
    // Android emulator uses 10.0.2.2 to reach host localhost
    // iOS simulator can use localhost directly
    const Platform = require('react-native').Platform;
    return Platform.OS === 'android' ? 'http://10.0.2.2:8080' : 'http://localhost:8080';
  }

  // Production fallback — should never hit this in a real prod build because
  // app.config.ts guards against placeholder URLs.
  return 'http://localhost:8080';
}

/**
 * Get the WebSocket base URL based on the environment.
 * Derives from API URL if not explicitly configured.
 */
export function getWsBaseUrl(): string {
  const configuredUrl = getConfigValue('WS_BASE_URL', '');
  if (configuredUrl) {
    return configuredUrl;
  }

  // Derive from API URL by replacing http(s) with ws(s)
  const apiUrl = getApiBaseUrl();
  return apiUrl.replace(/^http/, 'ws');
}

/**
 * Get the current environment.
 */
export function getEnvironment(): Environment {
  const env = getConfigValue('ENVIRONMENT', __DEV__ ? 'development' : 'production');
  if (env === 'development' || env === 'staging' || env === 'production') {
    return env;
  }
  return __DEV__ ? 'development' : 'production';
}

/**
 * Check if debug mode is enabled.
 */
export function isDebugMode(): boolean {
  const debugMode = getConfigValue('DEBUG_MODE', String(__DEV__));
  return debugMode === 'true' || debugMode === '1';
}

/**
 * API configuration object with environment-aware settings.
 */
export const apiConfig = {
  /** Base URL for API requests */
  get baseUrl(): string {
    return getApiBaseUrl();
  },

  /** WebSocket URL for real-time communication */
  get wsUrl(): string {
    return getWsBaseUrl();
  },

  /** Current environment */
  get environment(): Environment {
    return getEnvironment();
  },

  /** Whether debug mode is enabled */
  get debugMode(): boolean {
    return isDebugMode();
  },

  /** Request timeout in milliseconds */
  timeout: 30000,

  /** Number of retry attempts for failed requests */
  retryAttempts: 3,
};

/** Check if running in production environment */
export const isProduction = () => apiConfig.environment === 'production';

/** Check if running in development environment */
export const isDevelopment = () => apiConfig.environment === 'development';

/** Check if running in staging environment */
export const isStaging = () => apiConfig.environment === 'staging';
