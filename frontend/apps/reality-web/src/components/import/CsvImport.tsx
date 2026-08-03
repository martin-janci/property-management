/**
 * CsvImport Component
 *
 * Bulk import listings from CSV (Epic 46, Story 46.1).
 */

'use client';

import type { ColumnMapping, CsvImportPreview, CsvValidationError } from '@ppt/reality-api-client';
import { useCsvImport, useCsvPreview, useMyAgency } from '@ppt/reality-api-client';
import { useTranslations } from 'next-intl';
import { useCallback, useState } from 'react';

const REQUIRED_FIELDS = ['title', 'propertyType', 'transactionType', 'price'];
const OPTIONAL_FIELDS = [
  'description',
  'currency',
  'address',
  'city',
  'postalCode',
  'rooms',
  'bathrooms',
  'size',
  'yearBuilt',
  'features',
  'photos',
];

type ImportStep = 'upload' | 'mapping' | 'preview' | 'importing' | 'complete';

export function CsvImport() {
  const t = useTranslations('import');
  const [step, setStep] = useState<ImportStep>('upload');
  const [file, setFile] = useState<File | null>(null);
  const [mapping, setMapping] = useState<Partial<ColumnMapping>>({});
  const [skipInvalid, setSkipInvalid] = useState(true);
  const [isDragging, setIsDragging] = useState(false);

  const { data: agency } = useMyAgency();
  const {
    data: preview,
    isLoading: isPreviewLoading,
    error: previewError,
  } = useCsvPreview(agency?.id || '', file);
  const importMutation = useCsvImport(agency?.id || '');

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);

    const droppedFile = e.dataTransfer.files[0];
    if (droppedFile?.type === 'text/csv' || droppedFile?.name.endsWith('.csv')) {
      setFile(droppedFile);
      setStep('mapping');
    }
  }, []);

  const handleFileSelect = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const selectedFile = e.target.files?.[0];
    if (selectedFile) {
      setFile(selectedFile);
      setStep('mapping');
    }
  }, []);

  const handleMappingChange = (field: keyof ColumnMapping, value: string) => {
    setMapping((prev) => ({ ...prev, [field]: value }));
  };

  const isMappingComplete = REQUIRED_FIELDS.every((field) => mapping[field as keyof ColumnMapping]);

  const handleStartImport = async () => {
    if (!file || !isMappingComplete) return;
    setStep('importing');

    try {
      await importMutation.mutateAsync({
        file,
        mapping: mapping as ColumnMapping,
        skipInvalid,
      });
      setStep('complete');
    } catch {
      setStep('preview');
    }
  };

  const handleReset = () => {
    setFile(null);
    setMapping({});
    setStep('upload');
    importMutation.reset();
  };

  return (
    <div className="csv-import">
      {/* Progress Steps */}
      <div className="steps">
        <Step
          number={1}
          label={t('steps.upload')}
          active={step === 'upload'}
          complete={step !== 'upload'}
        />
        <StepConnector complete={step !== 'upload'} />
        <Step
          number={2}
          label={t('steps.mapColumns')}
          active={step === 'mapping'}
          complete={['preview', 'importing', 'complete'].includes(step)}
        />
        <StepConnector complete={['preview', 'importing', 'complete'].includes(step)} />
        <Step
          number={3}
          label={t('steps.preview')}
          active={step === 'preview'}
          complete={['importing', 'complete'].includes(step)}
        />
        <StepConnector complete={['importing', 'complete'].includes(step)} />
        <Step
          number={4}
          label={t('steps.import')}
          active={step === 'importing' || step === 'complete'}
          complete={step === 'complete'}
        />
      </div>

      {/* Step Content */}
      <div className="step-content">
        {step === 'upload' && (
          <UploadStep
            isDragging={isDragging}
            onDragEnter={() => setIsDragging(true)}
            onDragLeave={() => setIsDragging(false)}
            onDrop={handleDrop}
            onFileSelect={handleFileSelect}
          />
        )}

        {step === 'mapping' && preview && (
          <MappingStep
            preview={preview}
            mapping={mapping}
            onMappingChange={handleMappingChange}
            isComplete={isMappingComplete}
            onNext={() => setStep('preview')}
            onBack={handleReset}
            isLoading={isPreviewLoading}
          />
        )}

        {step === 'mapping' && isPreviewLoading && <LoadingState message={t('csv.analyzing')} />}
        {step === 'mapping' && previewError && (
          <ErrorState message={t('csv.parseFailed')} onRetry={handleReset} />
        )}

        {step === 'preview' && preview && (
          <PreviewStep
            preview={preview}
            mapping={mapping as ColumnMapping}
            skipInvalid={skipInvalid}
            onSkipInvalidChange={setSkipInvalid}
            onStartImport={handleStartImport}
            onBack={() => setStep('mapping')}
          />
        )}

        {step === 'importing' && <ImportingState />}

        {step === 'complete' && importMutation.data && (
          <CompleteStep result={importMutation.data} onNewImport={handleReset} />
        )}

        {importMutation.error && (
          <ErrorState message={t('csv.importFailed')} onRetry={() => setStep('preview')} />
        )}
      </div>

      <style jsx>{`
        .csv-import {
          padding: 24px;
          max-width: 900px;
          margin: 0 auto;
        }

        .steps {
          display: flex;
          align-items: center;
          justify-content: center;
          margin-bottom: 32px;
        }

        .step-content {
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-default);
          border-radius: 12px;
          padding: 32px;
        }
      `}</style>
    </div>
  );
}

