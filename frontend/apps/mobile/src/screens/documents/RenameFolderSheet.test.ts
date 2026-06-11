/**
 * Unit tests for the rename-folder slice
 * (feat-folder-organization-mobile-slice).
 *
 * RenameFolderSheet renders a bottom-sheet that PATCHes
 * /api/v1/documents/folders/{id}. The render path delegates to the
 * useApiMutation contract already covered elsewhere; what is load-bearing
 * and unique here are the two pure helpers that gate the request:
 *
 *   1. validateFolderName — enforces the same 1–255 char rule as the backend
 *      before a PATCH is sent.
 *   2. buildRenamePayload — builds a minimal PATCH body so a no-op rename
 *      sends nothing and a real rename only includes the changed `name`.
 */

import { buildRenamePayload, validateFolderName } from './RenameFolderSheet';

describe('validateFolderName', () => {
  it('rejects an empty name', () => {
    expect(validateFolderName('')).toEqual({ ok: false, reason: 'empty' });
  });

  it('rejects a whitespace-only name', () => {
    expect(validateFolderName('   ')).toEqual({ ok: false, reason: 'empty' });
  });

  it('accepts a normal name', () => {
    expect(validateFolderName('Contracts 2024')).toEqual({ ok: true });
  });

  it('accepts a name at the 255-char boundary', () => {
    expect(validateFolderName('a'.repeat(255))).toEqual({ ok: true });
  });

  it('rejects a name over 255 chars', () => {
    expect(validateFolderName('a'.repeat(256))).toEqual({
      ok: false,
      reason: 'tooLong',
    });
  });

  it('trims before measuring length', () => {
    expect(validateFolderName(`  ${'a'.repeat(255)}  `)).toEqual({ ok: true });
  });
});

describe('buildRenamePayload', () => {
  it('includes the trimmed name when it changed', () => {
    expect(buildRenamePayload('  Invoices  ', 'Contracts')).toEqual({
      name: 'Invoices',
    });
  });

  it('omits name for a no-op rename (same value)', () => {
    expect(buildRenamePayload('Contracts', 'Contracts')).toEqual({});
  });

  it('treats a name that only differs by surrounding whitespace as a no-op', () => {
    expect(buildRenamePayload('  Contracts  ', 'Contracts')).toEqual({});
  });

  it('omits name when the new value is empty', () => {
    expect(buildRenamePayload('   ', 'Contracts')).toEqual({});
  });
});
