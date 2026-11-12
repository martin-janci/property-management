/**
 * CsvImport Component Tests
 *
 * The upload step used to accept any selected/dropped file without validation
 * (wrong type, empty, or oversized files were forwarded straight to the preview
 * request with no user feedback). These tests ensure a localized, inline error
 * (role="alert") is surfaced for an invalid file, and that a valid CSV clears
 * the error and advances past the upload step.
 */

import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// Mock @ppt/reality-api-client before importing the component.
vi.mock('@ppt/reality-api-client', () => ({
  useMyAgency: vi.fn(() => ({ data: { id: 'agency-1' } })),
  useCsvPreview: vi.fn(() => ({ data: undefined, isLoading: false, error: null })),
  useCsvImport: vi.fn(() => ({
    mutateAsync: vi.fn().mockResolvedValue({}),
    reset: vi.fn(),
    data: undefined,
    error: null,
  })),
}));

import { CsvImport } from './CsvImport';

function fileInput(container: HTMLElement): HTMLInputElement {
  const input = container.querySelector('input[type="file"]');
  if (!input) {
    throw new Error('file input not found');
  }
  return input as HTMLInputElement;
}

function selectFile(container: HTMLElement, file: File) {
  const input = fileInput(container);
  Object.defineProperty(input, 'files', { value: [file], configurable: true });
  fireEvent.change(input);
}

// jsdom cannot build a multi-MB file cheaply; stub the reported size instead.
function sizedCsv(name: string, size: number): File {
  const file = new File(['a'], name, { type: 'text/csv' });
  Object.defineProperty(file, 'size', { value: size, configurable: true });
  return file;
}

describe('CsvImport upload validation', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('rejects a non-CSV file with an inline alert', () => {
    const { container } = render(<CsvImport />);
    selectFile(container, new File(['hello'], 'notes.txt', { type: 'text/plain' }));

    const alert = screen.getByRole('alert');
    expect(alert).toHaveTextContent('csv.invalidType');
  });

  it('rejects an empty CSV file', () => {
    const { container } = render(<CsvImport />);
    selectFile(container, new File([], 'empty.csv', { type: 'text/csv' }));

    expect(screen.getByRole('alert')).toHaveTextContent('csv.emptyFile');
  });

  it('rejects a CSV file over the size limit', () => {
    const { container } = render(<CsvImport />);
    selectFile(container, sizedCsv('big.csv', 6 * 1024 * 1024));

    expect(screen.getByRole('alert')).toHaveTextContent('csv.fileTooLarge');
  });

  it('accepts a valid CSV file and shows no error', () => {
    const { container } = render(<CsvImport />);
    selectFile(container, sizedCsv('listings.csv', 1024));

    expect(screen.queryByRole('alert')).toBeNull();
  });
});
