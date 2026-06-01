/**
 * Regression tests for the document API client (#751).
 *
 * Two findings are pinned here:
 *  - the document transport now routes through the shared authenticated
 *    `fetch` wrapper, so every request carries `Authorization: Bearer …`
 *    (previously it only set `Content-Type` and 401'd for protected routes);
 *  - `deleteFolder` / `revokeDocumentShare` hit endpoints that return
 *    `204 No Content` — they must resolve (to `undefined`) instead of
 *    rejecting while trying to parse an empty body.
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { clearTokenProvider, setTokenProvider } from '../auth/token-provider';
import { deleteFolder, listDocuments, revokeDocumentShare } from './api';

function mockOkResponse(body: unknown, status = 200): Response {
  return {
    ok: true,
    status,
    json: async () => body,
  } as Response;
}

function mock204Response(): Response {
  return {
    ok: true,
    status: 204,
    json: async () => {
      throw new Error('no body');
    },
  } as unknown as Response;
}

describe('documents api client', () => {
  beforeEach(() => {
    setTokenProvider(() => 'doc-token');
    vi.spyOn(globalThis, 'fetch');
  });

  afterEach(() => {
    clearTokenProvider();
    vi.restoreAllMocks();
  });

  it('attaches the bearer token from the registered provider', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(mockOkResponse({ documents: [], total: 0 }));
    await listDocuments();
    const headers = (vi.mocked(fetch).mock.calls[0][1] as RequestInit).headers as Record<
      string,
      string
    >;
    expect(headers.Authorization).toBe('Bearer doc-token');
  });

  it('resolves to undefined on a 204 delete-folder response', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(mock204Response());
    await expect(deleteFolder('folder-1', true)).resolves.toBeUndefined();
  });

  it('resolves to undefined on a 204 revoke-share response', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(mock204Response());
    await expect(revokeDocumentShare('doc-1', 'share-1')).resolves.toBeUndefined();
  });
});
