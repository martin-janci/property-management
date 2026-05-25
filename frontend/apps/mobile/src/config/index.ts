/**
 * App configuration exports.
 *
 * Prefer importing from this file rather than individual config modules.
 * All env-driven config flows through src/config/api.ts (reads from
 * Constants.expoConfig.extra set by app.config.ts, with EXPO_PUBLIC_* fallback).
 */
export { API_BASE_URL, APP_VERSION, BUILD_NUMBER, CONSTANTS } from './constants';
export {
  apiConfig,
  getApiBaseUrl,
  getEnvironment,
  getWsBaseUrl,
  isDebugMode,
  isDevelopment,
  isProduction,
  isStaging,
  type Environment,
} from './api';
