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

// jsdom has no real object-URL support — stub it so image previews don't throw.
const mockCreateObjectURL = vi.fn(() => 'blob:mock-url');
const mockRevokeObjectURL = vi.fn();
URL.createObjectURL = mockCreateObjectURL;
URL.revokeObjectURL = mockRevokeObjectURL;

function makeFile(name: string, type: string, sizeBytes = 1024): File {
  const file = new File(['x'], name, { type });
  // jsdom derives size from the blob parts; override for size-limit tests.
  Object.defineProperty(file, 'size', { value: sizeBytes });
  return file;
}

function selectFiles(files: File[]) {
  const input = document.querySelector('input[type="file"]') as HTMLInputElement;
  Object.defineProperty(input, 'files', { value: files, configurable: true });
  fireEvent.change(input);
}

describe('EvidenceUploader', () => {
  const onChange = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('renders the drop zone with the accepted-format hint when empty', () => {
    render(<EvidenceUploader files={[]} onChange={onChange} />);

    expect(
      screen.getByRole('button', { name: /drop evidence files here or click to browse/i })
    ).toBeInTheDocument();
    expect(screen.getByText(/JPG, PNG, PDF, MP3, MP4/i)).toBeInTheDocument();
  });

  it('exposes the accepted extensions and multiple attribute on the file input', () => {
    render(<EvidenceUploader files={[]} onChange={onChange} />);

    const input = document.querySelector('input[type="file"]') as HTMLInputElement;
    expect(input).toHaveAttribute('accept', '.jpg,.jpeg,.png,.webp,.mp3,.mp4,.pdf');
    expect(input).toHaveAttribute('multiple');
  });

  it('accepts a valid file with pending status and creates an image preview', () => {
    render(<EvidenceUploader files={[]} onChange={onChange} />);

    selectFiles([makeFile('photo.jpg', 'image/jpeg')]);

    expect(onChange).toHaveBeenCalledTimes(1);
    const next: PendingEvidence[] = onChange.mock.calls[0][0];
    expect(next).toHaveLength(1);
    expect(next[0].status).toBe('pending');
    expect(next[0].error).toBeUndefined();
    expect(next[0].preview).toBe('blob:mock-url');
    expect(mockCreateObjectURL).toHaveBeenCalledTimes(1);
  });

  it('marks an unsupported file type as error and skips the preview', () => {
    render(<EvidenceUploader files={[]} onChange={onChange} />);

    selectFiles([makeFile('virus.exe', 'application/x-msdownload')]);

    const next: PendingEvidence[] = onChange.mock.calls[0][0];
    expect(next[0].status).toBe('error');
    expect(next[0].error).toMatch(/unsupported file type/i);
    expect(next[0].preview).toBeUndefined();
    expect(mockCreateObjectURL).not.toHaveBeenCalled();
  });

  it('marks a file exceeding the 50 MB limit as error', () => {
    render(<EvidenceUploader files={[]} onChange={onChange} />);

    selectFiles([makeFile('huge.pdf', 'application/pdf', 51 * 1024 * 1024)]);

    const next: PendingEvidence[] = onChange.mock.calls[0][0];
    expect(next[0].status).toBe('error');
    expect(next[0].error).toMatch(/max 50 MB/i);
  });

  it('caps the queue at 10 files and hides the drop zone when full', () => {
    const full: PendingEvidence[] = Array.from({ length: 10 }, (_, i) => ({
      id: `ev-${i}`,
      file: makeFile(`f${i}.pdf`, 'application/pdf'),
      description: '',
      status: 'pending',
    }));

    render(<EvidenceUploader files={full} onChange={onChange} />);

    // Drop zone is gone once the cap is reached (the hidden file input itself
    // stays mounted; only the clickable drop zone is conditional).
    expect(
      screen.queryByRole('button', { name: /drop evidence files here/i })
    ).not.toBeInTheDocument();
    // All 10 queued files are still listed.
    expect(screen.getByRole('list', { name: /evidence files/i })).toBeInTheDocument();
  });

  it('drops files beyond the remaining slots', () => {
    const existing: PendingEvidence[] = Array.from({ length: 9 }, (_, i) => ({
      id: `ev-${i}`,
      file: makeFile(`f${i}.pdf`, 'application/pdf'),
      description: '',
      status: 'pending',
    }));

    render(<EvidenceUploader files={existing} onChange={onChange} />);

    // Two incoming files, but only one slot remains.
    selectFiles([makeFile('a.pdf', 'application/pdf'), makeFile('b.pdf', 'application/pdf')]);

    const next: PendingEvidence[] = onChange.mock.calls[0][0];
    expect(next).toHaveLength(10);
  });

  it('removes a queued file and revokes its object URL', () => {
    const files: PendingEvidence[] = [
      {
        id: 'ev-1',
        file: makeFile('photo.jpg', 'image/jpeg'),
        description: '',
        preview: 'blob:mock-url',
        status: 'pending',
      },
    ];

    render(<EvidenceUploader files={files} onChange={onChange} />);

    fireEvent.click(screen.getByRole('button', { name: /remove photo\.jpg/i }));

    expect(mockRevokeObjectURL).toHaveBeenCalledWith('blob:mock-url');
    expect(onChange).toHaveBeenCalledWith([]);
  });

  it('edits a per-file description through onChange', () => {
    const files: PendingEvidence[] = [
      {
        id: 'ev-1',
        file: makeFile('clip.mp4', 'video/mp4'),
        description: '',
        status: 'pending',
      },
    ];

    render(<EvidenceUploader files={files} onChange={onChange} />);

    fireEvent.change(screen.getByLabelText(/description for clip\.mp4/i), {
      target: { value: 'noise recording' },
    });

    const next: PendingEvidence[] = onChange.mock.calls[0][0];
    expect(next[0].description).toBe('noise recording');
  });

  it('shows the error message for an errored file and omits its description input', () => {
    const files: PendingEvidence[] = [
      {
        id: 'ev-1',
        file: makeFile('bad.exe', 'application/x-msdownload'),
        description: '',
        status: 'error',
        error: 'Unsupported file type: application/x-msdownload',
      },
    ];

    render(<EvidenceUploader files={files} onChange={onChange} />);

    expect(screen.getByText(/unsupported file type/i)).toBeInTheDocument();
    expect(screen.queryByLabelText(/description for bad\.exe/i)).not.toBeInTheDocument();
  });

  it('renders an uploading spinner and a disabled remove button mid-upload', () => {
    const files: PendingEvidence[] = [
      {
        id: 'ev-1',
        file: makeFile('doc.pdf', 'application/pdf'),
        description: '',
        status: 'uploading',
      },
    ];

    render(<EvidenceUploader files={files} onChange={onChange} />);

    expect(screen.getByLabelText(/uploading/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /remove doc\.pdf/i })).toBeDisabled();
  });
});
