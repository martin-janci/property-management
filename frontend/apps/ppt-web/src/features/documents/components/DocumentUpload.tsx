/**
 * Document Upload Component (Story 39.2).
 *
 * Upload documents with OCR processing support.
 */

import {
  DOCUMENT_CATEGORIES,
  type DocumentCategory,
  useUploadDocumentDirect,
} from '@ppt/api-client';
import { type ChangeEvent, type DragEvent, useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';

interface DocumentUploadProps {
  organizationId: string;
  buildingId?: string;
  folderId?: string;
  onUploadComplete?: (documentId: string) => void;
  onCancel?: () => void;
}

interface UploadFile {
  file: File;
  progress: number;
  status: 'pending' | 'uploading' | 'processing' | 'completed' | 'error';
  error?: string;
  documentId?: string;
}

const SUPPORTED_MIME_TYPES = [
  'application/pdf',
  'image/png',
  'image/jpeg',
  'image/tiff',
  'image/webp',
  'application/msword',
  'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
  'text/plain',
];

const MAX_FILE_SIZE = 50 * 1024 * 1024; // 50MB

export function DocumentUpload({
  organizationId,
  buildingId,
  folderId,
  onUploadComplete,
  onCancel,
}: DocumentUploadProps) {
  const { t } = useTranslation();
  const [uploadFiles, setUploadFiles] = useState<UploadFile[]>([]);
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [category, setCategory] = useState<DocumentCategory>('other');
  const [isDragOver, setIsDragOver] = useState(false);

  // Direct-to-S3 upload (gap-84-1): bytes go straight to storage via a
  // presigned PUT URL instead of proxying through the api-server.
  const uploadDocument = useUploadDocumentDirect();

  // Note: Client-side validation improves UX, but server-side validation of file
  // signatures (magic bytes) is required for security. The backend validates file types.
  const validateFile = useCallback((file: File): string | null => {
    if (!SUPPORTED_MIME_TYPES.includes(file.type)) {
      return `Unsupported file type: ${file.type || 'unknown'}`;
    }
    if (file.size > MAX_FILE_SIZE) {
      return `File too large: ${(file.size / 1024 / 1024).toFixed(1)}MB (max 50MB)`;
    }
    return null;
  }, []);

  const handleFiles = useCallback(
    (fileList: FileList) => {
      const newFiles: UploadFile[] = Array.from(fileList).map((file) => ({
        file,
        progress: 0,
        status: 'pending' as const,
        error: validateFile(file) || undefined,
      }));

      // Mark files with validation errors
      for (const f of newFiles) {
        if (f.error) {
          f.status = 'error';
        }
      }

      setUploadFiles((prev) => [...prev, ...newFiles]);

      // Auto-set title from first file if not set
      if (!title && newFiles.length > 0 && !newFiles[0].error) {
        const fileName = newFiles[0].file.name;
        const nameWithoutExt = fileName.replace(/\.[^/.]+$/, '');
        setTitle(nameWithoutExt);
      }
    },
    [validateFile, title]
  );

  const handleDrop = useCallback(
    (e: DragEvent<HTMLDivElement>) => {
      e.preventDefault();
      setIsDragOver(false);
      if (e.dataTransfer.files.length > 0) {
        handleFiles(e.dataTransfer.files);
      }
    },
    [handleFiles]
  );

  const handleDragOver = useCallback((e: DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    setIsDragOver(true);
  }, []);

  const handleDragLeave = useCallback((e: DragEvent<HTMLDivElement>) => {
    e.preventDefault();
    setIsDragOver(false);
  }, []);

  const handleFileInput = useCallback(
    (e: ChangeEvent<HTMLInputElement>) => {
      if (e.target.files && e.target.files.length > 0) {
        handleFiles(e.target.files);
      }
    },
    [handleFiles]
  );

  const removeFile = useCallback((index: number) => {
    setUploadFiles((prev) => prev.filter((_, i) => i !== index));
  }, []);

  const handleUpload = useCallback(async () => {
    const validFiles = uploadFiles.filter((f) => f.status === 'pending' && !f.error);
    if (validFiles.length === 0) return;

    // Collect all successfully uploaded document IDs
    const uploadedDocumentIds: string[] = [];

    for (let i = 0; i < validFiles.length; i++) {
      const uploadFile = validFiles[i];
      const fileIndex = uploadFiles.indexOf(uploadFile);

      // Update status to uploading
      setUploadFiles((prev) =>
        prev.map((f, idx) => (idx === fileIndex ? { ...f, status: 'uploading' as const } : f))
      );

      try {
        // For multiple files, append the filename to the title
        const documentTitle =
          validFiles.length === 1 ? title : `${title} - ${uploadFile.file.name}`;

        const result = await uploadDocument.mutateAsync({
          file: uploadFile.file,
          title: documentTitle,
          description,
          category,
          organizationId,
          buildingId,
          folderId,
          onProgress: (progress) => {
            // Keep status as 'uploading' during progress updates.
            // The transition to 'processing' happens only after successful server response.
            setUploadFiles((prev) =>
              prev.map((f, idx) => (idx === fileIndex ? { ...f, progress } : f))
            );
          },
        });

        // Update status to processing first, then completed after server confirms
        setUploadFiles((prev) =>
          prev.map((f, idx) =>
            idx === fileIndex
              ? { ...f, status: 'completed' as const, documentId: result.id, progress: 100 }
              : f
          )
        );

        uploadedDocumentIds.push(result.id);
      } catch (error) {
        // Update status to error
        setUploadFiles((prev) =>
          prev.map((f, idx) =>
            idx === fileIndex
              ? {
                  ...f,
                  status: 'error' as const,
                  error: error instanceof Error ? error.message : 'Upload failed',
                }
              : f
          )
        );
      }
    }

    // Only call onUploadComplete once after all files are processed,
    // passing the first successfully uploaded document ID
    if (uploadedDocumentIds.length > 0) {
      onUploadComplete?.(uploadedDocumentIds[0]);
    }
  }, [
    uploadFiles,
    title,
    description,
    category,
    organizationId,
    buildingId,
    folderId,
    uploadDocument,
    onUploadComplete,
  ]);

  const formatFileSize = (bytes: number): string => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const getFileTypeIcon = (mimeType: string): string => {
    if (mimeType === 'application/pdf') return 'PDF';
    if (mimeType.startsWith('image/')) return 'IMG';
    if (mimeType.includes('word')) return 'DOC';
    if (mimeType === 'text/plain') return 'TXT';
    return 'FILE';
  };

  const canUpload =
    uploadFiles.some((f) => f.status === 'pending' && !f.error) && title.trim().length > 0;
  const isUploading = uploadFiles.some(
    (f) => f.status === 'uploading' || f.status === 'processing'
  );

  return (
    <div className="document-upload">
      <div className="upload-header">
        <h2 className="upload-title">Upload Document</h2>
        <p className="upload-subtitle">
          Upload documents for automatic text extraction (OCR) and AI analysis
        </p>
      </div>

      {/* Drop Zone */}
      <div
        className={`drop-zone ${isDragOver ? 'drag-over' : ''}`}
        onDrop={handleDrop}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
      >
        <div className="drop-zone-content">
          <svg
            className="drop-icon"
            width="48"
            height="48"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            aria-hidden="true"
          >
            <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" />
            <polyline points="17 8 12 3 7 8" />
            <line x1="12" y1="3" x2="12" y2="15" />
          </svg>
          <p className="drop-text">
            <span className="drop-primary">Drop files here</span>
            <span className="drop-secondary"> or click to browse</span>
          </p>
          <p className="drop-hint">
            Supports PDF, images, Word documents, and text files (max 50MB)
          </p>
          <input
            type="file"
            className="file-input"
            onChange={handleFileInput}
            multiple
            accept={SUPPORTED_MIME_TYPES.join(',')}
            aria-label={t('aria.selectFilesToUpload')}
          />
        </div>
      </div>

      {/* File List */}
      {uploadFiles.length > 0 && (
        <div className="file-list">
          <h3 className="file-list-title">Selected Files ({uploadFiles.length})</h3>
          <ul className="files">
            {uploadFiles.map((uploadFile, index) => (
              <li
                key={`${uploadFile.file.name}-${index}`}
                className={`file-item ${uploadFile.status}`}
              >
                <div className="file-icon">{getFileTypeIcon(uploadFile.file.type)}</div>
                <div className="file-info">
                  <span className="file-name">{uploadFile.file.name}</span>
                  <span className="file-size">{formatFileSize(uploadFile.file.size)}</span>
                  {uploadFile.error && <span className="file-error">{uploadFile.error}</span>}
                  {uploadFile.status === 'uploading' && (
                    <div className="progress-bar">
                      <div className="progress-fill" style={{ width: `${uploadFile.progress}%` }} />
                    </div>
                  )}
                  {uploadFile.status === 'processing' && (
                    <span className="processing-text">Processing OCR...</span>
                  )}
                  {uploadFile.status === 'completed' && (
                    <span className="completed-text">Uploaded successfully</span>
                  )}
                </div>
                <div className="file-actions">
                  {uploadFile.status === 'pending' && (
                    <button
                      type="button"
                      onClick={() => removeFile(index)}
                      className="remove-btn"
                      aria-label={`Remove ${uploadFile.file.name}`}
                    >
                      <svg
                        width="12"
                        height="12"
                        viewBox="0 0 12 12"
                        aria-hidden="true"
                        focusable="false"
                      >
                        <path
                          d="M9.5 2.5L2.5 9.5M2.5 2.5L9.5 9.5"
                          stroke="currentColor"
                          strokeWidth="1.5"
                          strokeLinecap="round"
                        />
                      </svg>
                    </button>
                  )}
                  {uploadFile.status === 'completed' && (
                    <span
                      className="success-icon"
                      role="img"
                      aria-label={t('aria.uploadCompleted')}
                    >
                      <svg
                        width="16"
                        height="16"
                        viewBox="0 0 16 16"
                        aria-hidden="true"
                        focusable="false"
                      >
                        <path
                          d="M6.00016 11.2002L3.30016 8.50016L2.60016 9.20016L6.00016 12.6002L14.0002 4.60016L13.3002 3.90016L6.00016 11.2002Z"
                          fill="currentColor"
                        />
                      </svg>
                    </span>
                  )}
                </div>
              </li>
            ))}
          </ul>
        </div>
      )}

      {/* Document Metadata */}
      {uploadFiles.some((f) => f.status === 'pending' && !f.error) && (
        <div className="metadata-form">
          <h3 className="form-title">Document Details</h3>

          <div className="form-field">
            <label htmlFor="doc-title" className="field-label">
              Title <span className="required">*</span>
            </label>
            <input
              id="doc-title"
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="Enter document title"
              className="text-input"
              required
            />
          </div>

          <div className="form-field">
            <label htmlFor="doc-description" className="field-label">
              Description
            </label>
            <textarea
              id="doc-description"
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Optional description"
              className="textarea-input"
              rows={3}
            />
          </div>

          <div className="form-field">
            <label htmlFor="doc-category" className="field-label">
              Category
            </label>
            <select
              id="doc-category"
              value={category}
              onChange={(e) => setCategory(e.target.value as DocumentCategory)}
              className="select-input"
            >
              {DOCUMENT_CATEGORIES.map((cat) => (
                <option key={cat} value={cat}>
                  {cat.charAt(0).toUpperCase() + cat.slice(1)}
                </option>
              ))}
            </select>
            <p className="field-hint">AI will suggest a category after analysis</p>
          </div>
        </div>
      )}

      {/* OCR Info */}
      <div className="ocr-info">
        <div className="info-icon">
          <span className="ai-badge">AI</span>
        </div>
        <div className="info-content">
          <h4 className="info-title">Automatic Text Extraction</h4>
          <p className="info-text">
            After upload, OCR will automatically extract text from your documents. This enables
            full-text search, AI classification, and summarization.
          </p>
        </div>
      </div>

      {/* Actions */}
      <div className="upload-actions">
        {onCancel && (
          <button
            type="button"
            onClick={onCancel}
            className="btn btn-secondary"
            disabled={isUploading}
          >
            Cancel
          </button>
        )}
        <button
          type="button"
          onClick={handleUpload}
          disabled={!canUpload || isUploading}
          className="btn btn-primary"
        >
          {isUploading ? (
            <>
              <span className="spinner" />
              Uploading...
            </>
          ) : (
            <>
              <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
                aria-hidden="true"
              >
                <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" />
                <polyline points="17 8 12 3 7 8" />
                <line x1="12" y1="3" x2="12" y2="15" />
              </svg>
              Upload & Process
            </>
          )}
        </button>
      </div>

      <style>{`
        .document-upload {
          display: flex;
          flex-direction: column;
          gap: 1.5rem;
          max-width: 600px;
          margin: 0 auto;
        }

        .upload-header {
          text-align: center;
        }

        .upload-title {
          margin: 0 0 0.5rem;
          font-size: 1.5rem;
          font-weight: 600;
          color: var(--ppt-fg-primary);
        }

        .upload-subtitle {
          margin: 0;
          color: var(--ppt-fg-muted);
        }

        .drop-zone {
          position: relative;
          padding: 2rem;
          border: 2px dashed var(--ppt-border-strong);
          border-radius: 0.75rem;
          background: var(--ppt-bg-app);
          text-align: center;
          cursor: pointer;
          transition: all 0.2s;
        }

        .drop-zone:hover,
        .drop-zone:focus-within {
          border-color: var(--ppt-brand-500);
          background: var(--ppt-color-primary-soft-bg);
        }

        .drop-zone.drag-over {
          border-color: var(--ppt-brand-500);
          background: var(--ppt-color-primary-soft-bg);
          transform: scale(1.02);
        }

        .drop-zone-content {
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: 0.75rem;
        }

        .drop-icon {
          color: var(--ppt-fg-subtle);
        }

        .drop-zone:hover .drop-icon,
        .drop-zone.drag-over .drop-icon {
          color: var(--ppt-brand-500);
        }

        .drop-text {
          margin: 0;
          font-size: 1rem;
        }

        .drop-primary {
          font-weight: 600;
          color: var(--ppt-fg-primary);
        }

        .drop-secondary {
          color: var(--ppt-fg-muted);
        }

        .drop-hint {
          margin: 0;
          font-size: 0.875rem;
          color: var(--ppt-fg-subtle);
        }

        .file-input {
          position: absolute;
          inset: 0;
          width: 100%;
          height: 100%;
          opacity: 0;
          cursor: pointer;
        }

        .file-list {
          border: 1px solid var(--ppt-border-default);
          border-radius: 0.5rem;
          overflow: hidden;
        }

        .file-list-title {
          margin: 0;
          padding: 0.75rem 1rem;
          font-size: 0.875rem;
          font-weight: 600;
          background: var(--ppt-bg-app);
          color: var(--ppt-fg-primary);
          border-bottom: 1px solid var(--ppt-border-default);
        }

        .files {
          list-style: none;
          margin: 0;
          padding: 0;
        }

        .file-item {
          display: flex;
          align-items: center;
          gap: 0.75rem;
          padding: 0.75rem 1rem;
          border-bottom: 1px solid var(--ppt-border-default);
        }

        .file-item:last-child {
          border-bottom: none;
        }

        .file-item.error {
          background: var(--ppt-color-danger-light);
        }

        .file-item.completed {
          background: var(--ppt-color-success-light);
        }

        .file-icon {
          display: flex;
          align-items: center;
          justify-content: center;
          width: 40px;
          height: 40px;
          font-size: 0.625rem;
          font-weight: 700;
          background: var(--ppt-border-default);
          color: var(--ppt-fg-secondary);
          border-radius: 0.375rem;
        }

        .file-info {
          flex: 1;
          min-width: 0;
        }

        .file-name {
          display: block;
          font-size: 0.875rem;
          font-weight: 500;
          color: var(--ppt-fg-primary);
          white-space: nowrap;
          overflow: hidden;
          text-overflow: ellipsis;
        }

        .file-size {
          font-size: 0.75rem;
          color: var(--ppt-fg-muted);
        }

        .file-error {
          display: block;
          font-size: 0.75rem;
          color: var(--ppt-color-danger);
        }

        .progress-bar {
          margin-top: 0.5rem;
          height: 4px;
          background: var(--ppt-border-default);
          border-radius: 2px;
          overflow: hidden;
        }

        .progress-fill {
          height: 100%;
          background: var(--ppt-brand-500);
          transition: width 0.3s;
        }

        .processing-text {
          display: block;
          font-size: 0.75rem;
          color: var(--ppt-color-primary);
        }

        .completed-text {
          display: block;
          font-size: 0.75rem;
          color: var(--ppt-color-success);
        }

        .file-actions {
          flex-shrink: 0;
        }

        .remove-btn {
          display: flex;
          align-items: center;
          justify-content: center;
          width: 24px;
          height: 24px;
          font-size: 1rem;
          background: transparent;
          border: 1px solid var(--ppt-border-default);
          border-radius: 0.25rem;
          color: var(--ppt-fg-muted);
          cursor: pointer;
          transition: all 0.15s;
        }

        .remove-btn:hover {
          background: var(--ppt-color-danger-light);
          border-color: var(--ppt-color-danger-light);
          color: var(--ppt-color-danger);
        }

        .success-icon {
          display: flex;
          align-items: center;
          justify-content: center;
          width: 24px;
          height: 24px;
          font-size: 0.625rem;
          font-weight: 700;
          background: var(--ppt-color-success-light);
          color: var(--ppt-color-success);
          border-radius: 0.25rem;
        }

        .metadata-form {
          padding: 1.5rem;
          background: var(--ppt-bg-app);
          border-radius: 0.5rem;
        }

        .form-title {
          margin: 0 0 1rem;
          font-size: 1rem;
          font-weight: 600;
          color: var(--ppt-fg-primary);
        }

        .form-field {
          margin-bottom: 1rem;
        }

        .form-field:last-child {
          margin-bottom: 0;
        }

        .field-label {
          display: block;
          margin-bottom: 0.375rem;
          font-size: 0.875rem;
          font-weight: 500;
          color: var(--ppt-fg-secondary);
        }

        .required {
          color: var(--ppt-color-danger);
        }

        .text-input,
        .textarea-input,
        .select-input {
          width: 100%;
          padding: 0.5rem 0.75rem;
          font-size: 0.875rem;
          border: 1px solid var(--ppt-border-default);
          border-radius: 0.375rem;
          background: var(--ppt-bg-surface);
          transition: border-color 0.15s;
        }

        .text-input:focus,
        .textarea-input:focus,
        .select-input:focus {
          outline: none;
          border-color: var(--ppt-brand-500);
          box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
        }

        .textarea-input {
          resize: vertical;
          min-height: 80px;
        }

        .field-hint {
          margin: 0.375rem 0 0;
          font-size: 0.75rem;
          color: var(--ppt-fg-muted);
        }

        .ocr-info {
          display: flex;
          gap: 1rem;
          padding: 1rem;
          background: linear-gradient(135deg, #ede9fe 0%, #ddd6fe 100%);
          border: 1px solid #c4b5fd;
          border-radius: 0.5rem;
        }

        .info-icon {
          flex-shrink: 0;
        }

        .ai-badge {
          display: inline-flex;
          align-items: center;
          justify-content: center;
          padding: 0.25rem 0.5rem;
          font-size: 0.625rem;
          font-weight: 700;
          background: #7c3aed;
          color: var(--ppt-fg-on-accent);
          border-radius: 0.25rem;
        }

        .info-content {
          flex: 1;
        }

        .info-title {
          margin: 0 0 0.25rem;
          font-size: 0.875rem;
          font-weight: 600;
          color: #5b21b6;
        }

        .info-text {
          margin: 0;
          font-size: 0.875rem;
          color: #6d28d9;
          line-height: 1.5;
        }

        .upload-actions {
          display: flex;
          justify-content: flex-end;
          gap: 0.75rem;
          padding-top: 0.5rem;
        }

        .btn {
          display: inline-flex;
          align-items: center;
          justify-content: center;
          gap: 0.5rem;
          padding: 0.625rem 1.25rem;
          font-size: 0.875rem;
          font-weight: 600;
          border: none;
          border-radius: 0.375rem;
          cursor: pointer;
          transition: all 0.15s;
        }

        .btn:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }

        .btn-primary {
          background: var(--ppt-brand-500);
          color: var(--ppt-fg-on-accent);
        }

        .btn-primary:hover:not(:disabled) {
          background: var(--ppt-color-primary);
        }

        .btn-secondary {
          background: var(--ppt-border-default);
          color: var(--ppt-fg-secondary);
        }

        .btn-secondary:hover:not(:disabled) {
          background: var(--ppt-border-strong);
        }

        .spinner {
          width: 16px;
          height: 16px;
          border: 2px solid rgba(255, 255, 255, 0.3);
          border-top-color: var(--ppt-fg-on-accent);
          border-radius: 50%;
          animation: spin 0.6s linear infinite;
        }

        @keyframes spin {
          to { transform: rotate(360deg); }
        }
      `}</style>
    </div>
  );
}
