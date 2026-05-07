/**
 * Document Summary View (Story 39.4).
 *
 * Displays AI-generated document summaries with key points.
 */

import { useRequestSummarization } from '@ppt/api-client';
import { useState } from 'react';

interface DocumentSummaryProps {
  documentId: string;
  summary?: string;
  summaryGeneratedAt?: string;
  wordCount?: number;
  onSummaryGenerated?: () => void;
}

export function DocumentSummary({
  documentId,
  summary,
  summaryGeneratedAt,
  wordCount = 0,
  onSummaryGenerated,
}: DocumentSummaryProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const [summaryStyle, setSummaryStyle] = useState<'brief' | 'detailed' | 'bullets'>('brief');
  const requestSummary = useRequestSummarization();

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

  const handleGenerateSummary = () => {
    requestSummary.mutate(
      {
        id: documentId,
        options: { style: summaryStyle },
      },
      {
        onSuccess: () => {
          onSummaryGenerated?.();
        },
      }
    );
  };

  const handleRegenerateSummary = () => {
    requestSummary.mutate(
      {
        id: documentId,
        options: { style: summaryStyle },
      },
      {
        onSuccess: () => {
          onSummaryGenerated?.();
        },
      }
    );
  };

  // Parse bullet points if summary contains them
  const parseSummaryContent = (text: string) => {
    const lines = text.split('\n').filter((line) => line.trim());
    const isBulletList = lines.every(
      (line) => line.trim().startsWith('-') || line.trim().startsWith('•')
    );

    if (isBulletList) {
      return {
        type: 'bullets' as const,
        items: lines.map((line) => line.replace(/^[-•]\s*/, '').trim()),
      };
    }

    return {
      type: 'text' as const,
      content: text,
    };
  };

  // Show minimum words requirement
  const showGenerateOption = wordCount >= 100;

  if (!summary) {
    return (
      <div className="document-summary">
        <div className="summary-header">
          <h4 className="summary-title">
            <span className="ai-icon">AI</span>
            Summary
          </h4>
        </div>

        {showGenerateOption ? (
          <div className="generate-section">
            <p className="generate-prompt">Generate an AI summary of this document?</p>

            <fieldset className="style-selector">
              <legend className="style-label">Summary style:</legend>
              <div className="style-options">
                {(['brief', 'detailed', 'bullets'] as const).map((style) => (
                  <button
                    key={style}
                    type="button"
                    onClick={() => setSummaryStyle(style)}
                    className={`style-button ${summaryStyle === style ? 'active' : ''}`}
                  >
                    {style.charAt(0).toUpperCase() + style.slice(1)}
                  </button>
                ))}
              </div>
            </fieldset>

            <button
              type="button"
              onClick={handleGenerateSummary}
              disabled={requestSummary.isPending}
              className="generate-button"
            >
              {requestSummary.isPending ? (
                <>
                  <span className="spinner" />
                  Generating...
                </>
              ) : (
                'Generate Summary'
              )}
            </button>
          </div>
        ) : (
          <p className="no-summary-text">
            Document is too short for summarization (minimum 100 words required).
          </p>
        )}

        {requestSummary.isError && (
          <div className="error-message">Failed to generate summary. Please try again.</div>
        )}

        {requestSummary.isSuccess && !summary && (
          <div className="processing-message">
            Summary is being generated. It will appear shortly...
          </div>
        )}

        <style>{summaryStyles}</style>
      </div>
    );
  }

  const parsedContent = parseSummaryContent(summary);

  return (
    <div className="document-summary has-summary">
      <div className="summary-header">
        <h4 className="summary-title">
          <span className="ai-icon">AI</span>
          Summary
        </h4>
        <div className="summary-actions">
          <button
            type="button"
            onClick={() => setIsExpanded(!isExpanded)}
            className="toggle-button"
          >
            {isExpanded ? 'Collapse' : 'Expand'}
          </button>
          <button
            type="button"
            onClick={handleRegenerateSummary}
            disabled={requestSummary.isPending}
            className="regenerate-button"
            title="Generate a new summary"
          >
            {requestSummary.isPending ? '...' : '↻'}
          </button>
        </div>
      </div>

      <div className={`summary-content ${isExpanded ? 'expanded' : 'collapsed'}`}>
        {parsedContent.type === 'bullets' ? (
          <ul className="summary-bullets">
            {parsedContent.items.map((item, index) => (
              <li key={`${index}-${item.slice(0, 20)}`} className="summary-bullet">
                {item}
              </li>
            ))}
          </ul>
        ) : (
          <p className="summary-text">{parsedContent.content}</p>
        )}
      </div>

      {summaryGeneratedAt && (
        <div className="summary-footer">
          <span className="generated-date">Generated: {formatDate(summaryGeneratedAt)}</span>
        </div>
      )}

      <style>{summaryStyles}</style>
    </div>
  );
}

