/**
 * OnboardingToursPage — `/platform/onboarding`
 *
 * Platform-admin view for managing onboarding tour configurations
 * (Epic 10B, Story 10B.6).
 *
 * Displays the list of active onboarding tours fetched from the onboarding
 * config API, with interactive step-by-step tour overlay for previewing each
 * tour as an admin.
 *
 * Backend endpoints consumed:
 *   GET  /api/v1/onboarding/status
 *   GET  /api/v1/onboarding/tours
 *   POST /api/v1/onboarding/tours/{tour_id}/start
 *   POST /api/v1/onboarding/tours/{tour_id}/steps/{step_id}/complete
 *   POST /api/v1/onboarding/tours/{tour_id}/complete
 *   POST /api/v1/onboarding/tours/{tour_id}/skip
 *   POST /api/v1/onboarding/tours/{tour_id}/reset
 */

import {
  useCompleteOnboardingStep,
  useCompleteOnboardingTour,
  useOnboardingStatus,
  useResetOnboardingTour,
  useSkipOnboardingTour,
  useStartOnboardingTour,
} from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useToast } from '../components/Toast';
import {
  trackStepCompleted,
  trackTourCompleted,
  trackTourReset,
  trackTourSkipped,
  trackTourStarted,
} from '../features/onboarding/analytics';
import { TourOverlay } from '../features/onboarding/TourOverlay';
import type {
  OnboardingTour,
  TourWithProgress,
  UserOnboardingProgress,
} from '../features/onboarding/types';
import { getTourDisplayStatus } from '../features/onboarding/types';

// ─── Status badge ──────────────────────────────────────────────────────────

type DisplayStatus = 'not_started' | 'in_progress' | 'completed' | 'skipped';

const STATUS_STYLE: Record<DisplayStatus, { bg: string; fg: string; label: string }> = {
  not_started: { bg: '#f3f4f6', fg: '#374151', label: 'Not started' },
  in_progress: { bg: '#dbeafe', fg: '#1d4ed8', label: 'In progress' },
  completed: { bg: '#d1fae5', fg: '#065f46', label: 'Completed' },
  skipped: { bg: '#fef3c7', fg: '#92400e', label: 'Skipped' },
};

function StatusBadge({ status }: { status: DisplayStatus }) {
  const { t } = useTranslation();
  const { bg, fg } = STATUS_STYLE[status];
  return (
    <span
      style={{
        display: 'inline-block',
        padding: '2px 10px',
        borderRadius: 9999,
        fontSize: 11,
        fontWeight: 600,
        background: bg,
        color: fg,
        textTransform: 'uppercase',
        letterSpacing: '0.04em',
      }}
    >
      {t(`onboarding.status.${status}`)}
    </span>
  );
}

// ─── Tour card ──────────────────────────────────────────────────────────────

interface TourCardProps {
  tourWithProgress: TourWithProgress;
  onOpen: (tour: OnboardingTour, progress: UserOnboardingProgress | null) => void;
}

function TourCard({ tourWithProgress, onOpen }: TourCardProps) {
  const { t } = useTranslation();
  const { tour, progress } = tourWithProgress;
  const displayStatus = getTourDisplayStatus(progress);
  const completedCount = progress?.completed_steps.length ?? 0;

  return (
    <div
      style={{
        padding: 20,
        borderRadius: 10,
        border: '1px solid var(--ppt-border-default, #e5e7eb)',
        background: 'var(--ppt-bg-surface, #fff)',
        display: 'flex',
        flexDirection: 'column',
        gap: 10,
      }}
    >
      {/* Card header */}
      <div
        style={{
          display: 'flex',
          alignItems: 'flex-start',
          justifyContent: 'space-between',
          gap: 8,
        }}
      >
        <div style={{ flex: 1 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 4 }}>
            <h3
              style={{
                margin: 0,
                fontSize: 15,
                fontWeight: 700,
                color: 'var(--ppt-fg-primary, #111827)',
              }}
            >
              {tour.name}
            </h3>
            <StatusBadge status={displayStatus} />
          </div>
          {tour.description && (
            <p
              style={{
                margin: 0,
                fontSize: 13,
                color: 'var(--ppt-fg-muted, #6b7280)',
                lineHeight: 1.4,
              }}
            >
              {tour.description}
            </p>
          )}
        </div>
      </div>

      {/* Meta */}
      <div
        style={{ display: 'flex', gap: 16, fontSize: 12, color: 'var(--ppt-fg-muted, #6b7280)' }}
      >
        <span>
          <strong style={{ color: 'var(--ppt-fg-secondary, #374151)' }}>{tour.steps.length}</strong>{' '}
          {t('onboarding.stepsLabel')}
        </span>
        <span>
          <strong style={{ color: 'var(--ppt-fg-secondary, #374151)' }}>{completedCount}</strong>{' '}
          {t('onboarding.completedLabel')}
        </span>
        {tour.target_roles && tour.target_roles.length > 0 && (
          <span>
            {t('onboarding.targetRoles')}: {tour.target_roles.join(', ')}
          </span>
        )}
      </div>

      {/* Progress bar */}
      {tour.steps.length > 0 && (
        <div
          style={{
            height: 4,
            borderRadius: 2,
            background: 'var(--ppt-border-default, #e5e7eb)',
            overflow: 'hidden',
          }}
        >
          <div
            style={{
              height: '100%',
              width: `${Math.round((completedCount / tour.steps.length) * 100)}%`,
              background:
                displayStatus === 'completed'
                  ? '#10b981'
                  : displayStatus === 'skipped'
                    ? '#f59e0b'
                    : 'var(--ppt-brand-500, #3b82f6)',
              borderRadius: 2,
              transition: 'width 0.3s ease',
            }}
          />
        </div>
      )}

      {/* Action button */}
      <div>
        <button
          type="button"
          onClick={() => onOpen(tour, progress ?? null)}
          style={{
            padding: '7px 16px',
            fontSize: 13,
            fontWeight: 600,
            background:
              displayStatus === 'not_started' ? 'var(--ppt-brand-600, #2563eb)' : 'transparent',
            color: displayStatus === 'not_started' ? '#fff' : 'var(--ppt-brand-700, #1d4ed8)',
            border:
              displayStatus === 'not_started' ? 'none' : '1px solid var(--ppt-brand-300, #93c5fd)',
            borderRadius: 6,
            cursor: 'pointer',
          }}
        >
          {displayStatus === 'not_started'
            ? t('onboarding.startTour')
            : displayStatus === 'in_progress'
              ? t('onboarding.resumeTour')
              : t('onboarding.viewTour')}
        </button>
      </div>
    </div>
  );
}

