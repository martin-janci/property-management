/// <reference types="vitest/globals" />
/**
 * `LoginPage.getErrorMessage` regression — never leak a raw error message.
 *
 * Code-review finding (code-review-ppt-web-ui-loginpage-raw-error-message):
 * the login page's `getErrorMessage` helper routed `AuthError` codes through
 * i18n (`auth.invalidCredentials`, `auth.accountLocked`, …) but, for any other
 * `Error`, fell back to returning `error.message` verbatim. A generic backend
 * or transport failure (e.g. `Error("relation \"users\" does not exist")`, a
 * 500 body, a stack-trace-bearing message) was therefore surfaced directly in
 * the login error banner — leaking implementation detail and bypassing the
 * app's i18n convention.
 *
 * Locked in here:
 *   1. A generic `Error` degrades to `auth.unexpectedError` and its raw
 *      `message` never appears in the returned string. Fails on `dev`
 *      (returns the raw message), passes once the fallback is i18n'd.
 *   2. The network-error heuristic still maps `fetch`/`network` messages to
 *      `auth.networkError`.
 *   3. `AuthError` codes still map to their dedicated i18n keys, and an
 *      unknown/unlisted code falls back to `auth.unexpectedError`.
 *   4. A non-Error throwable (e.g. a thrown string) falls back to
 *      `auth.unexpectedError`.
 */

import { AuthError } from '@ppt/api-client';
import { getErrorMessage } from './LoginPage';

/** Identity `t` — returns the key so assertions read on the i18n key itself. */
const t = (key: string): string => key;

describe('LoginPage.getErrorMessage', () => {
  it('never surfaces a raw generic Error message to the user', () => {
    const raw = 'relation "users" does not exist';
    const result = getErrorMessage(new Error(raw), t);

    expect(result).toBe('auth.unexpectedError');
    expect(result).not.toContain(raw);
  });

  it('maps fetch/network failures to the network-error key', () => {
    expect(getErrorMessage(new Error('Failed to fetch'), t)).toBe('auth.networkError');
    expect(getErrorMessage(new Error('network request failed'), t)).toBe('auth.networkError');
  });

  it('maps known AuthError codes to their dedicated i18n keys', () => {
    expect(getErrorMessage(new AuthError('x', 'INVALID_CREDENTIALS'), t)).toBe(
      'auth.invalidCredentials'
    );
    expect(getErrorMessage(new AuthError('x', 'ACCOUNT_LOCKED'), t)).toBe('auth.accountLocked');
    expect(getErrorMessage(new AuthError('x', 'ACCOUNT_DISABLED'), t)).toBe('auth.accountDisabled');
    expect(getErrorMessage(new AuthError('x', 'SESSION_EXPIRED'), t)).toBe('auth.sessionExpired');
    expect(getErrorMessage(new AuthError('x', 'NETWORK_ERROR'), t)).toBe('auth.networkError');
  });

  it('falls back to the unexpected-error key for an unknown AuthError code', () => {
    expect(getErrorMessage(new AuthError('x', 'UNKNOWN_ERROR'), t)).toBe('auth.unexpectedError');
  });

  it('falls back to the unexpected-error key for a non-Error throwable', () => {
    expect(getErrorMessage('boom', t)).toBe('auth.unexpectedError');
  });
});
