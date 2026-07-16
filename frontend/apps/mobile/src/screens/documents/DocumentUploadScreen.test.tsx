/**
 * Tests for DocumentUploadScreen (Story 7A-1, mobile slice).
 *
 * The `documents/` folder shipped tests for download / preview / share /
 * folders but none for the upload path — the highest-risk mutation flow in the
 * directory (GitHub #2284). This file pins the load-bearing logic:
 *
 *  - `getFileIcon` — MIME → icon mapping shown on the file-preview card. A
 *    wrong branch order silently mislabels the file the user is about to send.
 *  - `uploadDocumentMultipart` — the multipart POST: auth/tenant headers, the
 *    deliberate *absence* of a Content-Type (so fetch keeps the boundary), the
 *    fail-fast `UploadAuthError` when no token is readable, and the
 *    server-error message extraction surfaced to the user.
 *  - `validate()` — accept/reject form branching, exercised through the render
 *    tree (it is a component closure over picked-file / title / category
 *    state), including the client-side max-size / allowed-MIME guards.
 *
 * The shared JWT decode (`extractTenantId` / `decodeJwtPayload`) that feeds the
 * `X-Tenant-ID` header now lives in `src/utils/jwt.ts` and is covered by a
 * single suite in `src/utils/jwt.test.ts`.
 *
 * `../../config/api` is mocked to a fixed base URL and `globalThis.fetch` is stubbed
 * so no network is touched. `expo-secure-store` is mocked globally in the jest
 * setup; per-test we override `getItemAsync`.
 */

import { fireEvent, render, screen, waitFor } from '@testing-library/react-native';
import * as DocumentPicker from 'expo-document-picker';
import * as ImagePicker from 'expo-image-picker';
import * as SecureStore from 'expo-secure-store';
import { Alert } from 'react-native';
import {
  DocumentUploadScreen,
  getFileIcon,
  UploadAuthError,
  uploadDocumentMultipart,
} from './DocumentUploadScreen';

// ─── Mocks ──────────────────────────────────────────────────────────────────

const BASE_URL = 'https://api.test';
jest.mock('../../config/api', () => ({
  getApiBaseUrl: () => 'https://api.test',
}));

jest.mock('expo-document-picker', () => ({
  getDocumentAsync: jest.fn(),
}));

// Imported at module scope by the screen — stub so the import graph resolves.
jest.mock('expo-image-picker', () => ({
  requestMediaLibraryPermissionsAsync: jest.fn(),
  launchImageLibraryAsync: jest.fn(),
}));

const mockGetItemAsync = SecureStore.getItemAsync as jest.Mock;
const mockGetDocumentAsync = DocumentPicker.getDocumentAsync as jest.Mock;
const mockRequestMediaPerms = ImagePicker.requestMediaLibraryPermissionsAsync as jest.Mock;
const mockLaunchImageLibrary = ImagePicker.launchImageLibraryAsync as jest.Mock;

/** Build an unsigned JWT (`header.base64url(payload).sig`) for tenant decoding. */
function makeJwt(payload: Record<string, unknown>): string {
  // `btoa` is provided by the React Native / jest-expo runtime (same env that
  // supplies `atob`, which the screen's `extractTenantId` decodes with).
  const b64url = (obj: unknown) =>
    globalThis.btoa(JSON.stringify(obj)).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
  return `${b64url({ alg: 'none', typ: 'JWT' })}.${b64url(payload)}.sig`;
}

beforeEach(() => {
  jest.clearAllMocks();
  mockGetItemAsync.mockResolvedValue(null);
});

// ─── getFileIcon ────────────────────────────────────────────────────────────

describe('getFileIcon', () => {
  it.each([
    ['application/pdf', '📄'],
    ['image/png', '🖼️'],
    ['image/jpeg', '🖼️'],
    ['application/vnd.openxmlformats-officedocument.spreadsheetml.sheet', '📊'],
    ['application/vnd.ms-excel', '📊'],
    ['application/msword', '📝'],
    ['application/vnd.openxmlformats-officedocument.wordprocessingml.document', '📝'],
    ['text/plain', '📎'],
    ['application/octet-stream', '📎'],
  ])('maps %s → %s', (mimeType, expected) => {
    expect(getFileIcon(mimeType)).toBe(expected);
  });

  it('prefers the pdf branch even though a pdf MIME also contains "application"', () => {
    // pdf is checked before the generic fallback — pin the branch ordering.
    expect(getFileIcon('application/pdf')).toBe('📄');
  });
});

// ─── uploadDocumentMultipart ────────────────────────────────────────────────

