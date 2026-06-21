/**
 * App configuration exports.
 *
 * Prefer importing from this file rather than individual config modules.
 * All env-driven config flows through src/config/api.ts (reads from
 * Constants.expoConfig.extra, set by app.config.ts from .env.<APP_ENV>).
 * The EXPO_PUBLIC_* fallback was removed in issue #523 — do not reintroduce it.
 */

export {
  apiConfig,
  type Environment,
  getApiBaseUrl,
  getEnvironment,
  getWsBaseUrl,
  isDebugMode,
  isDevelopment,
  isProduction,
  isStaging,
} from './api';
export { API_BASE_URL, APP_VERSION, BUILD_NUMBER, CONSTANTS } from './constants';
