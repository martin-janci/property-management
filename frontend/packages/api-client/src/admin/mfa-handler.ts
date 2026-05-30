/**
 * Admin MFA challenge handler — backward-compatible re-export.
 *
 * The handler implementation moved to `../lib/mfa-handler` so that the
 * shared `authenticatedFetchJson` factory (also in `lib/`) can reference
 * it without a lib → admin layering violation.
 *
 * All existing consumers that import from `@ppt/api-client` via the admin
 * subpath (`admin/index.ts` → here) continue to work unchanged.
 */

export {
  /** @deprecated Import from '../lib/mfa-handler' instead. This shim exists only for backward compatibility. */
  hasMfaChallengeHandler,
  /** @deprecated Import from '../lib/mfa-handler' instead. This shim exists only for backward compatibility. */
  type MfaChallengeHandler,
  /** @deprecated Import from '../lib/mfa-handler' instead. This shim exists only for backward compatibility. */
  requestMfaChallenge,
  /** @deprecated Import from '../lib/mfa-handler' instead. This shim exists only for backward compatibility. */
  setMfaChallengeHandler,
} from '../lib/mfa-handler';
