/**
 * Regression tests for DocumentPreviewScreen — Story 7A.4 (mobile slice).
 *
 * The load-bearing logic in the document download + preview flow is the
 * decision of whether a failed GET /api/v1/documents/{id}/preview should fall
 * back to GET /api/v1/documents/{id}/download (file-type not previewable) or be
 * surfaced as a hard error (auth/network/5xx). That classifier —
 * `isPreviewUnsupportedError` — is shared between DocumentPreviewScreen and
 * DocumentDetailScreen, so a regression here would break the fallback on both.
 *
 * These tests pin the classifier directly (no RN render tree), which keeps them
 * runnable regardless of the @testing-library/react-native peer-dep state.
 */

import { isPreviewUnsupportedError } from './DocumentPreviewScreen';

describe('isPreviewUnsupportedError', () => {
  it.each([
    ['the explicit backend code', 'PREVIEW_NOT_SUPPORTED'],
    ['a human-readable "not supported" message', 'Preview is not supported for this type'],
    ['a raw HTTP 400 status', 'HTTP 400 Bad Request'],
  ])('returns true for an unsupported-preview error: %s', (_label, message) => {
    expect(isPreviewUnsupportedError(new Error(message))).toBe(true);
  });

  it.each([
    ['an auth error', 'HTTP 401 Unauthorized'],
    ['a forbidden error', 'HTTP 403 Forbidden'],
    ['a server error', 'HTTP 500 Internal Server Error'],
    ['a network error', 'Network request failed'],
  ])('returns false for a hard failure that must surface: %s', (_label, message) => {
    expect(isPreviewUnsupportedError(new Error(message))).toBe(false);
  });

  it.each([
    ['null', null],
    ['undefined', undefined],
    ['a bare string', 'PREVIEW_NOT_SUPPORTED'],
    ['a plain object', { message: 'PREVIEW_NOT_SUPPORTED' }],
  ])('returns false for non-Error throwables: %s', (_label, value) => {
    // Only real Error instances carry a trusted `.message`; anything else is
    // treated as a hard failure rather than silently falling back to download.
    expect(isPreviewUnsupportedError(value)).toBe(false);
  });
});
