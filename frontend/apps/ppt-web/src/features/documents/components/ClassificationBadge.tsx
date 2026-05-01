/**
 * Document Classification Badge and UI (Story 39.3).
 *
 * Shows AI-predicted category with confidence and feedback options.
 */

import {
  type ClassificationResponse,
  DOCUMENT_CATEGORIES,
  useSubmitClassificationFeedback,
} from '@ppt/api-client';
import { useState } from 'react';

interface ClassificationBadgeProps {
  category: string;
  confidence: number;
  compact?: boolean;
  accepted?: boolean;
}

export function ClassificationBadge({
  category,
  confidence,
  compact = false,
  accepted,
}: ClassificationBadgeProps) {
  // Confidence level styling
  const getConfidenceLevel = (conf: number) => {
    if (conf >= 0.8) return 'high';
    if (conf >= 0.5) return 'medium';
    return 'low';
  };

  const confidenceLevel = getConfidenceLevel(confidence);
  const confidencePercent = Math.round(confidence * 100);

  return (
    <div
      className={`classification-badge confidence-${confidenceLevel} ${compact ? 'compact' : ''} ${accepted ? 'accepted' : ''}`}
    >
      <span className="ai-icon">AI</span>
      <span className="category-label">{category}</span>
      {!compact && <span className="confidence-value">{confidencePercent}%</span>}
      {accepted && <span className="accepted-icon">✓</span>}

      <style>{`
        .classification-badge {
          display: inline-flex;
          align-items: center;
          gap: 0.375rem;
          padding: 0.25rem 0.75rem;
          font-size: 0.75rem;
          font-weight: 500;
          border-radius: 9999px;
          white-space: nowrap;
        }

        .classification-badge.compact {
          padding: 0.125rem 0.5rem;
          font-size: 0.6875rem;
        }

        .ai-icon {
          padding: 0.0625rem 0.25rem;
          font-size: 0.625rem;
          font-weight: 700;
          background: currentColor;
          color: var(--ppt-fg-on-accent);
          border-radius: 0.125rem;
        }

        .confidence-high {
          background: var(--ppt-color-success-light);
          color: var(--ppt-color-success-dark);
        }

        .confidence-high .ai-icon {
          background: var(--ppt-color-success-dark);
        }

        .confidence-medium {
          background: var(--ppt-color-warning-light);
          color: var(--ppt-color-warning-dark);
        }

        .confidence-medium .ai-icon {
          background: var(--ppt-color-warning-dark);
        }

        .confidence-low {
          background: var(--ppt-color-danger-light);
          color: var(--ppt-color-danger-dark);
        }

        .confidence-low .ai-icon {
          background: var(--ppt-color-danger-dark);
        }

        .confidence-value {
          opacity: 0.8;
          font-size: 0.6875rem;
        }

        .accepted-icon {
          color: inherit;
        }

        .classification-badge.accepted {
          border: 1px solid currentColor;
        }
      `}</style>
    </div>
  );
}

/**
 * Full Classification UI with feedback (Story 39.3).
 */
interface ClassificationUIProps {
  documentId: string;
  classification: ClassificationResponse;
  onFeedbackSubmitted?: () => void;
}

