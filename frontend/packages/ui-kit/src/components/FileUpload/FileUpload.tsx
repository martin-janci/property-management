/**
 * FileUpload — drag-and-drop dropzone with per-file upload queue.
 *
 * Features:
 * - Drag active visual state
 * - Click-to-pick fallback via hidden <input type="file">
 * - Queue list showing file name, size, progress bar, status
 * - Per-file retry button on error state
 * - Per-file remove button
 *
 * Reference: preview/forms/file-upload.html
 *
 * Note: This component is uncontrolled for the queue display.
 * The parent should manage upload state and pass `files` prop.
 * `onFiles` fires when the user picks/drops new files.
 */

import type React from 'react';
import { useRef, useState } from 'react';
import styles from './FileUpload.module.css';

export type FileStatus = 'idle' | 'uploading' | 'done' | 'error';

export interface UploadFile {
  id: string;
  file: File;
  progress: number; // 0–100
  status: FileStatus;
  errorMessage?: string;
}

export interface FileUploadProps {
  /** Called when user selects / drops files. Parent drives upload. */
  onFiles: (files: File[]) => void;
  /** Current file queue displayed under the dropzone. */
  files?: UploadFile[];
  /** Called when user clicks retry on a failed file. */
  onRetry?: (id: string) => void;
  /** Called when user removes a file from queue. */
  onRemove?: (id: string) => void;
  /** Comma-separated MIME types or extensions, e.g. "image/*,.pdf" */
  accept?: string;
  /** Max file size in bytes. Used for hint display only; enforce server-side too. */
  maxSize?: number;
  /** Allow multiple file selection. Default: true. */
  multiple?: boolean;
  className?: string;
}

const UploadIcon = () => (
  <svg
    width="24"
    height="24"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
    <polyline points="17 8 12 3 7 8" />
    <line x1="12" y1="3" x2="12" y2="15" />
  </svg>
);

const FileIcon = () => (
  <svg
    width="16"
    height="16"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
    <polyline points="14 2 14 8 20 8" />
  </svg>
);

const CheckIcon = () => (
  <svg
    width="16"
    height="16"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <polyline points="20 6 9 17 4 12" />
  </svg>
);

const XIcon = () => (
  <svg
    width="14"
    height="14"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <line x1="18" y1="6" x2="6" y2="18" />
    <line x1="6" y1="6" x2="18" y2="18" />
  </svg>
);

const RefreshIcon = () => (
  <svg
    width="14"
    height="14"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <polyline points="1 4 1 10 7 10" />
    <path d="M3.51 15a9 9 0 1 0 .49-3.11" />
  </svg>
);

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

/** FileUpload: drag-and-drop zone with progress queue. */
export const FileUpload: React.FC<FileUploadProps> = ({
  onFiles,
  files = [],
  onRetry,
  onRemove,
  accept,
  maxSize,
  multiple = true,
  className,
}) => {
  const inputRef = useRef<HTMLInputElement>(null);
  const [dragActive, setDragActive] = useState(false);

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    setDragActive(true);
  };

  const handleDragLeave = () => setDragActive(false);

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setDragActive(false);
    const dropped = Array.from(e.dataTransfer.files);
    if (dropped.length > 0) onFiles(dropped);
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const picked = Array.from(e.target.files ?? []);
    if (picked.length > 0) onFiles(picked);
    // Reset so same file can be picked again
    if (inputRef.current) inputRef.current.value = '';
  };

  const sizeHint = maxSize ? ` up to ${formatBytes(maxSize)}` : '';

  return (
    <div className={[className].filter(Boolean).join(' ')}>
      {/* Dropzone */}
      <div
        className={[styles.zone, dragActive ? styles.zoneDragActive : ''].filter(Boolean).join(' ')}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDrop}
        tabIndex={0}
        role="button"
        aria-label="Click to upload or drag and drop files"
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') inputRef.current?.click();
        }}
      >
        <input
          ref={inputRef}
          type="file"
          className={styles.fileInput}
          accept={accept}
          multiple={multiple}
          onChange={handleInputChange}
          aria-hidden="true"
          tabIndex={-1}
        />
        <div className={styles.zoneIcon}>
          <UploadIcon />
        </div>
        <p className={styles.zoneTitle}>
          {dragActive ? (
            'Drop to upload'
          ) : (
            <>
              <strong>Click to upload</strong> or drag and drop
            </>
          )}
        </p>
        {(accept || sizeHint) && (
          <p className={styles.zoneHint}>{[accept, sizeHint].filter(Boolean).join(' · ')}</p>
        )}
      </div>

      {/* File queue */}
      {files.length > 0 && (
        <ul className={styles.queue} aria-label="Upload queue">
          {files.map((f) => (
            <li key={f.id} className={styles.queueItem}>
              <div className={styles.fileIcon}>
                <FileIcon />
              </div>
              <div className={styles.fileInfo}>
                <div className={styles.fileName}>{f.file.name}</div>
                <div className={styles.fileMeta}>
                  {formatBytes(f.file.size)}
                  {f.status === 'uploading' && ` · ${f.progress}%`}
                  {f.status === 'error' && f.errorMessage && ` · ${f.errorMessage}`}
                </div>
                {(f.status === 'uploading' || f.status === 'error') && (
                  <div className={styles.progressWrap}>
                    <div
                      className={[
                        styles.progressBar,
                        f.status === 'error' ? styles.progressBarError : '',
                      ]
                        .filter(Boolean)
                        .join(' ')}
                      style={{ width: `${f.status === 'error' ? 100 : f.progress}%` }}
                      role="progressbar"
                      aria-valuenow={f.progress}
                      aria-valuemin={0}
                      aria-valuemax={100}
                      aria-label={`${f.file.name} upload progress`}
                    />
                  </div>
                )}
              </div>

              {f.status === 'done' && (
                <span className={styles.statusDone} aria-label="Upload complete">
                  <CheckIcon />
                </span>
              )}

              {f.status === 'error' && onRetry && (
                <button
                  className={[styles.actionBtn, styles.retryBtn].join(' ')}
                  onClick={() => onRetry(f.id)}
                  type="button"
                  aria-label={`Retry upload for ${f.file.name}`}
                >
                  <RefreshIcon />
                </button>
              )}

              {onRemove && (
                <button
                  className={[styles.actionBtn, styles.removeBtn].join(' ')}
                  onClick={() => onRemove(f.id)}
                  type="button"
                  aria-label={`Remove ${f.file.name}`}
                >
                  <XIcon />
                </button>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
};

export default FileUpload;
