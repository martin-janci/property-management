/**
 * Environment variables type declarations for Expo.
 *
 * Epic 85 - Story 85.1: Environment Variable Setup
 *
 * SINGLE SOURCE OF TRUTH: `Constants.expoConfig.extra` — injected by
 * `app.config.ts` from `.env.<APP_ENV>` via dotenv. Also surfaced to native
 * iOS code via `ios.infoPlist`. EXPO_PUBLIC_* duplicates were removed (#523)
 * because they cannot reach Info.plist and required hand-syncing.
 *
 * Prefer reading through `src/config/api.ts` rather than process.env directly.
 */

declare global {
  namespace NodeJS {
    interface ProcessEnv {
      // ---- Build / CI variables (read by app.config.ts; not in client JS) --

      /** Build number injected by CI/CD */
      BUILD_NUMBER?: string;
      /** Override which env file app.config.ts loads (development|staging|production) */
      APP_ENV?: 'development' | 'staging' | 'production';
      /** Optional CI override for the API base URL (mirrors .env value). */
      API_BASE_URL?: string;
      /** Optional CI override for the WebSocket base URL. */
      WS_BASE_URL?: string;
    }
  }

  var process: {
    env: NodeJS.ProcessEnv;
  };
}

export {};