describe('uploadDocumentMultipart', () => {
  const file = {
    uri: 'file:///cache/report.pdf',
    name: 'report.pdf',
    mimeType: 'application/pdf',
    size: 2048,
  };

  function stubFetch(impl: Partial<Response> & { json: () => Promise<unknown> }) {
    const fetchMock = jest.fn().mockResolvedValue(impl);
    globalThis.fetch = fetchMock as unknown as typeof fetch;
    return fetchMock;
  }

  it('POSTs multipart to the upload endpoint with auth + tenant headers and no Content-Type', async () => {
    const token = makeJwt({ tenant_id: 'org-xyz' });
    mockGetItemAsync.mockResolvedValue(token);
    const fetchMock = stubFetch({ ok: true, json: async () => ({ id: 'doc-1', message: 'ok' }) });

    const result = await uploadDocumentMultipart({
      file,
      title: '  Quarterly report  ',
      description: '   ',
      category: 'report',
    });

    expect(result).toEqual({ id: 'doc-1', message: 'ok' });
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, options] = fetchMock.mock.calls[0];
    expect(url).toBe(`${BASE_URL}/api/v1/documents/upload`);
    expect(options.method).toBe('POST');
    expect(options.headers.Authorization).toBe(`Bearer ${token}`);
    expect(options.headers['X-Tenant-ID']).toBe('org-xyz');
    // Content-Type must be omitted so fetch/FormData set the multipart boundary.
    expect(options.headers['Content-Type']).toBeUndefined();
    expect(options.body).toBeInstanceOf(FormData);
  });

  it('fails fast with UploadAuthError (no request sent) when there is no stored token', async () => {
    mockGetItemAsync.mockResolvedValue(null);
    const fetchMock = stubFetch({ ok: true, json: async () => ({ id: 'doc-2', message: 'ok' }) });

    // The endpoint is auth-protected, so an unauthenticated POST is a
    // guaranteed 401 — we must reject before streaming the multipart body.
    await expect(
      uploadDocumentMultipart({ file, title: 'No auth', description: '', category: 'other' })
    ).rejects.toBeInstanceOf(UploadAuthError);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('fails fast with UploadAuthError (no request sent) when reading the token throws', async () => {
    mockGetItemAsync.mockRejectedValue(new Error('keystore locked'));
    const fetchMock = stubFetch({ ok: true, json: async () => ({ id: 'doc-3', message: 'ok' }) });

    await expect(
      uploadDocumentMultipart({
        file,
        title: 'Locked keystore',
        description: '',
        category: 'other',
      })
    ).rejects.toBeInstanceOf(UploadAuthError);
    expect(fetchMock).not.toHaveBeenCalled();
  });

  it('surfaces the server-provided error message on a non-2xx JSON response', async () => {
    mockGetItemAsync.mockResolvedValue(makeJwt({ tenant_id: 'org-1' }));
    stubFetch({ ok: false, status: 422, json: async () => ({ message: 'File too large' }) });

    await expect(
      uploadDocumentMultipart({ file, title: 'Big', description: '', category: 'report' })
    ).rejects.toThrow('File too large');
  });

  it('falls back to an HTTP status message when the error body is not JSON', async () => {
    mockGetItemAsync.mockResolvedValue(makeJwt({ tenant_id: 'org-1' }));
    stubFetch({
      ok: false,
      status: 500,
      json: async () => {
        throw new Error('Unexpected token < in JSON');
      },
    });

    await expect(
      uploadDocumentMultipart({ file, title: 'Boom', description: '', category: 'report' })
    ).rejects.toThrow('HTTP 500');
  });
});

// ─── validate() via the render tree ─────────────────────────────────────────

describe('DocumentUploadScreen validate()', () => {
  let alertSpy: jest.SpyInstance;

  beforeEach(() => {
    globalThis.fetch = jest.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ id: 'doc-1', message: 'ok' }),
    }) as unknown as typeof fetch;
    // A valid token so the fail-fast auth guard doesn't short-circuit the
    // accept-branch tests.
    mockGetItemAsync.mockResolvedValue(makeJwt({ tenant_id: 'org-1' }));
    alertSpy = jest.spyOn(Alert, 'alert').mockImplementation(() => {});
  });

  afterEach(() => {
    alertSpy.mockRestore();
  });

  /** Pick a file through the document picker and wait for its preview card. */
  async function pickFile(asset: { uri: string; name: string; mimeType: string; size: number }) {
    mockGetDocumentAsync.mockResolvedValue({ canceled: false, assets: [asset] });
    fireEvent.press(screen.getByText('documents.upload.pickFile'));
    await waitFor(() => expect(screen.getByText(asset.name)).toBeTruthy());
  }

  it('rejects submit and shows all three field errors when the form is empty', () => {
    render(<DocumentUploadScreen />);

    fireEvent.press(screen.getByText('documents.upload.submitButton'));

    // Reject branch: every required field flags its error and nothing is sent.
    expect(screen.getByText('documents.upload.fileRequired')).toBeTruthy();
    expect(screen.getByText('documents.upload.titleRequired')).toBeTruthy();
    expect(screen.getByText('documents.upload.categoryRequired')).toBeTruthy();
    expect(globalThis.fetch).not.toHaveBeenCalled();
  });

  it('accepts submit once a file, title, and category are supplied and fires the upload', async () => {
    render(<DocumentUploadScreen />);

    // Pick a file — this also auto-populates the title from the filename.
    await pickFile({
      uri: 'file:///cache/lease.pdf',
      name: 'lease.pdf',
      mimeType: 'application/pdf',
      size: 4096,
    });

    // Select a category chip (labels are i18n keys under the test mock).
    fireEvent.press(screen.getByText('documents.upload.categories.contract'));

    fireEvent.press(screen.getByText('documents.upload.submitButton'));

    // Accept branch: validate() passed, so the multipart upload fired once.
    await waitFor(() => expect(globalThis.fetch).toHaveBeenCalledTimes(1));
    const [url, options] = (globalThis.fetch as jest.Mock).mock.calls[0];
    expect(url).toBe('https://api.test/api/v1/documents/upload');
    expect(options.method).toBe('POST');
  });

  it('rejects an oversize file client-side (before any upload)', async () => {
    render(<DocumentUploadScreen />);

    // 51 MiB — one over the 50 MiB backend cap.
    await pickFile({
      uri: 'file:///cache/huge.pdf',
      name: 'huge.pdf',
      mimeType: 'application/pdf',
      size: 51 * 1024 * 1024,
    });
    fireEvent.press(screen.getByText('documents.upload.categories.contract'));

    fireEvent.press(screen.getByText('documents.upload.submitButton'));

    expect(screen.getByText('documents.upload.fileTooLarge')).toBeTruthy();
    expect(globalThis.fetch).not.toHaveBeenCalled();
  });

  it('rejects a disallowed MIME type client-side (before any upload)', async () => {
    render(<DocumentUploadScreen />);

    // application/zip is not in the backend allow-list.
    await pickFile({
      uri: 'file:///cache/archive.zip',
      name: 'archive.zip',
      mimeType: 'application/zip',
      size: 1024,
    });
    fireEvent.press(screen.getByText('documents.upload.categories.contract'));

    fireEvent.press(screen.getByText('documents.upload.submitButton'));

    expect(screen.getByText('documents.upload.fileTypeNotAllowed')).toBeTruthy();
    expect(globalThis.fetch).not.toHaveBeenCalled();
  });

  it('allows a picker-reported size of 0 (unknown size) through the size guard', async () => {
    render(<DocumentUploadScreen />);

    // size: 0 means the picker didn't report a size — we can't enforce the
    // cap, so the upload proceeds (server 422 is the backstop) rather than
    // blocking a legitimate file.
    await pickFile({
      uri: 'file:///cache/report.pdf',
      name: 'report.pdf',
      mimeType: 'application/pdf',
      size: 0,
    });
    fireEvent.press(screen.getByText('documents.upload.categories.contract'));

    fireEvent.press(screen.getByText('documents.upload.submitButton'));

    await waitFor(() => expect(globalThis.fetch).toHaveBeenCalledTimes(1));
    expect(screen.queryByText('documents.upload.fileTooLarge')).toBeNull();
  });

  it('shows an error alert carrying the server message when the upload is rejected', async () => {
    globalThis.fetch = jest.fn().mockResolvedValue({
      ok: false,
      status: 422,
      json: async () => ({ message: 'File too large' }),
    }) as unknown as typeof fetch;

    render(<DocumentUploadScreen />);
    await pickFile({
      uri: 'file:///cache/report.pdf',
      name: 'report.pdf',
      mimeType: 'application/pdf',
      size: 4096,
    });
    fireEvent.press(screen.getByText('documents.upload.categories.contract'));

    fireEvent.press(screen.getByText('documents.upload.submitButton'));

    await waitFor(() =>
      expect(alertSpy).toHaveBeenCalledWith('documents.upload.errorTitle', 'File too large')
    );
  });

  it('disables the submit button while an upload is in flight (double-submit guard)', async () => {
    // A never-resolving fetch keeps the screen in the uploading state.
    let resolveFetch: (value: unknown) => void = () => {};
    const pending = new Promise((resolve) => {
      resolveFetch = resolve;
    });
    globalThis.fetch = jest.fn().mockReturnValue(pending) as unknown as typeof fetch;

    render(<DocumentUploadScreen />);
    await pickFile({
      uri: 'file:///cache/report.pdf',
      name: 'report.pdf',
      mimeType: 'application/pdf',
      size: 4096,
    });
    fireEvent.press(screen.getByText('documents.upload.categories.contract'));

    fireEvent.press(screen.getByText('documents.upload.submitButton'));

    // Upload started exactly once; the button label is replaced by the
    // in-progress spinner (so it can't be pressed again) and the uploading
    // status is shown.
    await waitFor(() => expect(globalThis.fetch).toHaveBeenCalledTimes(1));
    expect(screen.getByText('documents.upload.uploading')).toBeTruthy();
    expect(screen.queryByText('documents.upload.submitButton')).toBeNull();

    resolveFetch({ ok: true, json: async () => ({ id: 'doc-1', message: 'ok' }) });
    await waitFor(() => expect(screen.queryByText('documents.upload.uploading')).toBeNull());
    expect(globalThis.fetch).toHaveBeenCalledTimes(1);
  });
});

