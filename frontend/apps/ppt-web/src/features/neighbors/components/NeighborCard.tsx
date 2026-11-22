/**
 * NeighborCard Component
 *
 * Displays neighbor information in a card format, respecting privacy settings.
 * Only shows fields that the neighbor has made visible.
 */

import type React from 'react';
import { useTranslation } from 'react-i18next';
import { getActiveLocale } from '../../../lib/locale';
import type { NeighborView } from '../types';

export interface NeighborCardProps {
  neighbor: NeighborView;
  onContact?: (neighbor: NeighborView) => void;
  onViewProfile?: (neighbor: NeighborView) => void;
  showActions?: boolean;
}

export const NeighborCard: React.FC<NeighborCardProps> = ({
  neighbor,
  onContact,
  onViewProfile,
  showActions = true,
}) => {
  const { t } = useTranslation();

  const displayName =
    neighbor.displayName ||
    [neighbor.firstName, neighbor.lastName].filter(Boolean).join(' ') ||
    t('neighbors.anonymous');

  const initials = displayName
    .split(' ')
    .map((n) => n[0])
    .join('')
    .toUpperCase()
    .slice(0, 2);

  return (
    <div className="neighbor-card">
      <div className="neighbor-card-header">
        {neighbor.avatarUrl ? (
          <img src={neighbor.avatarUrl} alt={displayName} className="neighbor-card-avatar" />
        ) : (
          <div className="neighbor-card-avatar-placeholder">
            <span>{initials || '?'}</span>
          </div>
        )}
        <div className="neighbor-card-info">
          <h3 className="neighbor-card-name">{displayName}</h3>
          {neighbor.unitNumber && (
            <span className="neighbor-card-unit">
              {t('neighbors.unit')} {neighbor.unitNumber}
              {neighbor.floor !== undefined && (
                <span className="neighbor-card-floor">
                  {' '}
                  ({t('neighbors.floor')} {neighbor.floor})
                </span>
              )}
            </span>
          )}
          {neighbor.isOwner !== undefined && (
            <span className="neighbor-card-badge">
              {neighbor.isOwner ? t('neighbors.owner') : t('neighbors.tenant')}
            </span>
          )}
        </div>
      </div>

      {neighbor.bio && (
        <div className="neighbor-card-bio">
          <p>{neighbor.bio}</p>
        </div>
      )}

      <div className="neighbor-card-details">
        {neighbor.email && (
          <div className="neighbor-card-field">
            <span className="neighbor-card-label">{t('neighbors.email')}:</span>
            <a href={`mailto:${neighbor.email}`} className="neighbor-card-value link">
              {neighbor.email}
            </a>
          </div>
        )}

        {neighbor.phone && (
          <div className="neighbor-card-field">
            <span className="neighbor-card-label">{t('neighbors.phone')}:</span>
            <a href={`tel:${neighbor.phone}`} className="neighbor-card-value link">
              {neighbor.phone}
            </a>
          </div>
        )}

        {neighbor.moveInDate && (
          <div className="neighbor-card-field">
            <span className="neighbor-card-label">{t('neighbors.moveInDate')}:</span>
            <span className="neighbor-card-value">
              {new Date(neighbor.moveInDate).toLocaleDateString(getActiveLocale())}
            </span>
          </div>
        )}
      </div>

      {showActions && (onContact || onViewProfile) && (
        <div className="neighbor-card-actions">
          {onViewProfile && (
            <button
              type="button"
              onClick={() => onViewProfile(neighbor)}
              className="neighbor-card-button secondary"
              aria-label={t('neighbors.viewProfile', { name: displayName })}
            >
              {t('neighbors.viewProfileButton')}
            </button>
          )}
          {onContact && neighbor.email && (
            <button
              type="button"
              onClick={() => onContact(neighbor)}
              className="neighbor-card-button primary"
              aria-label={t('neighbors.contactNeighbor', { name: displayName })}
            >
              {t('neighbors.contactButton')}
            </button>
          )}
        </div>
      )}
    </div>
  );
};

NeighborCard.displayName = 'NeighborCard';