function Step({
  number,
  label,
  active,
  complete,
}: {
  number: number;
  label: string;
  active: boolean;
  complete: boolean;
}) {
  return (
    <div className={`step ${active ? 'active' : ''} ${complete ? 'complete' : ''}`}>
      <div className="step-number">
        {complete ? (
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="3"
            aria-hidden="true"
          >
            <polyline points="20 6 9 17 4 12" />
          </svg>
        ) : (
          number
        )}
      </div>
      <span className="step-label">{label}</span>

      <style jsx>{`
        .step {
          display: flex;
          align-items: center;
          gap: 8px;
        }

        .step-number {
          width: 32px;
          height: 32px;
          border-radius: 50%;
          display: flex;
          align-items: center;
          justify-content: center;
          font-weight: 600;
          font-size: 14px;
          background: var(--ppt-border-default);
          color: var(--ppt-fg-muted);
        }

        .step.active .step-number {
          background: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent);
        }

        .step.complete .step-number {
          background: var(--ppt-color-success);
          color: var(--ppt-fg-on-accent);
        }

        .step-label {
          font-size: 14px;
          color: var(--ppt-fg-muted);
        }

        .step.active .step-label {
          color: var(--ppt-fg-primary);
          font-weight: 500;
        }
      `}</style>
    </div>
  );
}

function StepConnector({ complete }: { complete: boolean }) {
  return (
    <div className={`connector ${complete ? 'complete' : ''}`}>
      <style jsx>{`
        .connector {
          width: 48px;
          height: 2px;
          background: var(--ppt-border-default);
          margin: 0 8px;
        }

        .connector.complete {
          background: var(--ppt-color-success);
        }
      `}</style>
    </div>
  );
}