// ─── pickPhoto() MIME resolution (GitHub #2368) ───────────────────────────────

describe('DocumentUploadScreen pickPhoto()', () => {
  let alertSpy: jest.SpyInstance;

  beforeEach(() => {
    globalThis.fetch = jest.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ id: 'doc-1', message: 'ok' }),
    }) as unknown as typeof fetch;
    mockGetItemAsync.mockResolvedValue(makeJwt({ tenant_id: 'org-1' }));
    mockRequestMediaPerms.mockResolvedValue({ status: 'granted' });
    alertSpy = jest.spyOn(Alert, 'alert').mockImplementation(() => {});
  });

  afterEach(() => {
    alertSpy.mockRestore();
  });

  /** Pick a photo through the image picker and wait for its preview card. */
  async function pickPhoto(
    asset: { uri: string; mimeType?: string; fileName?: string; fileSize?: number },
    expectedName: string
  ) {
    mockLaunchImageLibrary.mockResolvedValue({ canceled: false, assets: [asset] });
    fireEvent.press(screen.getByText('documents.upload.pickPhoto'));
    await waitFor(() => expect(screen.getByText(expectedName)).toBeTruthy());
  }

  it('routes the picker-reported MIME through the same allow-list guard (HEIC rejected client-side)', async () => {
    render(<DocumentUploadScreen />);

    // An iOS capture the picker types as image/heic — not in the allow-list.
    // The old code hard-coded image/jpeg here, so the guard was a no-op and the
    // wrong type was uploaded; now validate() rejects it before any request.
    await pickPhoto(
      { uri: 'file:///DCIM/IMG_0001.HEIC', mimeType: 'image/heic', fileName: 'IMG_0001.HEIC' },
      'IMG_0001.HEIC'
    );
    fireEvent.press(screen.getByText('documents.upload.categories.contract'));
    fireEvent.press(screen.getByText('documents.upload.submitButton'));

    expect(screen.getByText('documents.upload.fileTypeNotAllowed')).toBeTruthy();
    expect(globalThis.fetch).not.toHaveBeenCalled();
  });

  it('prefers the picker-reported filename + MIME for an allowed photo and uploads it', async () => {
    render(<DocumentUploadScreen />);

    await pickPhoto(
      {
        uri: 'file:///DCIM/vacation.jpg',
        mimeType: 'image/jpeg',
        fileName: 'vacation.jpg',
        fileSize: 4096,
      },
      'vacation.jpg'
    );
    fireEvent.press(screen.getByText('documents.upload.categories.contract'));
    fireEvent.press(screen.getByText('documents.upload.submitButton'));

    await waitFor(() => expect(globalThis.fetch).toHaveBeenCalledTimes(1));
    expect(screen.queryByText('documents.upload.fileTypeNotAllowed')).toBeNull();
  });

  it('falls back to a lower-cased extension when the picker reports no MIME (.PNG accepted)', async () => {
    render(<DocumentUploadScreen />);

    // No mimeType and an upper-cased extension: the old `=== 'png'` check
    // mislabelled this image/jpeg; lower-casing resolves it to image/png, which
    // is in the allow-list, so the upload proceeds.
    await pickPhoto(
      { uri: 'file:///DCIM/IMG_1234.PNG', fileName: 'IMG_1234.PNG', fileSize: 2048 },
      'IMG_1234.PNG'
    );
    fireEvent.press(screen.getByText('documents.upload.categories.contract'));
    fireEvent.press(screen.getByText('documents.upload.submitButton'));

    await waitFor(() => expect(globalThis.fetch).toHaveBeenCalledTimes(1));
    expect(screen.queryByText('documents.upload.fileTypeNotAllowed')).toBeNull();
  });
});
