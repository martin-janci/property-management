/**
 * TourOverlay — step-by-step onboarding tour modal.
 * Epic 10B, Story 10B.6.
 *
 * Rendered as a fixed overlay. Receives tour + progress as props and
 * delegates mutations upward via callbacks so the parent page controls
 * query invalidation.
 */

import { useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { useFocusTrap } from '../../components/useFocusTrap';
import type { OnboardingTour, TourStepData, UserOnboardingProgress } from './types';
import { calculateTourProgress, getCurrentStepIndex, getTourDisplayStatus } from './types';

interface TourOverlayProps {
  tour: OnboardingTour;
  progress: UserOnboardingProgress | null;
  isMutating: boolean;
  /**
   * Called when the user completes a step.
   *
   * `isLast` lets the parent chain the `completeTour` mutation into the
   * `onSuccess` callback of the step mutation, so the two POSTs are
   * sequenced rather than racing (see issue #702 finding #1).
   */
  onCompleteStep: (stepId: string, isLast: boolean) => void;
  onSkipTour: () => void;
  onResetTour: () => void;
  onClose: () => void;
}

// ─── Step dot indicator ────────────────────────────────────────────────────

function StepDot({
  index,
  isCompleted,
  isCurrent,
}: {
  index: number;
  isCompleted: boolean;
  isCurrent: boolean;
}) {
  const base: React.CSSProperties = {
    width: 28,
    height: 28,
    borderRadius: '50%',
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    fontSize: 12,
    fontWeight: 600,
    flexShrink: 0,
    border: '2px solid transparent',
  };

  const style: React.CSSProperties = isCompleted
    ? { ...base, background: 'var(--ppt-brand-600, #2563eb)', color: '#fff' }
    : isCurrent
      ? {
          ...base,
          background: 'transparent',
          border: '2px solid var(--ppt-brand-600, #2563eb)',
          color: 'var(--ppt-brand-700, #1d4ed8)',
        }
      : { ...base, background: 'var(--ppt-border-default, #e5e7eb)', color: '#6b7280' };

  return (
    <div style={style}>
      {isCompleted ? (
        <svg width="14" height="14" viewBox="0 0 20 20" fill="currentColor">
          <title>Done</title>
          <path
            fillRule="evenodd"
            d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
            clipRule="evenodd"
          />
        </svg>
      ) : (
        index + 1
      )}
    </div>
  );
}

// ─── Single step row ───────────────────────────────────────────────────────

interface StepRowProps {
  step: TourStepData;
  index: number;
  totalSteps: number;
  isCompleted: boolean;
  isCurrent: boolean;
  isLast: boolean;
  isMutating: boolean;
  onComplete: () => void;
}

function StepRow({
  step,
  index,
  totalSteps,
  isCompleted,
  isCurrent,
  isLast,
  isMutating,
  onComplete,
}: StepRowProps) {
  const { t } = useTranslation();

  return (
    <div
      style={{
        display: 'flex',
        gap: 12,
        padding: '12px 0',
        borderBottom: isLast ? 'none' : '1px solid var(--ppt-border-default, #e5e7eb)',
        opacity: !isCurrent && !isCompleted ? 0.5 : 1,
      }}
    >
      {/* Step dot */}
      <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 4 }}>
        <StepDot index={index} isCompleted={isCompleted} isCurrent={isCurrent} />
        {!isLast && (
          <div
            style={{
              width: 2,
              flex: 1,
              minHeight: 20,
              background: isCompleted
                ? 'var(--ppt-brand-200, #bfdbfe)'
                : 'var(--ppt-border-default, #e5e7eb)',
              borderRadius: 1,
            }}
          />
        )}
      </div>

      {/* Step content */}
      <div style={{ flex: 1, paddingBottom: isLast ? 0 : 12 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
          <span style={{ fontWeight: 600, fontSize: 14, color: 'var(--ppt-fg-primary, #111827)' }}>
            {step.title}
          </span>
          <span style={{ fontSize: 11, color: 'var(--ppt-fg-muted, #6b7280)' }}>
            {t('onboarding.stepOf', { current: index + 1, total: totalSteps })}
          </span>
          {isCompleted && (
            <span
              style={{
                padding: '1px 8px',
                borderRadius: 9999,
                fontSize: 11,
                fontWeight: 600,
                background: '#d1fae5',
                color: '#065f46',
              }}
            >
              {t('onboarding.status.completed')}
            </span>
          )}
        </div>
        <p
          style={{
            fontSize: 13,
            color: 'var(--ppt-fg-secondary, #374151)',
            margin: 0,
            lineHeight: 1.5,
          }}
        >
          {step.content}
        </p>
        {step.target && isCurrent && (
          <p
            style={{
              marginTop: 6,
              fontSize: 12,
              color: '#92400e',
              background: '#fef3c7',
              border: '1px solid #fde68a',
              borderRadius: 4,
              padding: '4px 8px',
            }}
          >
            <strong>{t('onboarding.targetHint')}:</strong> {step.target}
          </p>
        )}

        {/* Actions for current step */}
        {isCurrent && !isCompleted && (
          <div style={{ display: 'flex', gap: 8, marginTop: 10 }}>
            <button
              type="button"
              disabled={isMutating}
              onClick={onComplete}
              style={{
                padding: '6px 14px',
                fontSize: 13,
                fontWeight: 600,
                background: isLast ? '#059669' : 'var(--ppt-brand-600, #2563eb)',
                color: '#fff',
                border: 'none',
                borderRadius: 6,
                cursor: isMutating ? 'not-allowed' : 'pointer',
                opacity: isMutating ? 0.7 : 1,
              }}
            >
              {isLast ? t('onboarding.finishTour') : t('onboarding.completeStep')}
            </button>
            {/*
              "Skip step" button removed (issue #702 finding #2): there is no
              backend skip-step endpoint, and aliasing it to the complete-step
              POST recorded a misleading "completed" entry in the audit trail.
              Tour-level skip is still available in the footer via `onSkipTour`.
            */}
          </div>
        )}
      </div>
    </div>
  );
}

// ─── Main overlay ──────────────────────────────────────────────────────────

export function TourOverlay({
  tour,
  progress,
  isMutating,
  onCompleteStep,
  onSkipTour,
  onResetTour,
  onClose,
}: TourOverlayProps) {
  const { t } = useTranslation();
  const panelRef = useRef<HTMLDivElement | null>(null);
  useFocusTrap(panelRef, onClose);

  const displayStatus = getTourDisplayStatus(progress);
  const progressPct = calculateTourProgress(tour.steps.length, progress);
  const currentStepIndex = getCurrentStepIndex(tour.steps, progress);
  const completedIds = progress?.completed_steps ?? [];

  const isCompleted = displayStatus === 'completed';
  const isSkipped = displayStatus === 'skipped';

  return (
    /* Backdrop */
    <div
      style={{
        position: 'fixed',
        inset: 0,
        // 1050 sits above Toast (z-index 1000 in Toast.css) and below the
        // OAuthClients modal (9000) and DestructiveConfirmDialog (9999), so
        // mutation-success toasts render on top of an open tour overlay
        // instead of behind it (issue #702 finding #3).
        zIndex: 1050,
        background: 'rgba(0,0,0,0.45)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: 16,
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      {/* Panel */}
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby="tour-overlay-title"
        tabIndex={-1}
        style={{
          background: 'var(--ppt-bg-surface, #fff)',
          borderRadius: 12,
          boxShadow: '0 20px 60px rgba(0,0,0,0.25)',
          width: '100%',
          maxWidth: 560,
          maxHeight: '90vh',
          display: 'flex',
          flexDirection: 'column',
          overflow: 'hidden',
        }}
      >
        {/* Header */}
        <div
          style={{
            padding: '20px 24px 16px',
            borderBottom: '1px solid var(--ppt-border-default, #e5e7eb)',
          }}
        >
          <div
            style={{
              display: 'flex',
              alignItems: 'flex-start',
              justifyContent: 'space-between',
              gap: 12,
            }}
          >
            <div style={{ flex: 1 }}>
              <h2
                id="tour-overlay-title"
                style={{
                  margin: 0,
                  fontSize: 18,
                  fontWeight: 700,
                  color: 'var(--ppt-fg-primary, #111827)',
                }}
              >
                {tour.name}
              </h2>
              {tour.description && (
                <p
                  style={{ margin: '4px 0 0', fontSize: 13, color: 'var(--ppt-fg-muted, #6b7280)' }}
                >
                  {tour.description}
                </p>
              )}
            </div>
            <button
              type="button"
              onClick={onClose}
              aria-label={t('common.close')}
              style={{
                background: 'transparent',
                border: 'none',
                cursor: 'pointer',
                padding: 4,
                color: 'var(--ppt-fg-muted, #6b7280)',
                flexShrink: 0,
              }}
            >
              <svg width="20" height="20" viewBox="0 0 20 20" fill="currentColor">
                <title>{t('common.close')}</title>
                <path
                  fillRule="evenodd"
                  d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
                  clipRule="evenodd"
                />
              </svg>
            </button>
          </div>

          {/* Progress bar */}
          <div style={{ marginTop: 12 }}>
            <div
              style={{
                display: 'flex',
                justifyContent: 'space-between',
                fontSize: 12,
                color: 'var(--ppt-fg-muted, #6b7280)',
                marginBottom: 4,
              }}
            >
              <span>
                {t('onboarding.stepsCompleted', {
                  completed: completedIds.length,
                  total: tour.steps.length,
                })}
              </span>
              <span style={{ fontWeight: 600 }}>{progressPct}%</span>
            </div>
            <div
              style={{
                height: 6,
                borderRadius: 3,
                background: 'var(--ppt-border-default, #e5e7eb)',
                overflow: 'hidden',
              }}
            >
              <div
                style={{
                  height: '100%',
                  width: `${progressPct}%`,
                  background: isCompleted
                    ? '#10b981'
                    : isSkipped
                      ? '#f59e0b'
                      : 'var(--ppt-brand-500, #3b82f6)',
                  borderRadius: 3,
                  transition: 'width 0.3s ease',
                }}
              />
            </div>
          </div>
        </div>

        {/* Body */}
        <div style={{ flex: 1, overflowY: 'auto', padding: '16px 24px' }}>
          {/* Completed banner */}
          {isCompleted && (
            <div
              style={{
                background: '#d1fae5',
                border: '1px solid #6ee7b7',
                borderRadius: 8,
                padding: '12px 16px',
                marginBottom: 16,
                display: 'flex',
                alignItems: 'center',
                gap: 10,
              }}
            >
              <svg
                width="24"
                height="24"
                viewBox="0 0 24 24"
                fill="none"
                stroke="#059669"
                strokeWidth="2"
              >
                <title>{t('onboarding.status.completed')}</title>
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
              <div>
                <p style={{ margin: 0, fontWeight: 600, fontSize: 14, color: '#065f46' }}>
                  {t('onboarding.tourCompleted')}
                </p>
                <p style={{ margin: 0, fontSize: 12, color: '#047857' }}>
                  {t('onboarding.tourCompletedDescription')}
                </p>
              </div>
            </div>
          )}

          {/* Skipped banner */}
          {isSkipped && (
            <div
              style={{
                background: '#fef3c7',
                border: '1px solid #fde68a',
                borderRadius: 8,
                padding: '12px 16px',
                marginBottom: 16,
                display: 'flex',
                alignItems: 'center',
                gap: 10,
              }}
            >
              <svg
                width="24"
                height="24"
                viewBox="0 0 24 24"
                fill="none"
                stroke="#d97706"
                strokeWidth="2"
              >
                <title>{t('onboarding.status.skipped')}</title>
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
                />
              </svg>
              <div>
                <p style={{ margin: 0, fontWeight: 600, fontSize: 14, color: '#92400e' }}>
                  {t('onboarding.tourSkipped')}
                </p>
                <p style={{ margin: 0, fontSize: 12, color: '#b45309' }}>
                  {t('onboarding.tourSkippedDescription')}
                </p>
              </div>
            </div>
          )}

          {/* Steps list */}
          <div>
            {tour.steps.map((step, index) => {
              const isStepCompleted = completedIds.includes(step.id);
              const isCurrent = index === currentStepIndex && !isCompleted && !isSkipped;
              const isLast = index === tour.steps.length - 1;

              return (
                <StepRow
                  key={step.id}
                  step={step}
                  index={index}
                  totalSteps={tour.steps.length}
                  isCompleted={isStepCompleted}
                  isCurrent={isCurrent}
                  isLast={isLast}
                  isMutating={isMutating}
                  onComplete={() => onCompleteStep(step.id, isLast)}
                />
              );
            })}
          </div>
        </div>

        {/* Footer */}
        <div
          style={{
            padding: '12px 24px',
            borderTop: '1px solid var(--ppt-border-default, #e5e7eb)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            gap: 8,
          }}
        >
          {isCompleted || isSkipped ? (
            <>
              <button
                type="button"
                disabled={isMutating}
                onClick={onResetTour}
                style={{
                  padding: '7px 14px',
                  fontSize: 13,
                  background: 'transparent',
                  color: 'var(--ppt-fg-secondary, #374151)',
                  border: '1px solid var(--ppt-border-default, #e5e7eb)',
                  borderRadius: 6,
                  cursor: isMutating ? 'not-allowed' : 'pointer',
                }}
              >
                {t('onboarding.restartTour')}
              </button>
              <button
                type="button"
                onClick={onClose}
                style={{
                  padding: '7px 14px',
                  fontSize: 13,
                  fontWeight: 600,
                  background: 'var(--ppt-brand-600, #2563eb)',
                  color: '#fff',
                  border: 'none',
                  borderRadius: 6,
                  cursor: 'pointer',
                }}
              >
                {t('common.close')}
              </button>
            </>
          ) : (
            <>
              <button
                type="button"
                disabled={isMutating}
                onClick={onSkipTour}
                style={{
                  padding: '7px 14px',
                  fontSize: 13,
                  background: 'transparent',
                  color: 'var(--ppt-fg-muted, #6b7280)',
                  border: 'none',
                  cursor: isMutating ? 'not-allowed' : 'pointer',
                }}
              >
                {t('onboarding.skipTour')}
              </button>
              <button
                type="button"
                onClick={onClose}
                style={{
                  padding: '7px 14px',
                  fontSize: 13,
                  background: 'transparent',
                  color: 'var(--ppt-fg-secondary, #374151)',
                  border: '1px solid var(--ppt-border-default, #e5e7eb)',
                  borderRadius: 6,
                  cursor: 'pointer',
                }}
              >
                {t('onboarding.continueOnboarding')}
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
