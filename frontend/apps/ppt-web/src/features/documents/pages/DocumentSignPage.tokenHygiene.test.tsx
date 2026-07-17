/// <reference types="vitest/globals" />
/**
 * Regression tests for signing-token hygiene on the public `/sign` page
 * (follow-up #2363, security finding from PR #2347 post-merge review).
 *
 * The single-use HMAC signing token arrives in `?token=…` and is the ONLY
 * credential for this auth-less route. Leaving it in the URL leaks it: on a
 * shared/kiosk browser it stays recoverable from history after the signer
 * leaves, and it rides the `Referer` header on outbound navigation. These
 * tests lock in that `DocumentSignPage`:
 *
 *   1. still hands the token to `useSignContext` (capture happens before the
 *      URL is rewritten, so the render context still loads), and
 *   2. strips only the `token` key from the visible URL/history on mount,
 *      keeping any other query params + fragment, and
 *   3. installs a page-scoped `<meta name="referrer" content="no-referrer">`, and
 *   4. persists the token to sessionStorage so a full page reload (which
 *      re-requests `/sign` with no `?token`) still finds it — the pre-#2402
 *      regression where a valid link showed "invalid link" after F5 — and
 *      clears that copy once the signature is submitted (#2402).
 *
 * Only the @ppt/api-client signer hooks (the network boundary) are mocked.
 */

import { render } from '@testing-library/react';
import { BrowserRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { DocumentSignPage } from './DocumentSignPage';

// ─── network boundary: @ppt/api-client signer hooks ─────────────────────────
const { useSignContextMock, useSubmitSignatureMock } = vi.hoisted(() => ({
  useSignContextMock: vi.fn(),
  useSubmitSignatureMock: vi.fn(),
}));

vi.mock('@ppt/api-client', () => {
  class SignErrorStub extends Error {
    code: string;
    constructor(code: string) {
      super(code);
      this.code = code;
    }
  }
  return {
    SignError: SignErrorStub,
    useSignContext: (token?: string) => useSignContextMock(token),
    useSubmitSignature: (token: string) => useSubmitSignatureMock(token),
  };
});

const SIGN_TOKEN_STORAGE_KEY = 'signToken';

function renderAt(url: string) {
  window.history.replaceState({}, '', url);
  return render(
    <BrowserRouter>
      <DocumentSignPage />
    </BrowserRouter>
  );
}

describe('DocumentSignPage token hygiene', () => {
  beforeEach(() => {
    useSignContextMock.mockReset();
    // Default: still loading — we only care about token plumbing here.
    useSignContextMock.mockReturnValue({
      data: undefined,
      isLoading: true,
      isError: false,
      error: undefined,
    });
    useSubmitSignatureMock.mockReset();
    useSubmitSignatureMock.mockReturnValue({
      mutate: vi.fn(),
      isPending: false,
      isSuccess: false,
      isError: false,
    });
    // Each test starts from a clean tab: no persisted token, no stray meta tag.
    window.sessionStorage.clear();
    for (const m of Array.from(document.head.querySelectorAll('meta[name="referrer"]'))) {
      m.remove();
    }
  });

  afterEach(() => {
    window.history.replaceState({}, '', '/');
  });

  it('passes the token from the URL to useSignContext before stripping it', () => {
    renderAt('/sign?token=tok-abc-123');
    expect(useSignContextMock).toHaveBeenCalledWith('tok-abc-123');
  });

  it('strips ?token from the visible URL/history on mount', () => {
    renderAt('/sign?token=tok-abc-123');
    expect(window.location.search).toBe('');
    expect(window.location.pathname).toBe('/sign');
  });

  it('strips only the token key, preserving other query params and the fragment', () => {
    renderAt('/sign?token=tok-abc-123&lang=de&ref=email#terms');
    // token removed…
    expect(window.location.search).not.toContain('token');
    // …but lang/ref and the hash survive.
    const params = new URLSearchParams(window.location.search);
    expect(params.get('lang')).toBe('de');
    expect(params.get('ref')).toBe('email');
    expect(window.location.hash).toBe('#terms');
    // and the token still reached the render-context hook.
    expect(useSignContextMock).toHaveBeenCalledWith('tok-abc-123');
  });

  it('installs a no-referrer meta tag while mounted and removes it on unmount', () => {
    const { unmount } = renderAt('/sign?token=tok-abc-123');
    const meta = document.head.querySelector('meta[name="referrer"]');
    expect(meta).not.toBeNull();
    expect(meta?.getAttribute('content')).toBe('no-referrer');
    unmount();
    expect(document.head.querySelector('meta[name="referrer"]')).toBeNull();
  });

  it('leaves the URL untouched when no token is present', () => {
    renderAt('/sign');
    expect(useSignContextMock).toHaveBeenCalledWith(undefined);
    expect(window.location.pathname).toBe('/sign');
    expect(window.location.search).toBe('');
  });

  // ─── reload regression (#2402) ────────────────────────────────────────────

  it('persists the captured token to sessionStorage for reload survival', () => {
    renderAt('/sign?token=tok-abc-123');
    expect(window.sessionStorage.getItem(SIGN_TOKEN_STORAGE_KEY)).toBe('tok-abc-123');
  });

  it('re-seeds the token from sessionStorage on a reload with no ?token in the URL', () => {
    // Simulate a prior visit having stashed the token, then a full reload where
    // the URL no longer carries it (the address bar was already stripped).
    window.sessionStorage.setItem(SIGN_TOKEN_STORAGE_KEY, 'tok-from-reload');
    renderAt('/sign');
    // Pre-fix this called useSignContext(undefined) → "invalid signing link".
    expect(useSignContextMock).toHaveBeenCalledWith('tok-from-reload');
  });

  it('clears the persisted token after a successful submit', () => {
    window.sessionStorage.setItem(SIGN_TOKEN_STORAGE_KEY, 'tok-abc-123');
    useSignContextMock.mockReturnValue({
      data: {
        subject: 'Lease',
        signer_name: 'Jane Signer',
        signer_email: 'jane@example.com',
        signer_status: 'pending',
      },
      isLoading: false,
      isError: false,
      error: undefined,
    });
    useSubmitSignatureMock.mockReturnValue({
      mutate: vi.fn(),
      isPending: false,
      isSuccess: true,
      isError: false,
      data: { status: 'completed', message: 'Signed.' },
    });
    renderAt('/sign?token=tok-abc-123');
    expect(window.sessionStorage.getItem(SIGN_TOKEN_STORAGE_KEY)).toBeNull();
  });
});
