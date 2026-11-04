/**
 * A failed document upload shows a translated error and can be retried
 * (round-3 audit #26). The row used to show the api-client's English
 * "Network connection lost. …" text, offer no retry and leave the Upload
 * button disabled.
 */
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import i18n from 'i18next';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import sk from '../../../../messages/sk.json';
import { DocumentUpload, uploadErrorMessage } from './DocumentUpload';

const directMutateAsync = vi.fn();

vi.mock('@ppt/api-client', () => ({
  DOCUMENT_CATEGORIES: ['contract', 'other'],
  useUploadDocumentDirect: () => ({ mutateAsync: directMutateAsync, isPending: false }),
  useUploadDocument: () => ({ mutateAsync: vi.fn(), isPending: false }),
}));

const NETWORK_ERROR = new Error(
  'Network connection lost. Please check your internet connection and try again.'
);
const U = sk.documents.uploadForm;

function selectFile(container: HTMLElement) {
  const input = container.querySelector('input[type="file"]') as HTMLInputElement;
  fireEvent.change(input, {
    target: { files: [new File(['x'], 'report.pdf', { type: 'application/pdf' })] },
  });
}

beforeEach(async () => {
  vi.clearAllMocks();
  i18n.addResourceBundle('sk', 'translation', sk, true, true);
  await i18n.changeLanguage('sk');
});

afterEach(async () => {
  await i18n.changeLanguage('en');
});

describe('uploadErrorMessage', () => {
  const t = (key: string) => key;
  it('maps network, cancel and other failures to translated keys', () => {
    expect(uploadErrorMessage(NETWORK_ERROR, t)).toBe('documents.uploadForm.networkError');
    expect(uploadErrorMessage(new TypeError('Failed to fetch'), t)).toBe(
      'documents.uploadForm.networkError'
    );
    expect(uploadErrorMessage(new Error('Upload was cancelled.'), t)).toBe(
      'documents.uploadForm.uploadCancelled'
    );
    expect(uploadErrorMessage(new Error('HTTP 500'), t)).toBe('documents.uploadForm.uploadFailed');
  });
});

describe('DocumentUpload failure and retry', () => {
  it('shows the Slovak network error and uploads again after retry', async () => {
    directMutateAsync.mockRejectedValueOnce(NETWORK_ERROR);
    directMutateAsync.mockResolvedValueOnce({ id: 'doc-1', message: 'ok' });
    const onUploadComplete = vi.fn();
    const { container } = render(
      <DocumentUpload organizationId="org-1" onUploadComplete={onUploadComplete} />
    );
    selectFile(container);

    const submit = container.querySelector('button.btn-primary') as HTMLButtonElement;
    fireEvent.click(submit);

    expect(await screen.findByText(U.networkError)).toBeTruthy();
    expect(screen.queryByText(/Network connection lost/)).toBeNull();
    expect(submit).toBeDisabled();

    fireEvent.click(
      screen.getByRole('button', { name: U.retryFile.replace('{{name}}', 'report.pdf') })
    );
    expect(screen.queryByText(U.networkError)).toBeNull();
    expect(submit).not.toBeDisabled();

    fireEvent.click(submit);
    await waitFor(() => expect(onUploadComplete).toHaveBeenCalledWith('doc-1'));
    expect(directMutateAsync).toHaveBeenCalledTimes(2);
  });
});
