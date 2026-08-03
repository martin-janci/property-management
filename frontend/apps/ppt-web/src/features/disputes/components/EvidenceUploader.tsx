/**
 * EvidenceUploader component for the dispute filing form (Epic 80, Story 80.2).
 *
 * Supports JPG/PNG/PDF/MP3/MP4 uploads up to 50 MB per file (matching the
 * file-dispute screen-map spec). Shows progress per-file and lets the user
 * remove queued items before form submission.
 */

import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

export interface PendingEvidence {
  /** Unique stable ID generated client-side */
  id: string;
  file: File;
  /** Optional human-readable description for this file */
  description: string;
  /** Preview URL (object URL) – only set for images */
  preview?: string;
  /** Upload lifecycle state */
  status: 'pending' | 'uploading' | 'uploaded' | 'error';
  error?: string;
}

const ACCEPTED_TYPES = [
  'image/jpeg',
  'image/png',
  'image/webp',
  'audio/mpeg',
  'video/mp4',
  'application/pdf',
];

const ACCEPTED_EXTENSIONS = '.jpg,.jpeg,.png,.webp,.mp3,.mp4,.pdf';
const MAX_FILE_SIZE = 50 * 1024 * 1024; // 50 MB
const MAX_FILES = 10;

function generateId() {
  return `ev-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

function isImage(file: File) {
  return file.type.startsWith('image/');
}

interface EvidenceUploaderProps {
  files: PendingEvidence[];
  onChange: (files: PendingEvidence[]) => void;
  disabled?: boolean;
}

export function EvidenceUploader({ files, onChange, disabled = false }: EvidenceUploaderProps) {
  const { t } = useTranslation();
  const inputRef = useRef<HTMLInputElement>(null);
  const [isDragging, setIsDragging] = useState(false);
  // Track all created object URLs so we can revoke them on unmount only.
  // Using a ref avoids re-running the effect when files change.
  const createdPreviewsRef = useRef<Set<string>>(new Set());

  // Revoke all created object URLs only when the component unmounts.
  useEffect(() => {
    return () => {
      for (const url of createdPreviewsRef.current) {
        URL.revokeObjectURL(url);
      }
    };
  }, []);

  const validate = useCallback((file: File): string | null => {
    if (!ACCEPTED_TYPES.includes(file.type)) {
      return `Unsupported file type: ${file.type || file.name}`;
    }
    if (file.size > MAX_FILE_SIZE) {
      return `File too large (max 50 MB): ${file.name}`;
    }
    return null;
  }, []);

  const addFiles = useCallback(
    (incoming: FileList | File[]) => {
      const arr = Array.from(incoming);
      const slots = MAX_FILES - files.length;
      if (slots <= 0) return;

      const next: PendingEvidence[] = [];
      for (const file of arr.slice(0, slots)) {
        const error = validate(file) ?? undefined;
        let preview: string | undefined;
        if (!error && isImage(file)) {
          preview = URL.createObjectURL(file);
          createdPreviewsRef.current.add(preview);
        }
        next.push({
          id: generateId(),
          file,
          description: '',
          preview,
          status: error ? 'error' : 'pending',
          error,
        });
      }
      onChange([...files, ...next]);
    },
    [files, validate, onChange]
  );

  const handleInputChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      if (e.target.files && e.target.files.length > 0) addFiles(e.target.files);
      e.target.value = '';
    },
    [addFiles]
  );

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      setIsDragging(false);
      if (disabled) return;
      if (e.dataTransfer.files.length > 0) addFiles(e.dataTransfer.files);
    },
    [disabled, addFiles]
  );

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    if (!disabled) setIsDragging(true);
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
  };

  const handleRemove = useCallback(
    (id: string) => {
      const item = files.find((f) => f.id === id);
      if (item?.preview) {
        URL.revokeObjectURL(item.preview);
        createdPreviewsRef.current.delete(item.preview);
      }
      onChange(files.filter((f) => f.id !== id));
    },
    [files, onChange]
  );

  const handleDescriptionChange = useCallback(
    (id: string, description: string) => {
      onChange(files.map((f) => (f.id === id ? { ...f, description } : f)));
    },
    [files, onChange]
  );

  const canAddMore = files.length < MAX_FILES;

  return (
    <div className="evidence-uploader space-y-3">
      {/* Hidden file input */}
      <input
        ref={inputRef}
        type="file"
        accept={ACCEPTED_EXTENSIONS}
        multiple
        className="sr-only"
        disabled={disabled}
        onChange={handleInputChange}
        aria-label={t('aria.addEvidenceFiles')}
      />

      {/* Drop zone */}
      {canAddMore && (
        <div
          role="button"
          tabIndex={disabled ? -1 : 0}
          onClick={() => !disabled && inputRef.current?.click()}
          onKeyDown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              if (!disabled) inputRef.current?.click();
            }
          }}
          onDrop={handleDrop}
          onDragOver={handleDragOver}
          onDragLeave={handleDragLeave}
          aria-label={t('aria.dropEvidenceFiles')}
          className={[
            'relative border-2 border-dashed rounded-lg p-6 text-center cursor-pointer',
            'transition-colors duration-200',
            isDragging ? 'border-blue-500 bg-blue-50' : 'border-gray-300 hover:border-gray-400',
            disabled ? 'opacity-50 cursor-not-allowed' : '',
          ]
            .filter(Boolean)
            .join(' ')}
        >
          <div className="flex flex-col items-center gap-2">
            {/* Upload icon */}
            <div className="w-12 h-12 rounded-full bg-gray-100 flex items-center justify-center">
              <svg
                className="w-6 h-6 text-gray-500"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12"
                />
              </svg>
            </div>
            <p className="text-sm font-medium text-gray-700">
              Drop files here or <span className="text-blue-600 underline">browse</span>
            </p>
            <p className="text-xs text-gray-500">
              JPG, PNG, PDF, MP3, MP4 · max 50 MB per file · up to {MAX_FILES} files
            </p>
          </div>
        </div>
      )}

      {/* File list */}
      {files.length > 0 && (
        <ul className="space-y-2" aria-label={t('aria.evidenceFiles')}>
          {files.map((item) => (
            <li
              key={item.id}
              className={[
                'flex gap-3 p-3 rounded-lg border',
                item.status === 'error' ? 'border-red-300 bg-red-50' : 'border-gray-200 bg-white',
              ].join(' ')}
            >
              {/* Thumbnail / icon */}
              <div className="shrink-0 w-12 h-12 rounded overflow-hidden bg-gray-100 flex items-center justify-center">
                {item.preview ? (
                  <img
                    src={item.preview}
                    alt={item.file.name}
                    className="w-full h-full object-cover"
                  />
                ) : (
                  <svg
                    className="w-6 h-6 text-gray-400"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                    aria-hidden="true"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
                    />
                  </svg>
                )}
              </div>

              {/* File info + description input */}
              <div className="flex-1 min-w-0 space-y-1">
                <p className="text-sm font-medium text-gray-800 truncate">{item.file.name}</p>
                <p className="text-xs text-gray-500">
                  {(item.file.size / 1024 / 1024).toFixed(2)} MB
                </p>
                {item.status === 'error' && item.error && (
                  <p className="text-xs text-red-600">{item.error}</p>
                )}
                {item.status !== 'error' && (
                  <input
                    type="text"
                    value={item.description}
                    onChange={(e) => handleDescriptionChange(item.id, e.target.value)}
                    placeholder="Short description (optional)"
                    className="w-full text-sm px-2 py-1 border border-gray-200 rounded focus:outline-none focus:ring-1 focus:ring-blue-500"
                    disabled={disabled || item.status === 'uploading'}
                    maxLength={200}
                    aria-label={`Description for ${item.file.name}`}
                  />
                )}
              </div>

              {/* Status indicator + remove */}
              <div className="shrink-0 flex flex-col items-center gap-2">
                {item.status === 'uploading' && (
                  <div
                    className="w-4 h-4 animate-spin rounded-full border-2 border-blue-600 border-t-transparent"
                    aria-label={t('aria.uploading')}
                  />
                )}
                {item.status === 'uploaded' && (
                  <svg
                    className="w-5 h-5 text-green-500"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                    aria-label={t('aria.uploaded')}
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M5 13l4 4L19 7"
                    />
                  </svg>
                )}
                <button
                  type="button"
                  onClick={() => handleRemove(item.id)}
                  disabled={disabled || item.status === 'uploading'}
                  className="text-gray-400 hover:text-red-500 transition-colors disabled:opacity-50"
                  aria-label={`Remove ${item.file.name}`}
                >
                  <svg
                    className="w-5 h-5"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                    aria-hidden="true"
                  >
                    <path
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      strokeWidth={2}
                      d="M6 18L18 6M6 6l12 12"
                    />
                  </svg>
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
