/// <reference types="vitest/globals" />
/**
 * EvidenceUploader component tests (Epic 80, Story 80.2 — AC-2).
 *
 * Regression coverage for the dispute-filing evidence surface:
 *  - accepted file types / extensions exposed on the input
 *  - per-file type + size validation (error status, no preview)
 *  - MAX_FILES cap (drop zone hidden, extra files dropped)
 *  - remove + description editing callbacks
 *  - status indicators (uploading / uploaded / error)
 */

import { fireEvent, render, screen } from '@testing-library/react';
import { EvidenceUploader, type PendingEvidence } from './EvidenceUploader';

const MAX_FILES = 10;
const MAX_FILE_SIZE = 50 * 1024 * 1024; // 50 MB — mirrors the component cap.

// jsdom has no real object-URL support — stub it so image previews don't throw.
const mockCreateObjectURL = vi.fn(() => 'blob:mock-url');
const mockRevokeObjectURL = vi.fn();
URL.createObjectURL = mockCreateObjectURL;
URL.revokeObjectURL = mockRevokeObjectURL;

const onChange = vi.fn();

beforeEach(() => {
  vi.clearAllMocks();
});

/** Build a File with an overridable size (jsdom derives size from blob parts). */
function makeFile(name: string, type: string, sizeBytes = 1024): File {
  const file = new File(['x'], name, { type });
  Object.defineProperty(file, 'size', { value: sizeBytes });
  return file;
}

/** Build a single pending-evidence entry, overriding any field as needed. */
function pendingEvidence(overrides: Partial<PendingEvidence> = {}): PendingEvidence {
  return {
    id: 'ev-1',
    file: makeFile('photo.jpg', 'image/jpeg'),
    description: '',
    status: 'pending',
    ...overrides,
  };
}

/** Build `count` pending PDF entries — used for the MAX_FILES cap cases. */
function pdfQueue(count: number): PendingEvidence[] {
  return Array.from({ length: count }, (_, i) =>
    pendingEvidence({ id: `ev-${i}`, file: makeFile(`f${i}.pdf`, 'application/pdf') })
  );
}

/** Render the uploader with a fresh `onChange` spy and the given queue. */
function renderUploader(files: PendingEvidence[] = []) {
  return render(<EvidenceUploader files={files} onChange={onChange} />);
}

/** Drive the hidden file input with a selection of files. */
function selectFiles(files: File[]) {
  const input = document.querySelector('input[type="file"]') as HTMLInputElement;
  Object.defineProperty(input, 'files', { value: files, configurable: true });
  fireEvent.change(input);
}

/** The queue handed to the most recent `onChange` call (asserts one happened). */
function lastChange(): PendingEvidence[] {
  expect(onChange).toHaveBeenCalled();
  return onChange.mock.lastCall?.[0] as PendingEvidence[];
}

describe('EvidenceUploader', () => {
  describe('empty state', () => {
    it('renders the drop zone with the accepted-format hint', () => {
      renderUploader();

      expect(
        screen.getByRole('button', { name: /drop evidence files here or click to browse/i })
      ).toBeInTheDocument();
      expect(screen.getByText(/JPG, PNG, PDF, MP3, MP4/i)).toBeInTheDocument();
    });

    it('exposes the accepted extensions and multiple attribute on the file input', () => {
      renderUploader();

      const input = document.querySelector('input[type="file"]') as HTMLInputElement;
      expect(input).toHaveAttribute('accept', '.jpg,.jpeg,.png,.webp,.mp3,.mp4,.pdf');
      expect(input).toHaveAttribute('multiple');
    });
  });

  describe('file selection + validation', () => {
    it('accepts a valid file with pending status and creates an image preview', () => {
      renderUploader();

      selectFiles([makeFile('photo.jpg', 'image/jpeg')]);

      expect(onChange).toHaveBeenCalledTimes(1);
      const next = lastChange();
      expect(next).toHaveLength(1);
      expect(next[0].status).toBe('pending');
      expect(next[0].error).toBeUndefined();
      expect(next[0].preview).toBe('blob:mock-url');
      expect(mockCreateObjectURL).toHaveBeenCalledTimes(1);
    });

    it('marks an unsupported file type as error and skips the preview', () => {
      renderUploader();

      selectFiles([makeFile('virus.exe', 'application/x-msdownload')]);

      const next = lastChange();
      expect(next[0].status).toBe('error');
      expect(next[0].error).toMatch(/unsupported file type/i);
      expect(next[0].preview).toBeUndefined();
      expect(mockCreateObjectURL).not.toHaveBeenCalled();
    });

    it('marks a file exceeding the 50 MB limit as error', () => {
      renderUploader();

      selectFiles([makeFile('huge.pdf', 'application/pdf', MAX_FILE_SIZE + 1)]);

      const next = lastChange();
      expect(next[0].status).toBe('error');
      expect(next[0].error).toMatch(/max 50 MB/i);
    });
  });

  describe('MAX_FILES cap', () => {
    it('hides the drop zone but keeps listing files when full', () => {
      renderUploader(pdfQueue(MAX_FILES));

      // The clickable drop zone is conditional; the hidden input stays mounted.
      expect(
        screen.queryByRole('button', { name: /drop evidence files here/i })
      ).not.toBeInTheDocument();
      expect(screen.getByRole('list', { name: /evidence files/i })).toBeInTheDocument();
    });

    it('drops files beyond the remaining slots', () => {
      renderUploader(pdfQueue(MAX_FILES - 1));

      // Two incoming files, but only one slot remains.
      selectFiles([makeFile('a.pdf', 'application/pdf'), makeFile('b.pdf', 'application/pdf')]);

      expect(lastChange()).toHaveLength(MAX_FILES);
    });
  });

  describe('per-file actions', () => {
    it('removes a queued file and revokes its object URL', () => {
      renderUploader([pendingEvidence({ preview: 'blob:mock-url' })]);

      fireEvent.click(screen.getByRole('button', { name: /remove photo\.jpg/i }));

      expect(mockRevokeObjectURL).toHaveBeenCalledWith('blob:mock-url');
      expect(onChange).toHaveBeenCalledWith([]);
    });

    it('edits a per-file description through onChange', () => {
      renderUploader([pendingEvidence({ file: makeFile('clip.mp4', 'video/mp4') })]);

      fireEvent.change(screen.getByLabelText(/description for clip\.mp4/i), {
        target: { value: 'noise recording' },
      });

      expect(lastChange()[0].description).toBe('noise recording');
    });
  });

  describe('status indicators', () => {
    it('shows the error message for an errored file and omits its description input', () => {
      renderUploader([
        pendingEvidence({
          file: makeFile('bad.exe', 'application/x-msdownload'),
          status: 'error',
          error: 'Unsupported file type: application/x-msdownload',
        }),
      ]);

      expect(screen.getByText(/unsupported file type/i)).toBeInTheDocument();
      expect(screen.queryByLabelText(/description for bad\.exe/i)).not.toBeInTheDocument();
    });

    it('renders an uploading spinner and a disabled remove button mid-upload', () => {
      renderUploader([
        pendingEvidence({ file: makeFile('doc.pdf', 'application/pdf'), status: 'uploading' }),
      ]);

      expect(screen.getByLabelText(/uploading/i)).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /remove doc\.pdf/i })).toBeDisabled();
    });

    it('renders an uploaded checkmark when a file finished uploading', () => {
      renderUploader([
        pendingEvidence({ file: makeFile('doc.pdf', 'application/pdf'), status: 'uploaded' }),
      ]);

      expect(screen.getByLabelText(/uploaded/i)).toBeInTheDocument();
    });
  });
});