function UploadStep({
  isDragging,
  onDragEnter,
  onDragLeave,
  onDrop,
  onFileSelect,
}: {
  isDragging: boolean;
  onDragEnter: () => void;
  onDragLeave: () => void;
  onDrop: (e: React.DragEvent) => void;
  onFileSelect: (e: React.ChangeEvent<HTMLInputElement>) => void;
}) {
  const t = useTranslations('import.csv');
  return (
    <div
      className={`upload-zone ${isDragging ? 'dragging' : ''}`}
      onDragEnter={onDragEnter}
      onDragLeave={onDragLeave}
      onDragOver={(e) => e.preventDefault()}
      onDrop={onDrop}
    >
      <svg
        width="64"
        height="64"
        viewBox="0 0 24 24"
        fill="none"
        stroke="var(--ppt-fg-subtle)"
        strokeWidth="1.5"
        aria-hidden="true"
      >
        <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
        <polyline points="17 8 12 3 7 8" />
        <line x1="12" y1="3" x2="12" y2="15" />
      </svg>
      <h3>{t('uploadTitle')}</h3>
      <p>{t('uploadHint')}</p>
      <label className="browse-button">
        {t('browseFiles')}
        <input type="file" accept=".csv,text/csv" onChange={onFileSelect} hidden />
      </label>
      <p className="hint">{t('supportedFormat')}</p>

      <style jsx>{`
        .upload-zone {
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
          padding: 64px 24px;
          border: 2px dashed var(--ppt-border-strong);
          border-radius: 12px;
          text-align: center;
          transition: all 0.2s;
        }

        .upload-zone.dragging {
          border-color: var(--ppt-color-primary);
          background: var(--ppt-color-primary-soft-bg);
        }

        h3 {
          font-size: 1.25rem;
          color: var(--ppt-fg-primary);
          margin: 24px 0 8px;
        }

        p {
          color: var(--ppt-fg-muted);
          margin: 0 0 16px;
        }

        .browse-button {
          padding: 12px 24px;
          background: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent);
          border-radius: 8px;
          font-weight: 500;
          cursor: pointer;
          transition: background 0.2s;
        }

        .browse-button:hover {
          background: var(--ppt-color-primary-hover);
        }

        .hint {
          margin-top: 24px;
          font-size: 13px;
          color: var(--ppt-fg-subtle);
        }
      `}</style>
    </div>
  );
}

function MappingStep({
  preview,
  mapping,
  onMappingChange,
  isComplete,
  onNext,
  onBack,
  isLoading,
}: {
  preview: CsvImportPreview;
  mapping: Partial<ColumnMapping>;
  onMappingChange: (field: keyof ColumnMapping, value: string) => void;
  isComplete: boolean;
  onNext: () => void;
  onBack: () => void;
  isLoading: boolean;
}) {
  const t = useTranslations('import.csv');
  const allFields = [...REQUIRED_FIELDS, ...OPTIONAL_FIELDS];

  return (
    <div className="mapping-step">
      <h2>{t('mapTitle')}</h2>
      <p className="subtitle">{t('mapSubtitle')}</p>

      <div className="mapping-info">
        <span className="file-info">
          <strong>{preview.totalRows}</strong> {t('rowsFoundSuffix')}
        </span>
        {preview.errors.length > 0 && (
          <span className="error-count">
            {t('validationIssues', { count: preview.errors.length })}
          </span>
        )}
      </div>

      <div className="mapping-grid">
        {allFields.map((field) => (
          <div key={field} className="mapping-row">
            <label htmlFor={`map-${field}`} className="field-label">
              {formatFieldName(field)}
              {REQUIRED_FIELDS.includes(field) && <span className="required">*</span>}
            </label>
            <select
              id={`map-${field}`}
              value={mapping[field as keyof ColumnMapping] || ''}
              onChange={(e) => onMappingChange(field as keyof ColumnMapping, e.target.value)}
              disabled={isLoading}
            >
              <option value="">{t('selectColumn')}</option>
              {preview.headers.map((header) => (
                <option key={header} value={header}>
                  {header}
                </option>
              ))}
            </select>
          </div>
        ))}
      </div>

      <div className="actions">
        <button type="button" className="secondary" onClick={onBack}>
          {t('back')}
        </button>
        <button type="button" className="primary" onClick={onNext} disabled={!isComplete}>
          {t('previewImport')}
        </button>
      </div>

      <style jsx>{`
        .mapping-step h2 {
          font-size: 1.5rem;
          color: var(--ppt-fg-primary);
          margin: 0 0 8px;
        }

        .subtitle {
          color: var(--ppt-fg-muted);
          margin: 0 0 24px;
        }

        .mapping-info {
          display: flex;
          gap: 16px;
          margin-bottom: 24px;
          padding: 12px 16px;
          background: var(--ppt-bg-app);
          border-radius: 8px;
        }

        .file-info {
          font-size: 14px;
          color: var(--ppt-fg-secondary);
        }

        .error-count {
          font-size: 14px;
          color: var(--ppt-color-danger-hover);
        }

        .mapping-grid {
          display: grid;
          grid-template-columns: repeat(2, 1fr);
          gap: 16px;
          margin-bottom: 24px;
        }

        .mapping-row {
          display: flex;
          flex-direction: column;
          gap: 6px;
        }

        .field-label {
          font-size: 13px;
          font-weight: 500;
          color: var(--ppt-fg-secondary);
        }

        .required {
          color: var(--ppt-color-danger-hover);
          margin-left: 2px;
        }

        select {
          padding: 10px 12px;
          border: 1px solid var(--ppt-border-strong);
          border-radius: 8px;
          font-size: 14px;
          background: var(--ppt-bg-surface);
        }

        select:focus-visible {
          outline: var(--ppt-focus-ring-width) solid var(--ppt-focus-ring-color);
          outline-offset: var(--ppt-focus-ring-offset);
          border-color: var(--ppt-color-primary);
        }

        .actions {
          display: flex;
          justify-content: space-between;
          padding-top: 24px;
          border-top: 1px solid var(--ppt-border-default);
        }

        button {
          padding: 12px 24px;
          border-radius: 8px;
          font-size: 14px;
          font-weight: 500;
          cursor: pointer;
          transition: all 0.2s;
        }

        .secondary {
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-strong);
          color: var(--ppt-fg-secondary);
        }

        .secondary:hover {
          background: var(--ppt-bg-app);
        }

        .primary {
          background: var(--ppt-color-primary);
          border: none;
          color: var(--ppt-fg-on-accent);
        }

        .primary:hover:not(:disabled) {
          background: var(--ppt-color-primary-hover);
        }

        .primary:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }

        @media (max-width: 640px) {
          .mapping-grid {
            grid-template-columns: 1fr;
          }
        }
      `}</style>
    </div>
  );
}

