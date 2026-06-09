/**
 * Unit tests for the load-bearing helpers in the document download + preview
 * flow (Story 7A.4, mobile slice).
 *
 * DocumentPreviewScreen.test.tsx already pins the preview→download fallback
 * classifier (`isPreviewUnsupportedError`). This file covers the remaining
 * load-bearing logic that has direct user-visible / interop consequences:
 *
 *  - `downloadToCache` — the cache-reuse decision: a second share of the same
 *    file MUST NOT re-download the presigned blob (correctness + bandwidth).
 *  - `mimeTypeFromDocType` / `utiFromDocType` — wrong values silently break
 *    the share-intent target resolution on Android / iOS respectively.
 *  - `formatBytes` — the size label shown on the preview card.
 *
 * expo-file-system/legacy is mocked so the download path is exercised without
 * touching a real device filesystem.
 */

// ─── Mock expo-file-system/legacy BEFORE importing the screen module ─────────────

const mockGetInfoAsync = jest.fn();
const mockDownloadAsync = jest.fn();

jest.mock('expo-file-system/legacy', () => ({
  get cacheDirectory() {
    return 'file:///cache/';
  },
  getInfoAsync: (...args: unknown[]) => mockGetInfoAsync(...args),
  downloadAsync: (...args: unknown[]) => mockDownloadAsync(...args),
}));

// expo-sharing is imported at module scope by DocumentPreviewScreen; stub it so
// the import doesn't reach into native code during the test.
jest.mock('expo-sharing', () => ({
  isAvailableAsync: jest.fn(),
  shareAsync: jest.fn(),
}));

import {
  downloadToCache,
  formatBytes,
  mimeTypeFromDocType,
  utiFromDocType,
} from './DocumentPreviewScreen';

beforeEach(() => {
  mockGetInfoAsync.mockReset();
  mockDownloadAsync.mockReset();
});

describe('downloadToCache', () => {
  it('downloads to the cache directory when no local copy exists', async () => {
    mockGetInfoAsync.mockResolvedValue({ exists: false });
    mockDownloadAsync.mockResolvedValue({ uri: 'file:///cache/report.pdf' });

    const uri = await downloadToCache('https://s3.example/presigned', 'report.pdf');

    expect(mockGetInfoAsync).toHaveBeenCalledWith('file:///cache/report.pdf');
    expect(mockDownloadAsync).toHaveBeenCalledWith(
      'https://s3.example/presigned',
      'file:///cache/report.pdf'
    );
    expect(uri).toBe('file:///cache/report.pdf');
  });

  it('reuses the cached copy without re-downloading when it already exists', async () => {
    mockGetInfoAsync.mockResolvedValue({ exists: true });

    const uri = await downloadToCache('https://s3.example/presigned', 'report.pdf');

    expect(mockDownloadAsync).not.toHaveBeenCalled();
    expect(uri).toBe('file:///cache/report.pdf');
  });

  it('returns the URI reported by downloadAsync (may differ from the requested dest)', async () => {
    mockGetInfoAsync.mockResolvedValue({ exists: false });
    mockDownloadAsync.mockResolvedValue({ uri: 'file:///cache/redirected.pdf' });

    const uri = await downloadToCache('https://s3.example/presigned', 'report.pdf');

    expect(uri).toBe('file:///cache/redirected.pdf');
  });
});

describe('mimeTypeFromDocType', () => {
  it.each([
    ['pdf', 'application/pdf'],
    ['image', 'image/*'],
    ['spreadsheet', 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'],
  ] as const)('maps %s → %s', (type, expected) => {
    expect(mimeTypeFromDocType(type)).toBe(expected);
  });

  it.each(['folder', 'document'] as const)('falls back to octet-stream for %s', (type) => {
    expect(mimeTypeFromDocType(type)).toBe('application/octet-stream');
  });
});

describe('utiFromDocType', () => {
  it('maps pdf to the Adobe PDF UTI', () => {
    expect(utiFromDocType('pdf')).toBe('com.adobe.pdf');
  });

  it('maps image to public.image', () => {
    expect(utiFromDocType('image')).toBe('public.image');
  });

  it.each([
    'folder',
    'document',
    'spreadsheet',
  ] as const)('falls back to public.data for %s', (type) => {
    expect(utiFromDocType(type)).toBe('public.data');
  });
});

describe('formatBytes', () => {
  it.each([
    [0, '0 B'],
    [512, '512 B'],
    [1023, '1023 B'],
    [1024, '1.0 KB'],
    [1536, '1.5 KB'],
    [1024 * 1024, '1.0 MB'],
    [Math.round(2.5 * 1024 * 1024), '2.5 MB'],
  ])('formats %d bytes as %s', (bytes, expected) => {
    expect(formatBytes(bytes)).toBe(expected);
  });
});
