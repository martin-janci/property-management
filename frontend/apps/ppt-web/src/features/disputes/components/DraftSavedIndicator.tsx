/**
 * DraftSavedIndicator — auto-save status pill for the dispute-filing form.
 *
 * Epic 80, Story 80.2 (AC-4). Renders "Draft saved · 14s ago" from a saved-at
 * epoch timestamp and re-renders on a 10s tick so the relative time stays
 * fresh. Renders nothing until the first save has happened.
 */

import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

export interface DraftSavedIndicatorProps {
  /** Epoch millis of the last successful save, or null if never saved. */
  savedAt: number | null;
  className?: string;
}

/** Format the elapsed time since `savedAt` into a localized relative string. */
function useRelativeSaved(savedAt: number | null): string | null {
  const { t } = useTranslation();
  // Tick every 10s so the relative label stays current without a per-second
  // re-render storm.
  const [, force] = useState(0);
  useEffect(() => {
    if (savedAt == null) return;
    const id = setInterval(() => force((n) => n + 1), 10_000);
    return () => clearInterval(id);
  }, [savedAt]);

  if (savedAt == null) return null;

  const elapsedMs = Math.max(0, Date.now() - savedAt);
  const seconds = Math.floor(elapsedMs / 1000);

  let time: string;
  if (seconds < 5) {
    time = t('disputes.draftJustNow', 'just now');
  } else if (seconds < 60) {
    time = t('disputes.draftSecondsAgo', { count: seconds, defaultValue: '{{count}}s ago' });
  } else {
    const minutes = Math.floor(seconds / 60);
    time = t('disputes.draftMinutesAgo', { count: minutes, defaultValue: '{{count}}m ago' });
  }
  return t('disputes.draftSavedAgo', { time, defaultValue: 'Draft saved · {{time}}' });
}

export function DraftSavedIndicator({ savedAt, className }: DraftSavedIndicatorProps) {
  const label = useRelativeSaved(savedAt);
  if (!label) return null;

  return (
    <span
      role="status"
      aria-live="polite"
      className={[
        'inline-flex items-center gap-1.5 rounded-full bg-gray-100 px-2.5 py-1 text-xs text-gray-600',
        className ?? '',
      ].join(' ')}
    >
      <svg
        className="h-3.5 w-3.5 text-green-600"
        fill="none"
        viewBox="0 0 24 24"
        stroke="currentColor"
        strokeWidth={2.5}
        aria-hidden="true"
      >
        <path strokeLinecap="round" strokeLinejoin="round" d="M5 13l4 4L19 7" />
      </svg>
      {label}
    </span>
  );
}
