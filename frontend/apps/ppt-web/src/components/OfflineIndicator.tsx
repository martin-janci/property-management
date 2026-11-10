/**
 * Offline Indicator Component.
 *
 * Displays a banner when the user loses network connectivity.
 * Animates smoothly in and out.
 */

import { useTranslation } from 'react-i18next';
import { useNetworkStatus } from '../hooks/useNetworkStatus';
import './OfflineIndicator.css';

/**
 * Offline indicator banner.
 *
 * Shows when the browser is offline and hides when back online.
 * Includes a brief "reconnected" message when coming back online.
 *
 * @example
 * ```tsx
 * function App() {
 *   return (
 *     <>
 *       <OfflineIndicator />
 *       <MainContent />
 *     </>
 *   );
 * }
 * ```
 */
export function OfflineIndicator() {
  const { t } = useTranslation();
  const { isOnline, wasOffline } = useNetworkStatus();

  // Show offline banner
  if (!isOnline) {
    return (
      <div
        className="offline-indicator offline-indicator--offline"
        role="alert"
        aria-live="assertive"
      >
        <div className="offline-indicator__content">
          <svg
            className="offline-indicator__icon"
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
          >
            <line x1="1" y1="1" x2="23" y2="23" />
            <path d="M16.72 11.06A10.94 10.94 0 0 1 19 12.55" />
            <path d="M5 12.55a10.94 10.94 0 0 1 5.17-2.39" />
            <path d="M10.71 5.05A16 16 0 0 1 22.58 9" />
            <path d="M1.42 9a15.91 15.91 0 0 1 4.7-2.88" />
            <path d="M8.53 16.11a6 6 0 0 1 6.95 0" />
            <line x1="12" y1="20" x2="12.01" y2="20" />
          </svg>
          <span className="offline-indicator__text">{t('offlineIndicator.offline')}</span>
        </div>
      </div>
    );
  }

  // Show reconnected message briefly
  if (wasOffline) {
    return (
      <div
        className="offline-indicator offline-indicator--reconnected"
        role="status"
        aria-live="polite"
      >
        <div className="offline-indicator__content">
          <svg
            className="offline-indicator__icon"
            width="20"
            height="20"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
          >
            <path d="M5 12.55a11 11 0 0 1 14.08 0" />
            <path d="M1.42 9a16 16 0 0 1 21.16 0" />
            <path d="M8.53 16.11a6 6 0 0 1 6.95 0" />
            <line x1="12" y1="20" x2="12.01" y2="20" />
          </svg>
          <span className="offline-indicator__text">{t('offlineIndicator.reconnected')}</span>
        </div>
      </div>
    );
  }

  return null;
}

OfflineIndicator.displayName = 'OfflineIndicator';
