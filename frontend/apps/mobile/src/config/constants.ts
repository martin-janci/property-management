/**
 * App configuration constants.
 *
 * This file provides a centralized location for app-wide constants including
 * version info and API endpoints.
 *
 * Epic 85 - Story 85.1: Environment Variable Setup.
 * Env-driven values (API base URL, environment, …) are NOT read from
 * `process.env` here — that bypasses the single source of truth and cannot
 * reach the iOS `Info.plist`. The `EXPO_PUBLIC_*` fallback was removed in
 * issue #523; do not reintroduce it. Instead this module re-exports the
 * resolved value from `./api`, which reads `Constants.expoConfig.extra`
 * (injected by `app.config.ts` from `.env.<APP_ENV>`). See README.md
 * ("Environment variable setup") and `src/config/api.ts`.
 */

import { getApiBaseUrl } from './api';

/**
 * App version from package.json
 * Note: In production, this should be synchronized with package.json via build process.
 * For now, updated manually or via version bump script.
 */
export const APP_VERSION = '0.2.96';

/**
 * Build number (can be overridden by CI/CD via environment variable)
 */
export const BUILD_NUMBER = process.env.BUILD_NUMBER ?? '1';

/**
 * API base URL — resolved from the single source of truth
 * (`Constants.expoConfig.extra.API_BASE_URL`, populated by `app.config.ts`
 * from `.env.<APP_ENV>`), with the same dev / platform fallbacks as the rest
 * of the app. Previously this read `process.env.EXPO_PUBLIC_API_BASE_URL`
 * directly, which silently shipped the `api.ppt.example.com` placeholder in
 * release builds and diverged from the iOS `Info.plist` value (issue #523).
 */
export const API_BASE_URL = getApiBaseUrl();

/**
 * Default constants
 */
export const CONSTANTS = {
  APP_VERSION,
  BUILD_NUMBER,
  API_BASE_URL,
} as const;
