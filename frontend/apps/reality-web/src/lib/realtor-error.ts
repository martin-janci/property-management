'use client';

/**
 * Localized, user-facing messages for realtor CRUD errors (UC-51).
 *
 * `realtor-api.ts` throws `RealtorApiError` with a developer-facing (English)
 * `message` and a stable, locale-independent `code`. The realtor create /
 * update / delete surfaces previously rendered that English `message` (and
 * hardcoded English fallbacks) directly, so cs/de/hu/pl/sk visitors saw
 * English error copy on a 6-language portal. This hook turns a thrown value
 * into a next-intl-translated string (`realtor.*`), mirroring the i18n
 * convention used by `ListingForm` (`listingForm.error*`) and the agent
 * profile page.
 */

import { useTranslations } from 'next-intl';
import { RealtorApiError } from './realtor-api';

/**
 * Operation-specific fallback keys under the `realtor` namespace. Used when the
 * thrown value is not a `RealtorApiError` (e.g. an unexpected runtime error) so
 * each call site can still surface a sensible, localized message.
 */
export type RealtorErrorFallbackKey =
  | 'errorLoadProfile'
  | 'errorSaveProfile'
  | 'errorLoadListing'
  | 'errorSaveListing'
  | 'errorCreateListing'
  | 'errorLoadAnalytics';

/**
 * Returns a translator that maps an unknown thrown value — in practice a
 * {@link RealtorApiError} — to a localized message:
 *
 * - `network` / `malformed` → generic `realtor.*` copy.
 * - `http` with a server-provided message → that message verbatim (the backend
 *   owns and localizes its own validation copy).
 * - `http` without a server message → `realtor.errorRequestFailed` with status.
 * - anything else → the call site's operation-specific fallback key.
 */
export function useRealtorErrorMessage() {
  const t = useTranslations('realtor');
  return (error: unknown, fallbackKey: RealtorErrorFallbackKey): string => {
    if (error instanceof RealtorApiError) {
      switch (error.code) {
        case 'network':
          return t('errorNetwork');
        case 'malformed':
          return t('errorMalformed');
        default:
          return error.serverMessage ?? t('errorRequestFailed', { status: error.status });
      }
    }
    return t(fallbackKey);
  };
}
