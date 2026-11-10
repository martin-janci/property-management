/// <reference types="vitest/globals" />
/**
 * Integration regression test for the document upload component (gap-84-1).
 *
 * The Story-39.2 upload UI was migrated from the byte-proxying multipart
 * `POST /api/v1/documents/upload` path (`useUploadDocument`) to the
 * direct-to-S3 path (`useUploadDocumentDirect` →
 * `POST /api/v1/documents/upload-url` → presigned S3 PUT → register). The
 * api-client layer of that flow is pinned by `api.test.ts`; this test pins the
 * *component integration* so it cannot silently regress back to proxying bytes
 * through the api-server:
 *
 *   1. selecting a file + confirming drives the DIRECT upload mutation
 *      (`useUploadDocumentDirect`), carrying the file, title and category;
 *   2. the legacy multipart mutation (`useUploadDocument`) is never invoked;
 *   3. a successful upload surfaces the returned document id via
 *      `onUploadComplete`.
 *
 * Only the `@ppt/api-client` upload hooks (the network boundary) are mocked.
 */

import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { DocumentUpload } from './DocumentUpload';

// ─── network boundary: @ppt/api-client upload hooks ─────────────────────────
const directMutateAsync = vi.fn();
const legacyMutateAsync = vi.fn();
const useUploadDocumentDirectMock = vi.fn(() => ({
  mutateAsync: directMutateAsync,
  isPending: false,
}));
const useUploadDocumentMock = vi.fn(() => ({
  mutateAsync: legacyMutateAsync,
  isPending: false,
}));

vi.mock('@ppt/api-client', () => ({
  // Real category list the component renders in its <select>.
  DOCUMENT_CATEGORIES: [
    'contract',
    'invoice',
    'receipt',
    'report',
    'minutes',
    'policy',
    'notice',
    'maintenance',
    'insurance',
    'legal',
    'financial',
    'correspondence',
    'other',
  ],
  useUploadDocumentDirect: () => useUploadDocumentDirectMock(),
  // Legacy multipart hook — exposed so the test can assert it is NOT used.
  useUploadDocument: () => useUploadDocumentMock(),
}));

function selectFile(container: HTMLElement, file: File) {
  const input = container.querySelector('input[type="file"]') as HTMLInputElement;
  fireEvent.change(input, { target: { files: [file] } });
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe('DocumentUpload — direct-to-S3 integration (gap-84-1)', () => {
  it('drives the direct-to-S3 upload mutation, not the legacy multipart path', async () => {
    directMutateAsync.mockResolvedValue({ id: 'doc-42', message: 'created' });
    const onUploadComplete = vi.fn();

    const { container } = render(
      <DocumentUpload organizationId="org-1" onUploadComplete={onUploadComplete} />
    );

    // Selecting a file auto-fills the title from the filename, enabling upload.
    const file = new File(['hello'], 'report.pdf', { type: 'application/pdf' });
    selectFile(container, file);

    fireEvent.click(screen.getByRole('button', { name: /Upload & Process/i }));

    await waitFor(() => {
      expect(directMutateAsync).toHaveBeenCalledTimes(1);
    });

    // The direct-to-S3 mutation carried the file, the derived title and the
    // selected category (defaults to 'other').
    expect(directMutateAsync).toHaveBeenCalledWith(
      expect.objectContaining({
        file,
        title: 'report',
        category: 'other',
        organizationId: 'org-1',
      })
    );

    // The byte-proxying multipart path must never fire — the whole point of
    // gap-84-1 is that bytes go straight to S3.
    expect(legacyMutateAsync).not.toHaveBeenCalled();

    // The uploaded document id is surfaced to the caller.
    await waitFor(() => {
      expect(onUploadComplete).toHaveBeenCalledWith('doc-42');
    });
  });

  it('does not upload when no file is selected', () => {
    render(<DocumentUpload organizationId="org-1" />);

    const uploadBtn = screen.getByRole('button', { name: /Upload & Process/i });
    // With no pending file the action is disabled and firing it is a no-op.
    expect(uploadBtn).toBeDisabled();
    fireEvent.click(uploadBtn);

    expect(directMutateAsync).not.toHaveBeenCalled();
    expect(legacyMutateAsync).not.toHaveBeenCalled();
  });
});