export function ClassificationUI({
  documentId,
  classification,
  onFeedbackSubmitted,
}: ClassificationUIProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [selectedCategory, setSelectedCategory] = useState<string>('');
  const submitFeedback = useSubmitClassificationFeedback();

  const handleAccept = () => {
    submitFeedback.mutate(
      {
        id: documentId,
        feedback: { accepted: true },
      },
      {
        onSuccess: () => {
          onFeedbackSubmitted?.();
        },
      }
    );
  };

  const handleReject = () => {
    setIsEditing(true);
  };

  const handleSubmitCorrection = () => {
    if (!selectedCategory) return;

    submitFeedback.mutate(
      {
        id: documentId,
        feedback: {
          accepted: false,
          correct_category: selectedCategory,
        },
      },
      {
        onSuccess: () => {
          setIsEditing(false);
          onFeedbackSubmitted?.();
        },
      }
    );
  };

  const formatDate = (dateString: string): string => {
    const date = new Date(dateString);
    return date.toLocaleString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  if (!classification.predicted_category) {
    return (
      <div className="classification-ui">
        <div className="no-classification">
          <span className="ai-icon-large">AI</span>
          <p>No classification available yet. The document will be automatically analyzed.</p>
        </div>

        <style>{`
          .classification-ui {
            padding: 1rem;
            background: var(--ppt-bg-app);
            border-radius: 0.5rem;
          }

          .no-classification {
            display: flex;
            align-items: center;
            gap: 0.75rem;
          }

          .ai-icon-large {
            padding: 0.25rem 0.5rem;
            font-size: 0.75rem;
            font-weight: 700;
            background: var(--ppt-fg-subtle);
            color: var(--ppt-fg-on-accent);
            border-radius: 0.25rem;
          }

          .no-classification p {
            margin: 0;
            font-size: 0.875rem;
            color: var(--ppt-fg-muted);
          }
        `}</style>
      </div>
    );
  }

  return (
    <div className="classification-ui">
      <div className="classification-header">
        <h4 className="section-title">AI Classification</h4>
        {classification.classified_at && (
          <span className="classified-date">
            Classified: {formatDate(classification.classified_at)}
          </span>
        )}
      </div>

      <div className="classification-result">
        <ClassificationBadge
          category={classification.predicted_category}
          confidence={classification.confidence || 0}
          accepted={classification.accepted || false}
        />
      </div>

      {/* Already accepted/rejected */}
      {classification.accepted !== null && classification.accepted !== undefined && (
        <div className={`feedback-status ${classification.accepted ? 'accepted' : 'rejected'}`}>
          {classification.accepted ? (
            <p>You confirmed this classification is correct.</p>
          ) : (
            <p>You provided a correction for this classification.</p>
          )}
        </div>
      )}

      {/* Feedback buttons (only if not yet responded) */}
      {classification.accepted === null || classification.accepted === undefined ? (
        <div className="feedback-actions">
          {!isEditing ? (
            <>
              <p className="feedback-prompt">Is this classification correct?</p>
              <div className="feedback-buttons">
                <button
                  type="button"
                  onClick={handleAccept}
                  disabled={submitFeedback.isPending}
                  className="btn btn-accept"
                >
                  {submitFeedback.isPending ? 'Saving...' : 'Yes, correct'}
                </button>
                <button
                  type="button"
                  onClick={handleReject}
                  disabled={submitFeedback.isPending}
                  className="btn btn-reject"
                >
                  No, suggest correction
                </button>
              </div>
            </>
          ) : (
            <div className="correction-form">
              <label className="correction-label" htmlFor="category-select">
                Select the correct category:
              </label>
              <select
                id="category-select"
                value={selectedCategory}
                onChange={(e) => setSelectedCategory(e.target.value)}
                className="category-select"
              >
                <option value="">Select a category...</option>
                {DOCUMENT_CATEGORIES.map((cat) => (
                  <option key={cat} value={cat}>
                    {cat}
                  </option>
                ))}
              </select>
              <div className="correction-buttons">
                <button
                  type="button"
                  onClick={handleSubmitCorrection}
                  disabled={!selectedCategory || submitFeedback.isPending}
                  className="btn btn-submit"
                >
                  {submitFeedback.isPending ? 'Saving...' : 'Submit Correction'}
                </button>
                <button
                  type="button"
                  onClick={() => setIsEditing(false)}
                  disabled={submitFeedback.isPending}
                  className="btn btn-cancel"
                >
                  Cancel
                </button>
              </div>
            </div>
          )}
        </div>
      ) : null}

      {submitFeedback.isError && (
        <div className="error-message">Failed to submit feedback. Please try again.</div>
      )}

      <style>{`
        .classification-ui {
          padding: 1rem;
          background: var(--ppt-bg-app);
          border-radius: 0.5rem;
        }

        .classification-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 0.75rem;
        }

        .section-title {
          margin: 0;
          font-size: 0.875rem;
          font-weight: 600;
          color: var(--ppt-fg-primary);
        }

        .classified-date {
          font-size: 0.75rem;
          color: var(--ppt-fg-muted);
        }

        .classification-result {
          margin-bottom: 1rem;
        }

        .feedback-status {
          padding: 0.5rem 0.75rem;
          border-radius: 0.375rem;
          font-size: 0.875rem;
        }

        .feedback-status.accepted {
          background: var(--ppt-color-success-light);
          color: var(--ppt-color-success-dark);
        }

        .feedback-status.rejected {
          background: var(--ppt-color-warning-light);
          color: var(--ppt-color-warning-dark);
        }

        .feedback-status p {
          margin: 0;
        }

        .feedback-actions {
          margin-top: 1rem;
          padding-top: 1rem;
          border-top: 1px solid var(--ppt-border-default);
        }

        .feedback-prompt {
          margin: 0 0 0.75rem;
          font-size: 0.875rem;
          font-weight: 500;
          color: var(--ppt-fg-secondary);
        }

        .feedback-buttons {
          display: flex;
          gap: 0.5rem;
        }

        .btn {
          padding: 0.5rem 1rem;
          font-size: 0.875rem;
          font-weight: 500;
          border: none;
          border-radius: 0.375rem;
          cursor: pointer;
          transition: all 0.15s;
        }

        .btn:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }

        .btn-accept {
          background: var(--ppt-color-success-light);
          color: var(--ppt-color-success-dark);
        }

        .btn-accept:hover:not(:disabled) {
          background: var(--ppt-color-success-light);
        }

        .btn-reject {
          background: var(--ppt-color-danger-light);
          color: var(--ppt-color-danger-dark);
        }

        .btn-reject:hover:not(:disabled) {
          background: var(--ppt-color-danger-light);
        }

        .correction-form {
          display: flex;
          flex-direction: column;
          gap: 0.75rem;
        }

        .correction-label {
          font-size: 0.875rem;
          font-weight: 500;
          color: var(--ppt-fg-secondary);
        }

        .category-select {
          padding: 0.5rem 0.75rem;
          font-size: 0.875rem;
          border: 1px solid var(--ppt-border-default);
          border-radius: 0.375rem;
          background: var(--ppt-bg-surface);
        }

        .correction-buttons {
          display: flex;
          gap: 0.5rem;
        }

        .btn-submit {
          background: var(--ppt-brand-500);
          color: var(--ppt-fg-on-accent);
        }

        .btn-submit:hover:not(:disabled) {
          background: var(--ppt-color-primary);
        }

        .btn-cancel {
          background: var(--ppt-border-default);
          color: var(--ppt-fg-secondary);
        }

        .btn-cancel:hover:not(:disabled) {
          background: var(--ppt-border-strong);
        }

        .error-message {
          margin-top: 0.75rem;
          padding: 0.5rem 0.75rem;
          background: var(--ppt-color-danger-light);
          border: 1px solid var(--ppt-color-danger-light);
          border-radius: 0.375rem;
          color: var(--ppt-color-danger);
          font-size: 0.875rem;
        }
      `}</style>
    </div>
  );
}