// ─── Page ──────────────────────────────────────────────────────────────────

export default function OnboardingToursPage() {
  const { t } = useTranslation();
  const { showToast } = useToast();

  // Active overlay state
  const [activeTour, setActiveTour] = useState<{
    tour: OnboardingTour;
    progress: UserOnboardingProgress | null;
  } | null>(null);

  // Data
  const { data, isLoading, isError, error, refetch } = useOnboardingStatus();

  // Mutations
  const startMutation = useStartOnboardingTour();
  const completeStepMutation = useCompleteOnboardingStep();
  const completeTourMutation = useCompleteOnboardingTour();
  const skipMutation = useSkipOnboardingTour();
  const resetMutation = useResetOnboardingTour();

  const isMutating =
    startMutation.isPending ||
    completeStepMutation.isPending ||
    completeTourMutation.isPending ||
    skipMutation.isPending ||
    resetMutation.isPending;

  // Sync overlay progress with latest server data
  const syncedActiveTour = activeTour
    ? {
        tour: activeTour.tour,
        progress:
          data?.tours.find((t) => t.tour.tour_id === activeTour.tour.tour_id)?.progress ??
          activeTour.progress,
      }
    : null;

  // ── Handlers ────────────────────────────────────────────────────────────

  const handleOpenTour = (tour: OnboardingTour, progress: UserOnboardingProgress | null) => {
    setActiveTour({ tour, progress });
    // Auto-start if not yet started
    if (!progress) {
      startMutation.mutate(tour.tour_id, {
        onSuccess: () => {
          trackTourStarted({ tourId: tour.tour_id, tourName: tour.name });
        },
        onError: (err) => {
          showToast({
            title: t('onboarding.toast.startFailedTitle'),
            message: err instanceof Error ? err.message : t('common.unknown'),
            type: 'error',
          });
        },
      });
    }
  };

  const handleCompleteStep = (stepId: string, isLast: boolean) => {
    if (!activeTour) return;
    const tourId = activeTour.tour.tour_id;
    const tourName = activeTour.tour.name;
    completeStepMutation.mutate(
      { tourId, stepId },
      {
        onSuccess: () => {
          trackStepCompleted({ tourId, tourName }, stepId, isLast);
          // Chain `completeOnboardingTour` only after the final step POST
          // has succeeded — keeps the two requests sequenced instead of
          // racing against the backend (issue #702 finding #1).
          if (!isLast) return;
          completeTourMutation.mutate(tourId, {
            onSuccess: () => {
              trackTourCompleted({ tourId, tourName });
              showToast({
                title: t('onboarding.toast.completedTitle'),
                message: t('onboarding.toast.completedMessage', { name: tourName }),
                type: 'success',
              });
            },
            onError: (err) => {
              showToast({
                title: t('onboarding.toast.completeFailedTitle'),
                message: err instanceof Error ? err.message : t('common.unknown'),
                type: 'error',
              });
            },
          });
        },
        onError: (err) => {
          showToast({
            title: t('onboarding.toast.stepFailedTitle'),
            message: err instanceof Error ? err.message : t('common.unknown'),
            type: 'error',
          });
        },
      }
    );
  };

  const handleSkipTour = () => {
    if (!activeTour) return;
    skipMutation.mutate(activeTour.tour.tour_id, {
      onSuccess: () => {
        trackTourSkipped({ tourId: activeTour.tour.tour_id, tourName: activeTour.tour.name });
        showToast({
          title: t('onboarding.toast.skippedTitle'),
          message: t('onboarding.toast.skippedMessage', { name: activeTour.tour.name }),
          type: 'info',
        });
        setActiveTour(null);
      },
      onError: (err) => {
        showToast({
          title: t('onboarding.toast.skipFailedTitle'),
          message: err instanceof Error ? err.message : t('common.unknown'),
          type: 'error',
        });
      },
    });
  };

  const handleResetTour = () => {
    if (!activeTour) return;
    resetMutation.mutate(activeTour.tour.tour_id, {
      onSuccess: () => {
        trackTourReset({ tourId: activeTour.tour.tour_id, tourName: activeTour.tour.name });
        showToast({
          title: t('onboarding.toast.resetTitle'),
          message: t('onboarding.toast.resetMessage', { name: activeTour.tour.name }),
          type: 'info',
        });
      },
      onError: (err) => {
        showToast({
          title: t('onboarding.toast.resetFailedTitle'),
          message: err instanceof Error ? err.message : t('common.unknown'),
          type: 'error',
        });
      },
    });
  };

  // ── Render ───────────────────────────────────────────────────────────────

  return (
    <div style={{ padding: 32, maxWidth: 900, margin: '0 auto' }}>
      {/* Page header */}
      <div style={{ marginBottom: 28 }}>
        <h1
          style={{
            margin: 0,
            fontSize: 24,
            fontWeight: 700,
            color: 'var(--ppt-fg-primary, #111827)',
          }}
        >
          {t('admin.onboarding.title')}
        </h1>
        <p style={{ margin: '6px 0 0', fontSize: 14, color: 'var(--ppt-fg-muted, #6b7280)' }}>
          {t('admin.onboarding.subtitle')}
        </p>

        {/* Overall needs-onboarding chip */}
        {!isLoading && data && (
          <div style={{ marginTop: 12 }}>
            <span
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: 6,
                padding: '4px 12px',
                borderRadius: 9999,
                fontSize: 12,
                fontWeight: 600,
                background: data.needs_onboarding ? '#fef3c7' : '#d1fae5',
                color: data.needs_onboarding ? '#92400e' : '#065f46',
                border: `1px solid ${data.needs_onboarding ? '#fde68a' : '#6ee7b7'}`,
              }}
            >
              {data.needs_onboarding
                ? t('admin.onboarding.pendingOnboarding')
                : t('admin.onboarding.allComplete')}
            </span>
          </div>
        )}
      </div>

      {/* Loading */}
      {isLoading && (
        <div
          style={{ textAlign: 'center', padding: '48px 0', color: 'var(--ppt-fg-muted, #6b7280)' }}
        >
          {t('admin.common.loading')}
        </div>
      )}

      {/* Error */}
      {isError && (
        <div
          style={{
            padding: '16px 20px',
            borderRadius: 8,
            background: '#fee2e2',
            border: '1px solid #fca5a5',
            color: '#991b1b',
            display: 'flex',
            alignItems: 'center',
            gap: 12,
          }}
        >
          <span style={{ flex: 1 }}>
            {t('admin.onboarding.loadError', {
              message: error instanceof Error ? error.message : String(error),
            })}
          </span>
          <button
            type="button"
            onClick={() => {
              void refetch();
            }}
            style={{
              padding: '5px 12px',
              fontSize: 13,
              background: '#fff',
              border: '1px solid #fca5a5',
              borderRadius: 6,
              cursor: 'pointer',
              color: '#991b1b',
            }}
          >
            {t('admin.common.retry')}
          </button>
        </div>
      )}

      {/* Empty state */}
      {!isLoading && !isError && data && data.tours.length === 0 && (
        <div
          style={{
            textAlign: 'center',
            padding: '64px 0',
            color: 'var(--ppt-fg-muted, #6b7280)',
          }}
        >
          <svg
            width="48"
            height="48"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            style={{ margin: '0 auto 12px', display: 'block', opacity: 0.4 }}
          >
            <title>{t('admin.onboarding.noTours')}</title>
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"
            />
          </svg>
          <p style={{ margin: 0, fontSize: 14 }}>{t('admin.onboarding.noTours')}</p>
        </div>
      )}

      {/* Tours grid */}
      {!isLoading && !isError && data && data.tours.length > 0 && (
        <div
          style={{
            display: 'grid',
            gridTemplateColumns: 'repeat(auto-fill, minmax(360px, 1fr))',
            gap: 16,
          }}
        >
          {data.tours.map((twp) => (
            <TourCard key={twp.tour.tour_id} tourWithProgress={twp} onOpen={handleOpenTour} />
          ))}
        </div>
      )}

      {/* Tour overlay */}
      {syncedActiveTour && (
        <TourOverlay
          tour={syncedActiveTour.tour}
          progress={syncedActiveTour.progress}
          isMutating={isMutating}
          onCompleteStep={handleCompleteStep}
          onSkipTour={handleSkipTour}
          onResetTour={handleResetTour}
          onClose={() => setActiveTour(null)}
        />
      )}
    </div>
  );
}
