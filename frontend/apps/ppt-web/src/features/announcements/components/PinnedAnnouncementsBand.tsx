/**
 * PinnedAnnouncementsBand (Story 6.4)
 *
 * Sticky horizontal band shown at the very top of the Announcements list.
 * Only renders when there is at least one pinned published announcement.
 * Each chip in the band is clickable and navigates to the announcement's
 * detail page.
 */

import type { AnnouncementSummary } from '@ppt/api-client';
import './PinnedAnnouncementsBand.css';
import { useTranslation } from 'react-i18next';

interface PinnedAnnouncementsBandProps {
  announcements: AnnouncementSummary[];
  onView: (id: string) => void;
}

export function PinnedAnnouncementsBand({ announcements, onView }: PinnedAnnouncementsBandProps) {
  const { t } = useTranslation();
  if (announcements.length === 0) return null;

  return (
    <div className="pinned-band" role="region" aria-label={t('aria.pinnedAnnouncements')}>
      <div className="pinned-band__label">
        <svg
          className="pinned-band__pin-icon"
          width="14"
          height="14"
          fill="currentColor"
          viewBox="0 0 20 20"
          aria-hidden="true"
        >
          <path d="M9.828.722a.5.5 0 01.354 0l7 3A.5.5 0 0117.5 4v1.5a.5.5 0 01-.5.5h-1v4.5a.5.5 0 01-.5.5H13v5.5a.5.5 0 01-.5.5h-5a.5.5 0 01-.5-.5V11H4.5a.5.5 0 01-.5-.5V6h-1a.5.5 0 01-.5-.5V4a.5.5 0 01.328-.472l7-3z" />
        </svg>
        <span>Pinned</span>
      </div>

      <div className="pinned-band__chips" role="list">
        {announcements.map((ann) => (
          <button
            key={ann.id}
            type="button"
            className="pinned-band__chip"
            onClick={() => onView(ann.id)}
          >
            <span className="pinned-band__chip-title">{ann.title}</span>
            {ann.acknowledgmentRequired && (
              <span className="pinned-band__chip-ack" aria-label={t('aria.acknowledgmentRequired')}>
                !
              </span>
            )}
          </button>
        ))}
      </div>
    </div>
  );
}
