'use client';

/**
 * Localized, user-facing messages for account/auth API errors (UC-47).
 *
 * `auth-api.ts` throws `AuthApiError` with a developer-facing (English)
 * `message` and a stable `code` (`NETWORK_ERROR`, `MALFORMED_RESPONSE`,
 * `NOT_IMPLEMENTED`, or a backend-supplied code). The account pages
 * (`/account/profile`, `/account/password`) previously rendered that English
 * `message` directly, so cs/de/hu/pl/sk visitors saw English error copy —
 * most visibly the password page's "Changing your password … is not available
 * yet" string — on a 6-language portal. This hook turns a thrown value into a
 * next-intl-translated string (`authError.*`), mirroring `useRealtorErrorMessage`
 * (`realtor.*`) used by the sibling realtor pages.
 */

import { useTranslations } from 'next-intl';
import { AuthApiError } from './auth-api';

/**
 * Returns a translator that maps an unknown thrown value — in practice an
 * {@link AuthApiError} — to a localized message:
 *
 * - `NETWORK_ERROR` → generic network copy.
 * - `MALFORMED_RESPONSE` → generic malformed-response copy.
 * - `NOT_IMPLEMENTED` → "feature not available yet" copy.
 * - any other `AuthApiError` → the backend's own `serverMessage` (which it
 *   owns and localizes) when present, else `authError.errorRequestFailed`
 *   with the status.
 * - anything that is not an `AuthApiError` → the caller-supplied, already
 *   translated `fallback` string (each call site passes its own operation
 *   copy, e.g. `t('saveFailed')`).
 */
export function useAuthErrorMessage() {
  const t = useTranslations('authError');
  return (error: unknown, fallback: string): string => {
    if (error instanceof AuthApiError) {
      switch (error.code) {
        case 'NETWORK_ERROR':
          return t('errorNetwork');
        case 'MALFORMED_RESPONSE':
          return t('errorMalformed');
        case 'NOT_IMPLEMENTED':
          return t('errorNotImplemented');
        default:
          return error.serverMessage ?? t('errorRequestFailed', { status: error.status });
      }
    }
    return fallback;
  };
}
