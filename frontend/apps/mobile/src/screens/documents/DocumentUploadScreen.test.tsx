/**
 * Tests for DocumentUploadScreen (Story 7A-1, mobile slice).
 *
 * The `documents/` folder shipped tests for download / preview / share /
 * folders but none for the upload path — the highest-risk mutation flow in the
 * directory (GitHub #2284). This file pins the load-bearing logic:
 *
 *  - `getFileIcon` — MIME → icon mapping shown on the file-preview card. A
 *    wrong branch order silently mislabels the file the user is about to send.
 *  - `extractTenantId` — JWT (base64url) decode that feeds the `X-Tenant-ID`
 *    header. Bad decoding → the upload is attributed to the wrong tenant or
 *    rejected. (Mirrors the `useApi` tenant logic — regression risk on both.)
 *  - `uploadDocumentMultipart` — the multipart POST: auth/tenant headers, the
 *    deliberate *absence* of a Content-Type (so fetch keeps the boundary), and
 *    the server-error message extraction surfaced to the user.
 *  - `validate()` — accept/reject form branching, exercised through the render
 *    tree (it is a component closure over picked-file / title / category state).
 *
 * `../../config/api` is mocked to a fixed base URL and `globalThis.fetch` is stubbed
 * so no network is touched. `expo-secure-store` is mocked globally in the jest
 * setup; per-test we override `getItemAsync`.
 */

import { fireEvent, render, screen, waitFor } from '@testing-library/react-native';
import * as DocumentPicker from 'expo-document-picker';
import * as SecureStore from 'expo-secure-store';
import {
  DocumentUploadScreen,
  extractTenantId,
  getFileIcon,
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

// ─── extractTenantId ────────────────────────────────────────────────────────

describe('extractTenantId', () => {
  it('decodes the tenant_id claim from a base64url JWT payload', () => {
    expect(extractTenantId(makeJwt({ tenant_id: 'org-7f3c' }))).toBe('org-7f3c');
  });

  it('returns null when the tenant_id claim is absent', () => {
    expect(extractTenantId(makeJwt({ sub: 'user-1' }))).toBeNull();
  });

  it('returns null when tenant_id is not a string', () => {
    expect(extractTenantId(makeJwt({ tenant_id: 42 }))).toBeNull();
  });

  it('returns null for a token with fewer than two segments', () => {
    expect(extractTenantId('not-a-jwt')).toBeNull();
  });

  it('returns null when the payload segment is not valid base64/JSON', () => {
    expect(extractTenantId('header.%%%not-base64%%%.sig')).toBeNull();
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

  it('omits the auth + tenant headers when there is no stored token', async () => {
    mockGetItemAsync.mockResolvedValue(null);
    const fetchMock = stubFetch({ ok: true, json: async () => ({ id: 'doc-2', message: 'ok' }) });

    await uploadDocumentMultipart({ file, title: 'No auth', description: '', category: 'other' });

    const [, options] = fetchMock.mock.calls[0];
    expect(options.headers.Authorization).toBeUndefined();
    expect(options.headers['X-Tenant-ID']).toBeUndefined();
  });

  it('still uploads (unauthenticated) when reading the token throws', async () => {
    mockGetItemAsync.mockRejectedValue(new Error('keystore locked'));
    const fetchMock = stubFetch({ ok: true, json: async () => ({ id: 'doc-3', message: 'ok' }) });

    const result = await uploadDocumentMultipart({
      file,
      title: 'Locked keystore',
      description: '',
      category: 'other',
    });

    expect(result).toEqual({ id: 'doc-3', message: 'ok' });
    const [, options] = fetchMock.mock.calls[0];
    expect(options.headers.Authorization).toBeUndefined();
  });

  it('surfaces the server-provided error message on a non-2xx JSON response', async () => {
    stubFetch({ ok: false, status: 422, json: async () => ({ message: 'File too large' }) });

    await expect(
      uploadDocumentMultipart({ file, title: 'Big', description: '', category: 'report' })
    ).rejects.toThrow('File too large');
  });

  it('falls back to an HTTP status message when the error body is not JSON', async () => {
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
  beforeEach(() => {
    globalThis.fetch = jest.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ id: 'doc-1', message: 'ok' }),
    }) as unknown as typeof fetch;
  });

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
    mockGetDocumentAsync.mockResolvedValue({
      canceled: false,
      assets: [
        {
          uri: 'file:///cache/lease.pdf',
          name: 'lease.pdf',
          mimeType: 'application/pdf',
          size: 4096,
        },
      ],
    });

    render(<DocumentUploadScreen />);

    // Pick a file — this also auto-populates the title from the filename.
    fireEvent.press(screen.getByText('documents.upload.pickFile'));
    await waitFor(() => expect(screen.getByText('lease.pdf')).toBeTruthy());

    // Select a category chip (labels are i18n keys under the test mock).
    fireEvent.press(screen.getByText('documents.upload.categories.contract'));

    fireEvent.press(screen.getByText('documents.upload.submitButton'));

    // Accept branch: validate() passed, so the multipart upload fired once.
    await waitFor(() => expect(globalThis.fetch).toHaveBeenCalledTimes(1));
    const [url, options] = (globalThis.fetch as jest.Mock).mock.calls[0];
    expect(url).toBe('https://api.test/api/v1/documents/upload');
    expect(options.method).toBe('POST');
  });
});