function PreviewStep({
  preview,
  mapping,
  skipInvalid,
  onSkipInvalidChange,
  onStartImport,
  onBack,
}: {
  preview: CsvImportPreview;
  mapping: ColumnMapping;
  skipInvalid: boolean;
  onSkipInvalidChange: (value: boolean) => void;
  onStartImport: () => void;
  onBack: () => void;
}) {
  const t = useTranslations('import.csv');
  return (
    <div className="preview-step">
      <h2>{t('previewTitle')}</h2>
      <p className="subtitle">{t('previewSubtitle')}</p>

      <div className="summary-cards">
        <div className="summary-card">
          <span className="value">{preview.totalRows}</span>
          <span className="label">{t('totalRows')}</span>
        </div>
        <div className="summary-card success">
          <span className="value">{preview.validRows}</span>
          <span className="label">{t('valid')}</span>
        </div>
        <div className="summary-card error">
          <span className="value">{preview.invalidRows}</span>
          <span className="label">{t('invalid')}</span>
        </div>
      </div>

      {/* Sample Preview */}
      {preview.sampleData.length > 0 && (
        <div className="sample-section">
          <h3>{t('sampleTitle')}</h3>
          <div className="sample-table-container">
            <table className="sample-table">
              <thead>
                <tr>
                  {Object.keys(mapping)
                    .filter((k) => mapping[k as keyof ColumnMapping])
                    .map((field) => (
                      <th key={field}>{formatFieldName(field)}</th>
                    ))}
                </tr>
              </thead>
              <tbody>
                {preview.sampleData.slice(0, 3).map((row, i) => (
                  <tr key={`row-${i}-${Object.values(row).join('-')}`}>
                    {Object.entries(mapping)
                      .filter(([, csvCol]) => csvCol)
                      .map(([field, csvCol]) => (
                        <td key={`${field}-${csvCol}`}>{row[csvCol as string] || '-'}</td>
                      ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {/* Validation Errors */}
      {preview.errors.length > 0 && (
        <div className="errors-section">
          <h3>{t('validationTitle', { count: preview.errors.length })}</h3>
          <div className="errors-list">
            {preview.errors.slice(0, 10).map((error: CsvValidationError, i) => (
              <div
                key={`err-${error.row}-${error.column}-${i}`}
                className={`error-item ${error.severity}`}
              >
                <span className="error-location">
                  {t('errorLocation', { row: error.row, column: error.column })}
                </span>
                <span className="error-message">{error.message}</span>
              </div>
            ))}
            {preview.errors.length > 10 && (
              <p className="more-errors">
                {t('moreIssues', { count: preview.errors.length - 10 })}
              </p>
            )}
          </div>
        </div>
      )}

      {/* Skip Invalid Option */}
      {preview.invalidRows > 0 && (
        <label className="skip-option">
          <input
            type="checkbox"
            checked={skipInvalid}
            onChange={(e) => onSkipInvalidChange(e.target.checked)}
          />
          <span>{t('skipInvalid', { count: preview.validRows })}</span>
        </label>
      )}

      <div className="actions">
        <button type="button" className="secondary" onClick={onBack}>
          {t('backToMapping')}
        </button>
        <button
          type="button"
          className="primary"
          onClick={onStartImport}
          disabled={preview.validRows === 0}
        >
          {t('importListings', { count: skipInvalid ? preview.validRows : preview.totalRows })}
        </button>
      </div>

      <style jsx>{`
        .preview-step h2 {
          font-size: 1.5rem;
          color: var(--ppt-fg-primary);
          margin: 0 0 8px;
        }

        .subtitle {
          color: var(--ppt-fg-muted);
          margin: 0 0 24px;
        }

        .summary-cards {
          display: grid;
          grid-template-columns: repeat(3, 1fr);
          gap: 16px;
          margin-bottom: 24px;
        }

        .summary-card {
          padding: 20px;
          background: var(--ppt-bg-app);
          border-radius: 12px;
          text-align: center;
        }

        .summary-card .value {
          display: block;
          font-size: 2rem;
          font-weight: bold;
          color: var(--ppt-fg-primary);
        }

        .summary-card .label {
          font-size: 14px;
          color: var(--ppt-fg-muted);
        }

        .summary-card.success {
          background: var(--ppt-color-success-light);
        }

        .summary-card.success .value {
          color: var(--ppt-color-success-hover);
        }

        .summary-card.error {
          background: var(--ppt-color-danger-light);
        }

        .summary-card.error .value {
          color: var(--ppt-color-danger-hover);
        }

        .sample-section,
        .errors-section {
          margin-bottom: 24px;
        }

        h3 {
          font-size: 1rem;
          color: var(--ppt-fg-primary);
          margin: 0 0 12px;
        }

        .sample-table-container {
          overflow-x: auto;
          border: 1px solid var(--ppt-border-default);
          border-radius: 8px;
        }

        .sample-table {
          width: 100%;
          border-collapse: collapse;
          font-size: 13px;
        }

        .sample-table th,
        .sample-table td {
          padding: 10px 12px;
          text-align: left;
          border-bottom: 1px solid var(--ppt-border-default);
        }

        .sample-table th {
          background: var(--ppt-bg-app);
          font-weight: 600;
          color: var(--ppt-fg-secondary);
        }

        .sample-table td {
          color: var(--ppt-fg-muted);
          max-width: 200px;
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
        }

        .errors-list {
          border: 1px solid var(--ppt-color-danger-light);
          border-radius: 8px;
          background: var(--ppt-color-danger-light);
        }

        .error-item {
          padding: 10px 12px;
          border-bottom: 1px solid var(--ppt-color-danger-light);
          font-size: 13px;
        }

        .error-item:last-child {
          border-bottom: none;
        }

        .error-item.warning {
          background: var(--ppt-color-warning-light);
          border-color: var(--ppt-color-warning-light);
        }

        .error-location {
          font-weight: 500;
          color: var(--ppt-color-danger-dark);
          margin-right: 8px;
        }

        .error-message {
          color: var(--ppt-color-danger-hover);
        }

        .more-errors {
          padding: 10px 12px;
          margin: 0;
          font-size: 13px;
          color: var(--ppt-color-danger-dark);
          font-style: italic;
        }

        .skip-option {
          display: flex;
          align-items: center;
          gap: 10px;
          padding: 16px;
          background: var(--ppt-bg-app);
          border-radius: 8px;
          margin-bottom: 24px;
          cursor: pointer;
        }

        .skip-option input {
          width: 18px;
          height: 18px;
        }

        .skip-option span {
          font-size: 14px;
          color: var(--ppt-fg-secondary);
        }

        .actions {
          display: flex;
          justify-content: space-between;
          padding-top: 24px;
          border-top: 1px solid var(--ppt-border-default);
        }

        button {
          padding: 12px 24px;
          border-radius: 8px;
          font-size: 14px;
          font-weight: 500;
          cursor: pointer;
          transition: all 0.2s;
        }

        .secondary {
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-strong);
          color: var(--ppt-fg-secondary);
        }

        .secondary:hover {
          background: var(--ppt-bg-app);
        }

        .primary {
          background: var(--ppt-color-primary);
          border: none;
          color: var(--ppt-fg-on-accent);
        }

        .primary:hover:not(:disabled) {
          background: var(--ppt-color-primary-hover);
        }

        .primary:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }
      `}</style>
    </div>
  );
}

function ImportingState() {
  const t = useTranslations('import.csv');
  return (
    <div className="importing-state">
      <div className="spinner" />
      <h3>{t('importing')}</h3>
      <p>{t('importingHint')}</p>

      <style jsx>{`
        .importing-state {
          display: flex;
          flex-direction: column;
          align-items: center;
          padding: 64px 24px;
          text-align: center;
        }

        .spinner {
          width: 48px;
          height: 48px;
          border: 3px solid var(--ppt-border-default);
          border-top-color: var(--ppt-color-primary);
          border-radius: 50%;
          animation: spin 0.8s linear infinite;
        }

        @keyframes spin {
          to {
            transform: rotate(360deg);
          }
        }

        h3 {
          font-size: 1.25rem;
          color: var(--ppt-fg-primary);
          margin: 24px 0 8px;
        }

        p {
          color: var(--ppt-fg-muted);
          margin: 0;
        }
      `}</style>
    </div>
  );
}

function CompleteStep({
  result,
  onNewImport,
}: {
  result: { successCount: number; failedCount: number; skippedCount: number };
  onNewImport: () => void;
}) {
  const t = useTranslations('import.csv');
  return (
    <div className="complete-state">
      <svg
        width="64"
        height="64"
        viewBox="0 0 24 24"
        fill="none"
        stroke="var(--ppt-color-success)"
        strokeWidth="2"
        aria-hidden="true"
      >
        <circle cx="12" cy="12" r="10" />
        <polyline points="16 8 10 14 8 12" />
      </svg>
      <h3>{t('complete')}</h3>

      <div className="result-summary">
        <div className="result-item success">
          <span className="value">{result.successCount}</span>
          <span className="label">{t('imported')}</span>
        </div>
        {result.failedCount > 0 && (
          <div className="result-item error">
            <span className="value">{result.failedCount}</span>
            <span className="label">{t('failed')}</span>
          </div>
        )}
        {result.skippedCount > 0 && (
          <div className="result-item warning">
            <span className="value">{result.skippedCount}</span>
            <span className="label">{t('skipped')}</span>
          </div>
        )}
      </div>

      <div className="actions">
        <a href="/agency/listings" className="view-button">
          {t('viewListings')}
        </a>
        <button type="button" onClick={onNewImport} className="new-import">
          {t('importMore')}
        </button>
      </div>

      <style jsx>{`
        .complete-state {
          display: flex;
          flex-direction: column;
          align-items: center;
          padding: 48px 24px;
          text-align: center;
        }

        h3 {
          font-size: 1.5rem;
          color: var(--ppt-fg-primary);
          margin: 24px 0;
        }

        .result-summary {
          display: flex;
          gap: 24px;
          margin-bottom: 32px;
        }

        .result-item {
          display: flex;
          flex-direction: column;
          align-items: center;
          padding: 16px 24px;
          border-radius: 12px;
        }

        .result-item .value {
          font-size: 2rem;
          font-weight: bold;
        }

        .result-item .label {
          font-size: 14px;
        }

        .result-item.success {
          background: var(--ppt-color-success-light);
        }

        .result-item.success .value {
          color: var(--ppt-color-success-hover);
        }

        .result-item.success .label {
          color: var(--ppt-color-success-dark);
        }

        .result-item.error {
          background: var(--ppt-color-danger-light);
        }

        .result-item.error .value {
          color: var(--ppt-color-danger-hover);
        }

        .result-item.error .label {
          color: var(--ppt-color-danger-dark);
        }

        .result-item.warning {
          background: var(--ppt-color-warning-light);
        }

        .result-item.warning .value {
          color: var(--ppt-color-warning-hover);
        }

        .result-item.warning .label {
          color: var(--ppt-color-warning-dark);
        }

        .actions {
          display: flex;
          gap: 16px;
        }

        .view-button,
        .new-import {
          padding: 12px 24px;
          border-radius: 8px;
          font-size: 14px;
          font-weight: 500;
          cursor: pointer;
          text-decoration: none;
        }

        .view-button {
          background: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent);
        }

        .new-import {
          background: var(--ppt-bg-surface);
          border: 1px solid var(--ppt-border-strong);
          color: var(--ppt-fg-secondary);
        }
      `}</style>
    </div>
  );
}

function LoadingState({ message }: { message: string }) {
  return (
    <div className="loading-state">
      <div className="spinner" />
      <p>{message}</p>

      <style jsx>{`
        .loading-state {
          display: flex;
          flex-direction: column;
          align-items: center;
          padding: 64px 24px;
        }

        .spinner {
          width: 40px;
          height: 40px;
          border: 3px solid var(--ppt-border-default);
          border-top-color: var(--ppt-color-primary);
          border-radius: 50%;
          animation: spin 0.8s linear infinite;
        }

        @keyframes spin {
          to {
            transform: rotate(360deg);
          }
        }

        p {
          margin-top: 16px;
          color: var(--ppt-fg-muted);
        }
      `}</style>
    </div>
  );
}

function ErrorState({ message, onRetry }: { message: string; onRetry: () => void }) {
  const t = useTranslations('import.csv');
  return (
    <div className="error-state">
      <svg
        width="48"
        height="48"
        viewBox="0 0 24 24"
        fill="none"
        stroke="var(--ppt-color-danger)"
        strokeWidth="2"
        aria-hidden="true"
      >
        <circle cx="12" cy="12" r="10" />
        <line x1="12" y1="8" x2="12" y2="12" />
        <line x1="12" y1="16" x2="12.01" y2="16" />
      </svg>
      <h3>{t('errorTitle')}</h3>
      <p>{message}</p>
      <button type="button" onClick={onRetry}>
        {t('tryAgain')}
      </button>

      <style jsx>{`
        .error-state {
          display: flex;
          flex-direction: column;
          align-items: center;
          padding: 64px 24px;
          text-align: center;
        }

        h3 {
          font-size: 1.25rem;
          color: var(--ppt-fg-primary);
          margin: 24px 0 8px;
        }

        p {
          color: var(--ppt-fg-muted);
          margin: 0 0 24px;
        }

        button {
          padding: 12px 24px;
          background: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent);
          border: none;
          border-radius: 8px;
          font-weight: 500;
          cursor: pointer;
        }
      `}</style>
    </div>
  );
}

function formatFieldName(field: string): string {
  return field
    .replace(/([A-Z])/g, ' $1')
    .replace(/^./, (str) => str.toUpperCase())
    .trim();
}
