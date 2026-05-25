/**
 * Environment variables type declarations for Expo.
 *
 * Epic 85 - Story 85.1: Environment Variable Setup
 *
 * Two mechanisms provide env config at runtime:
 *
 * 1. EXPO_PUBLIC_* vars — loaded by Metro / Expo build system from .env files.
 *    Accessible as `process.env.EXPO_PUBLIC_XXX` in JS.
 *    See: https://docs.expo.dev/guides/environment-variables/
 *
 * 2. Constants.expoConfig.extra — injected by app.config.ts via dotenv.
 *    Accessible via `import Constants from 'expo-constants'`.
 *    Also includes iOS Info.plist keys (API_BASE_URL, ENVIRONMENT).
 *
 * Prefer reading through `src/config/api.ts` rather than directly
 * from process.env — api.ts checks both sources with a safe fallback.
 */

declare global {
  namespace NodeJS {
    interface ProcessEnv {
      // ---- Expo public env vars (Metro / Expo build system) ----------------

      /** API base URL for backend communication */
      EXPO_PUBLIC_API_BASE_URL?: string;
      /** WebSocket base URL for real-time communication */
      EXPO_PUBLIC_WS_BASE_URL?: string;
      /** Current environment: development, staging, or production */
      EXPO_PUBLIC_ENVIRONMENT?: 'development' | 'staging' | 'production';
      /** Enable debug mode features */
      EXPO_PUBLIC_DEBUG_MODE?: string;

      // ---- Build / CI variables (not exposed to client JS bundles) ---------

      /** Build number injected by CI/CD */
      BUILD_NUMBER?: string;
      /** Override which env file app.config.ts loads (development|staging|production) */
      APP_ENV?: 'development' | 'staging' | 'production';
    }
  }

  var process: {
    env: NodeJS.ProcessEnv;
  };
}

export {};
