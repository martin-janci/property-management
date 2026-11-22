/**
 * TourCard component - displays a tour summary with progress.
 * Epic 10B: User Onboarding (Story 10B.6)
 */

import { useTranslation } from 'react-i18next';
import { getActiveLocale } from '../../../lib/locale';
import type { OnboardingTour, UserOnboardingProgress } from '../types';
import { calculateTourProgress, getTourDisplayStatus } from '../types';

interface TourCardProps {
  tour: OnboardingTour;
  progress: UserOnboardingProgress | null;
  onStart?: (tourId: string) => void;
  onResume?: (tourId: string) => void;
  onView?: (tourId: string) => void;
  onReset?: (tourId: string) => void;
}

const statusColors = {
  not_started: 'bg-gray-100 text-gray-800',
  in_progress: 'bg-blue-100 text-blue-800',
  completed: 'bg-green-100 text-green-800',
  skipped: 'bg-yellow-100 text-yellow-800',
};

export function TourCard({ tour, progress, onStart, onResume, onView, onReset }: TourCardProps) {
  const { t } = useTranslation();

  const displayStatus = getTourDisplayStatus(progress);
  const progressPercent = calculateTourProgress(tour, progress);
  const completedSteps = progress?.completedSteps.length ?? 0;
  const totalSteps = tour.steps.length;

  const handlePrimaryAction = () => {
    if (displayStatus === 'not_started') {
      onStart?.(tour.tourId);
    } else if (displayStatus === 'in_progress') {
      onResume?.(tour.tourId);
    } else {
      onView?.(tour.tourId);
    }
  };

  return (
    <div className="bg-white rounded-lg shadow p-4 hover:shadow-md transition-shadow">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <div className="flex items-center gap-2">
            <h3 className="text-lg font-semibold text-gray-900">{tour.name}</h3>
            <span
              className={`px-2 py-0.5 text-xs font-medium rounded ${statusColors[displayStatus]}`}
            >
              {t(`onboarding.status.${displayStatus}`)}
            </span>
          </div>

          {tour.description && <p className="mt-1 text-sm text-gray-600">{tour.description}</p>}

          {/* Progress bar */}
          <div className="mt-3">
            <div className="flex items-center justify-between text-xs text-gray-500 mb-1">
              <span>
                {t('onboarding.stepsCompleted', { completed: completedSteps, total: totalSteps })}
              </span>
              <span>{progressPercent}%</span>
            </div>
            <div className="w-full bg-gray-200 rounded-full h-2">
              <div
                className={`h-2 rounded-full transition-all duration-300 ${
                  displayStatus === 'completed'
                    ? 'bg-green-500'
                    : displayStatus === 'skipped'
                      ? 'bg-yellow-500'
                      : 'bg-blue-500'
                }`}
                style={{ width: `${progressPercent}%` }}
              />
            </div>
          </div>

          {/* Meta info */}
          {progress?.startedAt && (
            <p className="mt-2 text-xs text-gray-400">
              {t('onboarding.startedAt', {
                date: new Date(progress.startedAt).toLocaleDateString(getActiveLocale()),
              })}
            </p>
          )}
          {progress?.completedAt && (
            <p className="text-xs text-gray-400">
              {t('onboarding.completedAt', {
                date: new Date(progress.completedAt).toLocaleDateString(getActiveLocale()),
              })}
            </p>
          )}
        </div>
      </div>

      {/* Actions */}
      <div className="mt-4 flex items-center gap-2 border-t pt-3">
        <button
          type="button"
          onClick={handlePrimaryAction}
          className={`text-sm font-medium ${
            displayStatus === 'not_started'
              ? 'text-blue-600 hover:text-blue-800'
              : displayStatus === 'in_progress'
                ? 'text-green-600 hover:text-green-800'
                : 'text-gray-600 hover:text-gray-800'
          }`}
        >
          {displayStatus === 'not_started'
            ? t('onboarding.startTour')
            : displayStatus === 'in_progress'
              ? t('onboarding.resumeTour')
              : t('common.view')}
        </button>

        {(displayStatus === 'completed' || displayStatus === 'skipped') && onReset && (
          <button
            type="button"
            onClick={() => onReset(tour.tourId)}
            className="text-sm text-gray-500 hover:text-gray-700"
          >
            {t('onboarding.restartTour')}
          </button>
        )}
      </div>
    </div>
  );
}
