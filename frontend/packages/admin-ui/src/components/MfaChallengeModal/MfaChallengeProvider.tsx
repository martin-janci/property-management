/**
 * MFA challenge provider (N9).
 *
 * Glues the api-client's `setMfaChallengeHandler` seam to a real
 * `<MfaChallengeModal>` mounted at the app root. The seam is registered
 * lazily through a passed-in `register` prop so this package does not
 * depend on `@ppt/api-client` at build time (which would create a
 * circular workspace edge — api-client already exports the handler
 * setter, and admin-ui is consumed by api-client's own consumers).
 *
 * Usage at the app shell:
 *
 * ```tsx
 * import { MfaChallengeProvider } from '@ppt/admin-ui';
 * import { setMfaChallengeHandler, suspendAgency } from '@ppt/api-client';
 *
 * <MfaChallengeProvider
 *   registerHandler={setMfaChallengeHandler}
 *   verify={async (code) => {
 *     const r = await fetch('/api/v1/auth/mfa/verify', {
 *       method: 'POST',
 *       headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}` },
 *       body: JSON.stringify({ code }),
 *     });
 *     return r.ok;
 *   }}
 * >
 *   <App />
 * </MfaChallengeProvider>
 * ```
 *
 * The provider serializes challenges: only one modal at a time, even if
 * five mutations all 401 in quick succession. The api-client guards a
 * second-time retry, so the worst case is "user closes the modal" → all
 * five callers see the original 401.
 */

import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from 'react';
import { MfaChallengeModal, type MfaChallengeLabels } from './MfaChallengeModal';

type MfaChallengeHandler = () => Promise<boolean>;
type RegisterFn = (handler: MfaChallengeHandler | null) => void;
type VerifyFn = (code: string) => Promise<boolean>;

export interface MfaChallengeProviderProps {
  /**
   * The api-client's `setMfaChallengeHandler` (or any compatible
   * registration callback). Wired in `useEffect` so re-renders do not
   * thrash the global seam.
   */
  registerHandler: RegisterFn;
  /** POST {code} to /api/v1/auth/mfa/verify and return ok. */
  verify: VerifyFn;
  /**
   * Optional translated modal labels. Forwarded to `<MfaChallengeModal>`.
   * Hosts using i18n (ppt-web/react-i18next) should pass labels resolved
   * from `admin.mfa.*` keys.
   */
  labels?: MfaChallengeLabels;
  children: ReactNode;
}

interface MfaChallengeContextValue {
  /** Imperative trigger — used by features that want their own challenge flow. */
  prompt: (actionLabel?: string) => Promise<boolean>;
}

const MfaChallengeContext = createContext<MfaChallengeContextValue | null>(null);

export function useMfaChallenge(): MfaChallengeContextValue {
  const ctx = useContext(MfaChallengeContext);
  if (!ctx) {
    throw new Error('useMfaChallenge must be used inside <MfaChallengeProvider>');
  }
  return ctx;
}

interface PendingChallenge {
  resolve: (ok: boolean) => void;
  actionLabel?: string;
}

export function MfaChallengeProvider({
  registerHandler,
  verify,
  labels,
  children,
}: MfaChallengeProviderProps) {
  const [pending, setPending] = useState<PendingChallenge | null>(null);
  // Keep a ref so the handler we register with api-client does not
  // capture a stale `setPending` closure across re-renders.
  const setPendingRef = useRef(setPending);
  setPendingRef.current = setPending;

  // Single in-flight challenge guard: if a second 401 lands while the
  // modal is open, both callers wait on the same Promise.
  const inFlightRef = useRef<Promise<boolean> | null>(null);

  const prompt = useCallback<MfaChallengeContextValue['prompt']>(
    (actionLabel) => {
      if (inFlightRef.current) return inFlightRef.current;
      const promise = new Promise<boolean>((resolve) => {
        setPendingRef.current({
          resolve: (ok) => {
            inFlightRef.current = null;
            resolve(ok);
          },
          actionLabel,
        });
      });
      inFlightRef.current = promise;
      return promise;
    },
    [],
  );

  // Register the api-client seam exactly once per `registerHandler` ref.
  useEffect(() => {
    registerHandler(() => prompt());
    return () => registerHandler(null);
  }, [registerHandler, prompt]);

  const handleSuccess = useCallback(() => {
    pending?.resolve(true);
    setPending(null);
  }, [pending]);

  const handleCancel = useCallback(() => {
    pending?.resolve(false);
    setPending(null);
  }, [pending]);

  const value = useMemo<MfaChallengeContextValue>(() => ({ prompt }), [prompt]);

  return (
    <MfaChallengeContext.Provider value={value}>
      {children}
      <MfaChallengeModal
        open={pending !== null}
        actionLabel={pending?.actionLabel}
        onVerify={verify}
        onSuccess={handleSuccess}
        onCancel={handleCancel}
        labels={labels}
      />
    </MfaChallengeContext.Provider>
  );
}

MfaChallengeProvider.displayName = 'MfaChallengeProvider';
