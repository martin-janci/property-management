/**
 * AiSuggestionBadge component (Epic 126, Story 126.2).
 *
 * Displays AI-suggested category and priority with confidence indicator.
 * Allows users to accept or modify suggestions.
 */

import type { FaultCategory, FaultPriority } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

export interface AiSuggestionBadgeProps {
  /** Suggested category */
  category: FaultCategory;
  /** Confidence score (0-1) */
  confidence: number;
  /** Suggested priority (optional) */
  priority?: FaultPriority;
  /** Whether suggestion is loading */
  isLoading?: boolean;
  /** Callback when user accepts the suggestion */
  onAccept: (category: FaultCategory, priority?: FaultPriority) => void;
  /** Callback when user modifies the suggestion */
  onModify: () => void;
  /** Whether the suggestion was accepted */
  isAccepted?: boolean;
}

/** Get confidence level for display */
function getConfidenceLevel(confidence: number): 'high' | 'medium' | 'low' {
  if (confidence >= 0.8) return 'high';
  if (confidence >= 0.5) return 'medium';
  return 'low';
}

/** Get color classes based on confidence */
function getConfidenceColors(level: 'high' | 'medium' | 'low') {
  switch (level) {
    case 'high':
      return {
        bg: 'bg-green-100',
        text: 'text-green-800',
        bar: 'bg-green-500',
        border: 'border-green-200',
      };
    case 'medium':
      return {
        bg: 'bg-yellow-100',
        text: 'text-yellow-800',
        bar: 'bg-yellow-500',
        border: 'border-yellow-200',
      };
    case 'low':
      return {
        bg: 'bg-red-100',
        text: 'text-red-800',
        bar: 'bg-red-500',
        border: 'border-red-200',
      };
  }
}

/** Get priority badge styles */
function getPriorityStyles(priority: FaultPriority) {
  switch (priority) {
    case 'urgent':
      return 'bg-red-100 text-red-800';
    case 'high':
      return 'bg-orange-100 text-orange-800';
    case 'medium':
      return 'bg-yellow-100 text-yellow-800';
    case 'low':
      return 'bg-green-100 text-green-800';
  }
}

export function AiSuggestionBadge({
  category,
  confidence,
  priority,
  isLoading = false,
  onAccept,
  onModify,
  isAccepted = false,
}: AiSuggestionBadgeProps) {
  const { t } = useTranslation();
  const [isExpanded, setIsExpanded] = useState(false);

  const confidenceLevel = getConfidenceLevel(confidence);
  const colors = getConfidenceColors(confidenceLevel);
  const confidencePercent = Math.round(confidence * 100);

  if (isLoading) {
    return (
      <div className="flex items-center gap-2 p-3 bg-gray-50 rounded-lg border border-gray-200 animate-pulse">
        <div className="w-5 h-5 bg-gray-200 rounded-full" />
        <div className="flex-1">
          <div className="h-4 bg-gray-200 rounded w-32 mb-1" />
          <div className="h-3 bg-gray-200 rounded w-24" />
        </div>
      </div>
    );
  }

  return (
    <div
      className={`
        p-4 rounded-lg border transition-all duration-200
        ${isAccepted ? 'bg-green-50 border-green-300' : `${colors.bg} ${colors.border}`}
      `}
      role="region"
      aria-label={t('faults.ai.suggestionRegion')}
    >
      {/* Header */}
      <div className="flex items-start gap-3">
        {/* AI Icon */}
        <div className={`p-2 rounded-full ${isAccepted ? 'bg-green-200' : 'bg-blue-100'}`}>
          <svg
            className={`w-5 h-5 ${isAccepted ? 'text-green-700' : 'text-blue-600'}`}
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
            aria-hidden="true"
          >
            {isAccepted ? (
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M5 13l4 4L19 7"
              />
            ) : (
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"
              />
            )}
          </svg>
        </div>

        {/* Content */}
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 flex-wrap">
            <h4 className="font-medium text-gray-900">
              {isAccepted ? t('faults.ai.accepted') : t('faults.ai.suggestion')}
            </h4>
            <span
              className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium ${colors.bg} ${colors.text}`}
            >
              {confidencePercent}% {t(`faults.ai.confidence.${confidenceLevel}`)}
            </span>
          </div>

          {/* Suggested values */}
          <div className="mt-2 flex items-center gap-2 flex-wrap">
            <span className="inline-flex items-center px-2.5 py-1 rounded-md text-sm font-medium bg-gray-100 text-gray-800">
              {t(`faults.category.${category}`)}
            </span>
            {priority && (
              <span
                className={`inline-flex items-center px-2.5 py-1 rounded-md text-sm font-medium ${getPriorityStyles(priority)}`}
              >
                {t(`faults.priority${priority.charAt(0).toUpperCase()}${priority.slice(1)}`)}
              </span>
            )}
          </div>

          {/* Confidence bar */}
          <div className="mt-3">
            <div className="flex items-center justify-between text-xs text-gray-500 mb-1">
              <span>{t('faults.ai.confidenceLabel')}</span>
              <span>{confidencePercent}%</span>
            </div>
            <div className="h-1.5 bg-gray-200 rounded-full overflow-hidden">
              <div
                className={`h-full ${colors.bar} transition-all duration-500`}
                style={{ width: `${confidencePercent}%` }}
                role="progressbar"
                aria-valuenow={confidencePercent}
                aria-valuemin={0}
                aria-valuemax={100}
                aria-label={t('faults.ai.confidenceProgress')}
              />
            </div>
          </div>
        </div>

        {/* Expand button (mobile) */}
        <button
          type="button"
          onClick={() => setIsExpanded(!isExpanded)}
          className="sm:hidden p-1 text-gray-400 hover:text-gray-600"
          aria-expanded={isExpanded}
          aria-label={isExpanded ? t('common.collapse') : t('common.expand')}
        >
          <svg
            className={`w-5 h-5 transform transition-transform ${isExpanded ? 'rotate-180' : ''}`}
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
          </svg>
        </button>
      </div>

      {/* Actions */}
      {!isAccepted && (
        <div className={`mt-4 flex gap-2 ${isExpanded ? 'flex' : 'hidden sm:flex'}`}>
          <button
            type="button"
            onClick={() => onAccept(category, priority)}
            className="flex-1 px-3 py-2 bg-blue-600 text-white text-sm font-medium rounded-md hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
          >
            {t('faults.ai.acceptSuggestion')}
          </button>
          <button
            type="button"
            onClick={onModify}
            className="flex-1 px-3 py-2 border border-gray-300 text-gray-700 text-sm font-medium rounded-md hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2"
          >
            {t('faults.ai.modifySuggestion')}
          </button>
        </div>
      )}

      {/* Explanation text */}
      {!isAccepted && confidenceLevel !== 'high' && (
        <p className="mt-3 text-xs text-gray-500">{t('faults.ai.lowConfidenceNote')}</p>
      )}
    </div>
  );
}

AiSuggestionBadge.displayName = 'AiSuggestionBadge';
