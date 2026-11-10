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

  it('uploadDocumentDirect forwards folder_id and building scope via access_scope (#2366)', async () => {
    // The shipped registration contract (`POST /api/v1/documents`) has no
    // `building_id` field — building association is expressed as
    // `access_scope='building'` + `access_target_ids=[buildingId]` (the JSONB
    // array the RLS gate and building list/search filter read). gap-84-1
    // originally dropped the association when it switched to the direct path;
    // #2366 restores it the shipped way. `folder_id` must also survive.
    const listeners: Record<string, () => void> = {};
    const xhrMock = {
      upload: { addEventListener: vi.fn() },
      addEventListener: (evt: string, cb: () => void) => {
        listeners[evt] = cb;
      },
      open: vi.fn(),
      setRequestHeader: vi.fn(),
      send: vi.fn(() => {
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
      .mockResolvedValueOnce(mockOkResponse({ id: 'doc-99', message: 'created' }));

    const file = new File(['hello'], 'report.pdf', { type: 'application/pdf' });
    await uploadDocumentDirect({
      file,
      title: 'Report',
      category: 'report',
      organizationId: 'org-1',
      buildingId: 'building-7',
      folderId: 'folder-3',
    });

    const registerBody = JSON.parse(
      (vi.mocked(fetch).mock.calls[1][1] as RequestInit).body as string
    );
    expect(registerBody.folder_id).toBe('folder-3');
    // Building association is carried by the shipped access-scope mechanism,
    // NOT a raw `building_id` field (which the server would ignore).
    expect(registerBody).not.toHaveProperty('building_id');
    expect(registerBody.access_scope).toBe('building');
    expect(registerBody.access_target_ids).toEqual(['building-7']);

    vi.unstubAllGlobals();
  });

  it('uploadDocumentDirect omits access_scope when no building is supplied (#2366)', async () => {
    // Without a building context the client must NOT set an access scope — the
    // server then applies its default (organization) scope. Setting
    // `access_scope='building'` unconditionally would 400 (missing
    // access_target_ids) and would wrongly narrow org-wide uploads.
    const listeners: Record<string, () => void> = {};
    const xhrMock = {
      upload: { addEventListener: vi.fn() },
      addEventListener: (evt: string, cb: () => void) => {
        listeners[evt] = cb;
      },
      open: vi.fn(),
      setRequestHeader: vi.fn(),
      send: vi.fn(() => {
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
      .mockResolvedValueOnce(mockOkResponse({ id: 'doc-1', message: 'created' }));

    const file = new File(['hello'], 'report.pdf', { type: 'application/pdf' });
    await uploadDocumentDirect({
      file,
      title: 'Report',
      category: 'report',
      organizationId: 'org-1',
    });

    const registerBody = JSON.parse(
      (vi.mocked(fetch).mock.calls[1][1] as RequestInit).body as string
    );
    expect(registerBody).not.toHaveProperty('access_scope');
    expect(registerBody).not.toHaveProperty('access_target_ids');

    vi.unstubAllGlobals();
  });

  // Shared XHR mock that resolves the S3 PUT (step 2) successfully.
  function stubSuccessfulS3Put(): { open: ReturnType<typeof vi.fn> } {
    const listeners: Record<string, () => void> = {};
    const xhrMock = {
      upload: { addEventListener: vi.fn() },
      addEventListener: (evt: string, cb: () => void) => {
        listeners[evt] = cb;
      },
      open: vi.fn(),
      setRequestHeader: vi.fn(),
      send: vi.fn(() => {
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
    return { open: xhrMock.open };
  }

  it('uploadDocumentDirect best-effort deletes the orphan when registration fails (#2564)', async () => {
    // Deterministic compensating delete: after the bytes are PUT to S3 (step 2),
    // a failing register (step 3) must NOT be swallowed — the caller still sees
    // the real registration error — and the client must fire the auth+org-scoped
    // DELETE-by-file_key route to reap the orphan immediately.
    const { open } = stubSuccessfulS3Put();

    const registrationError = new Error('registration boom (HTTP 422)');
    // 1) upload-url ok  2) register rejects  3) compensating delete → 204
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
      .mockRejectedValueOnce(registrationError)
      .mockResolvedValueOnce(mock204Response());

    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

    const file = new File(['hello'], 'report.pdf', { type: 'application/pdf' });
    // The ORIGINAL registration error is re-thrown unchanged (not swallowed).
    await expect(
      uploadDocumentDirect({ file, title: 'Report', category: 'report', organizationId: 'org-1' })
    ).rejects.toBe(registrationError);

    // The S3 PUT (step 2) did happen — this is why the object is now orphaned.
    expect(open).toHaveBeenCalledWith('PUT', 'https://s3.example/bucket/key?sig=abc');
    // The compensating delete (step 3b) fired: DELETE to the by-file-key route
    // with the URL-encoded file_key.
    const [delUrl, delInit] = vi.mocked(fetch).mock.calls[2];
    expect(delUrl).toBe('/api/v1/documents/by-file-key?file_key=org%2F2026%2F07%2Fuuid_report.pdf');
    expect((delInit as RequestInit).method).toBe('DELETE');
    // Cleanup succeeded, so no orphan warning was logged.
    expect(warnSpy).not.toHaveBeenCalled();

    vi.unstubAllGlobals();
  });

  it('uploadDocumentDirect logs the orphan when the compensating delete also fails (#2564)', async () => {
    // If the best-effort delete itself fails, the orphan must be made observable
    // (greppable file_key) for the lifecycle-rule sweep to reconcile, and the
    // ORIGINAL registration error must still surface.
    stubSuccessfulS3Put();

    const registrationError = new Error('registration boom (HTTP 422)');
    // 1) upload-url ok  2) register rejects  3) compensating delete rejects
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
      .mockRejectedValueOnce(registrationError)
      .mockRejectedValueOnce(new Error('storage unavailable (HTTP 503)'));

    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

    const file = new File(['hello'], 'report.pdf', { type: 'application/pdf' });
    await expect(
      uploadDocumentDirect({ file, title: 'Report', category: 'report', organizationId: 'org-1' })
    ).rejects.toBe(registrationError);

    // The orphan is observable: the warning names the exact orphaned file_key.
    expect(warnSpy).toHaveBeenCalledTimes(1);
    expect(warnSpy.mock.calls[0][0]).toContain('org/2026/07/uuid_report.pdf');

    vi.unstubAllGlobals();
  });
});
