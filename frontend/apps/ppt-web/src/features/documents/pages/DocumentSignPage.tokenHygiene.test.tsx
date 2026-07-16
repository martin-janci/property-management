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
 *   2. strips `?token=…` from the visible URL/history on mount, and
 *   3. installs a page-scoped `<meta name="referrer" content="no-referrer">`.
 *
 * Only the @ppt/api-client signer hooks (the network boundary) are mocked.
 */

import { render } from '@testing-library/react';
import { BrowserRouter } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { DocumentSignPage } from './DocumentSignPage';

// ─── network boundary: @ppt/api-client signer hooks ─────────────────────────
const { useSignContextMock } = vi.hoisted(() => ({ useSignContextMock: vi.fn() }));

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
    useSubmitSignature: () => ({
      mutate: vi.fn(),
      isPending: false,
      isSuccess: false,
      isError: false,
    }),
  };
});

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
    // Clean out any meta[name=referrer] a previous test may have left.
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
});
