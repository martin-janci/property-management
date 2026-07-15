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
import {
  createUploadUrl,
  deleteFolder,
  listDocuments,
  revokeDocumentShare,
  uploadDocumentDirect,
} from './api';

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

  // --- Direct-to-S3 upload (gap-84-1) ---

  it('createUploadUrl POSTs to /upload-url with the bearer token', async () => {
    vi.mocked(fetch).mockResolvedValueOnce(
      mockOkResponse({
        url: 'https://s3.example/bucket/key?sig=abc',
        file_key: 'org/2026/07/uuid_report.pdf',
        content_type: 'application/pdf',
        method: 'PUT',
        expires_at: '2026-07-15T00:05:00Z',
      })
    );

    const res = await createUploadUrl({
      file_name: 'report.pdf',
      mime_type: 'application/pdf',
      size_bytes: 1024,
    });

    const [url, init] = vi.mocked(fetch).mock.calls[0];
    expect(url).toBe('/api/v1/documents/upload-url');
    expect((init as RequestInit).method).toBe('POST');
    const headers = (init as RequestInit).headers as Record<string, string>;
    expect(headers.Authorization).toBe('Bearer doc-token');
    expect(res.file_key).toBe('org/2026/07/uuid_report.pdf');
  });

  it('uploadDocumentDirect chains upload-url -> S3 PUT -> register', async () => {
    // Mock XMLHttpRequest for the direct S3 PUT step.
    const setRequestHeader = vi.fn();
    const open = vi.fn();
    const listeners: Record<string, () => void> = {};
    const xhrMock = {
      upload: { addEventListener: vi.fn() },
      addEventListener: (evt: string, cb: () => void) => {
        listeners[evt] = cb;
      },
      open,
      setRequestHeader,
      send: vi.fn(() => {
        // Simulate a successful S3 response on next tick.
        queueMicrotask(() => {
          xhrMock.status = 200;
          listeners.load?.();
        });
      }),
      status: 0,
    };
    vi.stubGlobal('XMLHttpRequest', function XMLHttpRequestMock() {
      return xhrMock;
    });

    // 1) upload-url  2) register document
    vi.mocked(fetch)
      .mockResolvedValueOnce(
        mockOkResponse({
          url: 'https://s3.example/bucket/key?sig=abc',
          file_key: 'org/2026/07/uuid_report.pdf',
          content_type: 'application/pdf',
          method: 'PUT',
          expires_at: '2026-07-15T00:05:00Z',
        })
      )
      .mockResolvedValueOnce(mockOkResponse({ id: 'doc-42', message: 'created' }));

    const file = new File(['hello'], 'report.pdf', { type: 'application/pdf' });
    const result = await uploadDocumentDirect({
      file,
      title: 'Report',
      category: 'report',
      organizationId: 'org-1',
    });

    expect(result).toEqual({ id: 'doc-42', message: 'created' });
    // PUT went to the presigned URL with the signed Content-Type, no auth header.
    expect(open).toHaveBeenCalledWith('PUT', 'https://s3.example/bucket/key?sig=abc');
    expect(setRequestHeader).toHaveBeenCalledWith('Content-Type', 'application/pdf');
    // Registration call carried the file_key from the presign response.
    const registerBody = JSON.parse(
      (vi.mocked(fetch).mock.calls[1][1] as RequestInit).body as string
    );
    expect(registerBody.file_key).toBe('org/2026/07/uuid_report.pdf');
    expect(registerBody.mime_type).toBe('application/pdf');

    vi.unstubAllGlobals();
  });
});