const summaryStyles = `
  .document-summary {
    padding: 1rem;
    background: linear-gradient(135deg, #ede9fe 0%, #ddd6fe 100%);
    border-radius: 0.5rem;
    border: 1px solid #c4b5fd;
  }

  .summary-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.75rem;
  }

  .summary-title {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0;
    font-size: 0.875rem;
    font-weight: 600;
    color: #5b21b6;
  }

  .ai-icon {
    padding: 0.125rem 0.375rem;
    font-size: 0.625rem;
    font-weight: 700;
    background: #7c3aed;
    color: var(--ppt-fg-on-accent);
    border-radius: 0.25rem;
  }

  .summary-actions {
    display: flex;
    gap: 0.5rem;
  }

  .toggle-button,
  .regenerate-button {
    padding: 0.25rem 0.5rem;
    font-size: 0.75rem;
    font-weight: 500;
    background: var(--ppt-bg-surface);
    border: 1px solid #c4b5fd;
    border-radius: 0.25rem;
    color: #7c3aed;
    cursor: pointer;
    transition: all 0.15s;
  }

  .toggle-button:hover,
  .regenerate-button:hover {
    background: #f5f3ff;
  }

  .summary-content {
    overflow: hidden;
    transition: max-height 0.3s ease;
  }

  .summary-content.collapsed {
    max-height: 4.5rem;
    position: relative;
  }

  .summary-content.collapsed::after {
    content: '';
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    height: 2rem;
    background: linear-gradient(transparent, #ede9fe);
  }

  .summary-content.expanded {
    max-height: none;
  }

  .summary-text {
    margin: 0;
    font-size: 0.875rem;
    line-height: 1.6;
    color: #4c1d95;
  }

  .summary-bullets {
    margin: 0;
    padding-left: 1.25rem;
    list-style-type: disc;
  }

  .summary-bullet {
    font-size: 0.875rem;
    line-height: 1.5;
    color: #4c1d95;
    margin-bottom: 0.25rem;
  }

  .summary-footer {
    margin-top: 0.75rem;
    padding-top: 0.5rem;
    border-top: 1px solid #c4b5fd;
  }

  .generated-date {
    font-size: 0.75rem;
    color: #7c3aed;
  }

  .generate-section {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .generate-prompt {
    margin: 0;
    font-size: 0.875rem;
    color: #5b21b6;
  }

  .style-selector {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    border: none;
    padding: 0;
    margin: 0;
  }

  .style-label {
    font-size: 0.75rem;
    font-weight: 500;
    color: #7c3aed;
    padding: 0;
  }

  .style-options {
    display: flex;
    gap: 0.25rem;
  }

  .style-button {
    padding: 0.25rem 0.75rem;
    font-size: 0.75rem;
    font-weight: 500;
    background: var(--ppt-bg-surface);
    border: 1px solid #c4b5fd;
    border-radius: 9999px;
    color: #7c3aed;
    cursor: pointer;
    transition: all 0.15s;
  }

  .style-button:hover {
    background: #f5f3ff;
  }

  .style-button.active {
    background: #7c3aed;
    border-color: #7c3aed;
    color: var(--ppt-fg-on-accent);
  }

  .generate-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    font-size: 0.875rem;
    font-weight: 600;
    background: #7c3aed;
    border: none;
    border-radius: 0.375rem;
    color: var(--ppt-fg-on-accent);
    cursor: pointer;
    transition: all 0.15s;
  }

  .generate-button:hover:not(:disabled) {
    background: #6d28d9;
  }

  .generate-button:disabled {
    opacity: 0.7;
    cursor: not-allowed;
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

  .no-summary-text {
    margin: 0;
    font-size: 0.875rem;
    color: #7c3aed;
    font-style: italic;
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

  .processing-message {
    margin-top: 0.75rem;
    padding: 0.5rem 0.75rem;
    background: var(--ppt-color-success-light);
    border: 1px solid var(--ppt-color-success-light);
    border-radius: 0.375rem;
    color: var(--ppt-color-success-dark);
    font-size: 0.875rem;
  }
`;
