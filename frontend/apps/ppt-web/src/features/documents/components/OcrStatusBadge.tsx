/**
 * OCR Processing Status Badge (Story 39.2).
 *
 * Shows the current OCR status with appropriate styling.
 */

import type { OcrStatus } from '@ppt/api-client';
import { formatDateTime } from '@ppt/shared';
import { useTranslation } from 'react-i18next';

interface OcrStatusBadgeProps {
  status: OcrStatus;
  compact?: boolean;
  onReprocess?: () => void;
  isReprocessing?: boolean;
}

export function OcrStatusBadge({
  status,
  compact = false,
  onReprocess,
  isReprocessing = false,
}: OcrStatusBadgeProps) {
  const { t } = useTranslation();
  // Get status configuration
  const getStatusConfig = (s: OcrStatus) => {
    const variant = compact ? 'compact' : 'full';
    switch (s) {
      case 'completed':
        return {
          label: t(`ocrStatus.${variant}.completed`),
          className: 'status-completed',
          icon: '✓',
        };
      case 'processing':
        return {
          label: t(`ocrStatus.${variant}.processing`),
          className: 'status-processing',
          icon: '⟳',
        };
      case 'pending':
        return {
          label: t(`ocrStatus.${variant}.pending`),
          className: 'status-pending',
          icon: '○',
        };
      case 'failed':
        return {
          label: t(`ocrStatus.${variant}.failed`),
          className: 'status-failed',
          icon: '✕',
        };
      case 'not_applicable':
        return {
          label: t(`ocrStatus.${variant}.not_applicable`),
          className: 'status-na',
          icon: '—',
        };
      default:
        return {
          label: status,
          className: 'status-unknown',
          icon: '?',
        };
    }
  };

  const config = getStatusConfig(status);

  return (
    <div className={`ocr-status-badge ${config.className} ${compact ? 'compact' : ''}`}>
      <span className={`status-icon ${isReprocessing ? 'spinning' : ''}`}>
        {isReprocessing ? '⟳' : config.icon}
      </span>
      <span className="status-label">
        {isReprocessing ? t('ocrStatus.reprocessing') : config.label}
      </span>

      {/* Reprocess button for failed status */}
      {status === 'failed' && onReprocess && !isReprocessing && (
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onReprocess();
          }}
          className="reprocess-button"
          title={t('ocrStatus.retryTitle')}
        >
          {t('ocrStatus.retry')}
        </button>
      )}

      <style>{`
        .ocr-status-badge {
          display: inline-flex;
          align-items: center;
          gap: 0.375rem;
          padding: 0.25rem 0.75rem;
          font-size: 0.75rem;
          font-weight: 500;
          border-radius: 9999px;
          white-space: nowrap;
        }

        .ocr-status-badge.compact {
          padding: 0.125rem 0.5rem;
          font-size: 0.6875rem;
        }

        .status-icon {
          font-size: 0.875em;
        }

        .status-icon.spinning {
          animation: spin 1s linear infinite;
        }

        @keyframes spin {
          from { transform: rotate(0deg); }
          to { transform: rotate(360deg); }
        }

        .status-completed {
          background: var(--ppt-color-success-light);
          color: var(--ppt-color-success-dark);
        }

        .status-processing {
          background: var(--ppt-color-primary-soft-bg);
          color: var(--ppt-color-primary-hover);
        }

        .status-pending {
          background: var(--ppt-color-warning-light);
          color: var(--ppt-color-warning-dark);
        }

        .status-failed {
          background: var(--ppt-color-danger-light);
          color: var(--ppt-color-danger-dark);
        }

        .status-na {
          background: var(--ppt-bg-app);
          color: var(--ppt-fg-muted);
        }

        .status-unknown {
          background: var(--ppt-border-default);
          color: var(--ppt-fg-secondary);
        }

        .reprocess-button {
          margin-left: 0.25rem;
          padding: 0.125rem 0.375rem;
          font-size: 0.6875rem;
          font-weight: 600;
          background: var(--ppt-bg-surface);
          border: 1px solid currentColor;
          border-radius: 0.25rem;
          color: inherit;
          cursor: pointer;
          transition: all 0.15s;
        }

        .reprocess-button:hover {
          background: var(--ppt-color-danger-light);
        }
      `}</style>
    </div>
  );
}

/**
 * OCR Status Display with Processing Animation (Story 39.2).
 */
interface OcrProcessingStatusProps {
  status: OcrStatus;
  processedAt?: string;
  onReprocess?: () => void;
  isReprocessing?: boolean;
}

export function OcrProcessingStatus({
  status,
  processedAt,
  onReprocess,
  isReprocessing,
}: OcrProcessingStatusProps) {
  const { t, i18n } = useTranslation();
  const formatDate = (dateString: string): string =>
    formatDateTime(dateString, { locale: i18n.language });

  return (
    <div className="ocr-processing-status">
      <div className="status-header">
        <h4 className="status-title">{t('ocrStatus.processingTitle')}</h4>
        <OcrStatusBadge status={status} onReprocess={onReprocess} isReprocessing={isReprocessing} />
      </div>

      {status === 'processing' && (
        <div className="processing-animation">
          <div className="progress-bar">
            <div className="progress-bar-fill" />
          </div>
          <p className="processing-text">{t('ocrStatus.extractingText')}</p>
        </div>
      )}

      {status === 'completed' && processedAt && (
        <p className="processed-date">
          {t('ocrStatus.textExtractedOn', { date: formatDate(processedAt) })}
        </p>
      )}

      {status === 'failed' && (
        <p className="error-text">
          {t('ocrStatus.processingFailed')}
          {onReprocess && ` ${t('ocrStatus.clickRetry')}`}
        </p>
      )}

      {status === 'pending' && <p className="pending-text">{t('ocrStatus.queuedForExtraction')}</p>}

      <style>{`
        .ocr-processing-status {
          padding: 1rem;
          background: var(--ppt-bg-app);
          border-radius: 0.5rem;
        }

        .status-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 0.5rem;
        }

        .status-title {
          margin: 0;
          font-size: 0.875rem;
          font-weight: 600;
          color: var(--ppt-fg-primary);
        }

        .processing-animation {
          margin-top: 0.75rem;
        }

        .progress-bar {
          height: 4px;
          background: var(--ppt-border-default);
          border-radius: 2px;
          overflow: hidden;
        }

        .progress-bar-fill {
          height: 100%;
          width: 30%;
          background: var(--ppt-brand-500);
          border-radius: 2px;
          animation: progress 1.5s ease-in-out infinite;
        }

        @keyframes progress {
          0% { transform: translateX(-100%); }
          100% { transform: translateX(400%); }
        }

        .processing-text,
        .processed-date,
        .error-text,
        .pending-text {
          margin: 0.5rem 0 0;
          font-size: 0.875rem;
          color: var(--ppt-fg-muted);
        }

        .error-text {
          color: var(--ppt-color-danger-hover);
        }
      `}</style>
    </div>
  );
}
