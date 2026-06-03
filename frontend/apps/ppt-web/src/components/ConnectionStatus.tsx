/**
 * Connection Status Indicator for ppt-web (Story 79.4)
 *
 * Displays WebSocket connection status with:
 * - Green dot for connected
 * - Yellow dot for connecting
 * - Red dot for disconnected/error
 * - Tooltip with details
 */

import type React from 'react';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useWebSocketState } from '../hooks/useWebSocket';
import './ConnectionStatus.css';

/**
 * Props for the ConnectionStatus component.
 */
export interface ConnectionStatusProps {
  /**
   * Whether to show the status label text.
   * @default false
   */
  showLabel?: boolean;

  /**
   * Custom class name for styling.
   */
  className?: string;

  /**
   * Aria label for the status indicator.
   * Defaults to the translated `connectionStatus.ariaLabel` string.
   */
  ariaLabel?: string;
}

/**
 * Connection status indicator component.
 *
 * Shows a colored dot indicating WebSocket connection state:
 * - Green: Connected
 * - Yellow: Connecting
 * - Red: Disconnected or Error
 *
 * Includes a tooltip with more details on hover.
 */
export function ConnectionStatus({
  showLabel = false,
  className = '',
  ariaLabel,
}: ConnectionStatusProps): React.ReactElement {
  const { t } = useTranslation();
  const { isConnected, isConnecting, error, reconnect } = useWebSocketState();
  const [showTooltip, setShowTooltip] = useState(false);

  const resolvedAriaLabel = ariaLabel ?? t('connectionStatus.ariaLabel');

  const status = isConnected ? 'connected' : isConnecting ? 'connecting' : 'disconnected';

  const statusLabel = isConnected
    ? t('connectionStatus.connected')
    : isConnecting
      ? t('connectionStatus.connecting')
      : error
        ? t('connectionStatus.error')
        : t('connectionStatus.disconnected');

  const statusDescription = isConnected
    ? t('connectionStatus.connectedDescription')
    : isConnecting
      ? t('connectionStatus.connectingDescription')
      : error
        ? t('connectionStatus.errorDescription', { message: error.message })
        : t('connectionStatus.disconnectedDescription');

  const handleMouseEnter = useCallback(() => {
    setShowTooltip(true);
  }, []);

  const handleMouseLeave = useCallback(() => {
    setShowTooltip(false);
  }, []);

  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent) => {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        if (!isConnected && !isConnecting) {
          reconnect();
        }
      }
    },
    [isConnected, isConnecting, reconnect]
  );

  const handleClick = useCallback(() => {
    if (!isConnected && !isConnecting) {
      reconnect();
    }
  }, [isConnected, isConnecting, reconnect]);

  return (
    <div
      className={`connection-status ${className}`.trim()}
      onMouseEnter={handleMouseEnter}
      onMouseLeave={handleMouseLeave}
      onClick={handleClick}
      onKeyDown={handleKeyDown}
      role="status"
      aria-label={`${resolvedAriaLabel}: ${statusLabel}`}
      tabIndex={!isConnected && !isConnecting ? 0 : -1}
    >
      <span
        className={`connection-status__dot connection-status__dot--${status}`}
        aria-hidden="true"
      />

      {showLabel && <span className="connection-status__label">{statusLabel}</span>}

      {showTooltip && (
        <div className="connection-status__tooltip" role="tooltip">
          <div className="connection-status__tooltip-header">{statusLabel}</div>
          <div className="connection-status__tooltip-description">{statusDescription}</div>
          {!isConnected && !isConnecting && (
            <div className="connection-status__tooltip-action">
              {t('connectionStatus.clickToReconnect')}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

ConnectionStatus.displayName = 'ConnectionStatus';
