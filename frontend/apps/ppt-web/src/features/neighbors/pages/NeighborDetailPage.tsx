/**
 * NeighborDetailPage Component
 *
 * Page for viewing and editing neighbor details.
 * Presentational component - receives data as props.
 */

import type React from 'react';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { getActiveLocale } from '../../../lib/locale';
import type { NeighborView, PrivacySettings } from '../types';
import { DEFAULT_PRIVACY_SETTINGS, VISIBILITY_OPTIONS } from '../types';

export interface NeighborDetailPageProps {
  neighbor: NeighborView;
  privacySettings?: PrivacySettings;
  isCurrentUser?: boolean;
  isLoading?: boolean;
  error?: string | null;
  isSubmitting?: boolean;
  successMessage?: string | null;
  onSave?: (neighbor: Partial<NeighborView>) => void;
  onUpdatePrivacy?: (settings: PrivacySettings) => void;
  onRemove?: (neighbor: NeighborView) => void;
  onMessage?: (neighbor: NeighborView) => void;
  onBack?: () => void;
}

export const NeighborDetailPage: React.FC<NeighborDetailPageProps> = ({
  neighbor,
  privacySettings = DEFAULT_PRIVACY_SETTINGS,
  isCurrentUser = false,
  isLoading = false,
  error = null,
  isSubmitting = false,
  successMessage = null,
  onSave,
  onUpdatePrivacy,
  onRemove,
  onMessage,
  onBack,
}) => {
  const { t } = useTranslation();
  const [isEditing, setIsEditing] = useState(false);
  const [showRemoveConfirm, setShowRemoveConfirm] = useState(false);
  const [editedNeighbor, setEditedNeighbor] = useState<Partial<NeighborView>>({});
  const [editedPrivacy, setEditedPrivacy] = useState<PrivacySettings>(privacySettings);

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

  const handleEditToggle = useCallback(() => {
    if (isEditing) {
      // Discard changes
      setEditedNeighbor({});
      setEditedPrivacy(privacySettings);
    } else {
      // Start editing
      setEditedNeighbor({
        firstName: neighbor.firstName,
        lastName: neighbor.lastName,
        bio: neighbor.bio,
        email: neighbor.email,
        phone: neighbor.phone,
      });
      setEditedPrivacy(privacySettings);
    }
    setIsEditing(!isEditing);
  }, [isEditing, neighbor, privacySettings]);

  const handleInputChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
      const { name, value } = e.target;
      setEditedNeighbor((prev) => ({
        ...prev,
        [name]: value,
      }));
    },
    []
  );

  const handlePrivacyChange = useCallback(
    (field: keyof PrivacySettings, value: string | boolean) => {
      setEditedPrivacy((prev) => ({
        ...prev,
        [field]: value,
      }));
    },
    []
  );

  const handleSave = useCallback(() => {
    if (onSave) {
      onSave(editedNeighbor);
    }
    if (onUpdatePrivacy && isCurrentUser) {
      onUpdatePrivacy(editedPrivacy);
    }
    setIsEditing(false);
  }, [editedNeighbor, editedPrivacy, isCurrentUser, onSave, onUpdatePrivacy]);

  const handleRemoveClick = useCallback(() => {
    setShowRemoveConfirm(true);
  }, []);

  const handleRemoveConfirm = useCallback(() => {
    if (onRemove) {
      onRemove(neighbor);
    }
    setShowRemoveConfirm(false);
  }, [neighbor, onRemove]);

  const handleRemoveCancel = useCallback(() => {
    setShowRemoveConfirm(false);
  }, []);

  if (isLoading) {
    return (
      <div className="neighbor-detail-page">
        <div className="neighbor-detail-page-loading">
          <div className="neighbor-detail-page-spinner" />
          <p>{t('neighbors.detail.loading')}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="neighbor-detail-page">
      <div className="neighbor-detail-page-header">
        {onBack && (
          <button
            type="button"
            onClick={onBack}
            className="neighbor-detail-page-back"
            aria-label={t('common.back')}
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              className="neighbor-detail-page-back-icon"
            >
              <path d="M19 12H5M12 19l-7-7 7-7" />
            </svg>
            {t('neighbors.backToNeighbors')}
          </button>
        )}
      </div>

      {error && (
        <div className="neighbor-detail-page-error" role="alert">
          <p>{error}</p>
        </div>
      )}

      {successMessage && (
        <div className="neighbor-detail-page-success" role="status">
          <p>{successMessage}</p>
        </div>
      )}

      <div className="neighbor-detail-page-content">
        <div className="neighbor-detail-page-profile">
          <div className="neighbor-detail-page-avatar-section">
            {neighbor.avatarUrl ? (
              <img
                src={neighbor.avatarUrl}
                alt={displayName}
                className="neighbor-detail-page-avatar"
              />
            ) : (
              <div className="neighbor-detail-page-avatar-placeholder">
                <span>{initials || '?'}</span>
              </div>
            )}
            {isCurrentUser && isEditing && (
              <button type="button" className="neighbor-detail-page-avatar-edit">
                {t('neighbors.detail.changePhoto')}
              </button>
            )}
          </div>

          <div className="neighbor-detail-page-info">
            {isEditing ? (
              <div className="neighbor-detail-page-edit-form">
                <div className="neighbor-detail-page-field-row">
                  <div className="neighbor-detail-page-field">
                    <label htmlFor="firstName">{t('neighbors.detail.firstName')}</label>
                    <input
                      type="text"
                      id="firstName"
                      name="firstName"
                      value={editedNeighbor.firstName || ''}
                      onChange={handleInputChange}
                      className="neighbor-detail-page-input"
                      disabled={isSubmitting}
                    />
                  </div>
                  <div className="neighbor-detail-page-field">
                    <label htmlFor="lastName">{t('neighbors.detail.lastName')}</label>
                    <input
                      type="text"
                      id="lastName"
                      name="lastName"
                      value={editedNeighbor.lastName || ''}
                      onChange={handleInputChange}
                      className="neighbor-detail-page-input"
                      disabled={isSubmitting}
                    />
                  </div>
                </div>

                <div className="neighbor-detail-page-field">
                  <label htmlFor="bio">{t('neighbors.detail.bio')}</label>
                  <textarea
                    id="bio"
                    name="bio"
                    value={editedNeighbor.bio || ''}
                    onChange={handleInputChange}
                    className="neighbor-detail-page-textarea"
                    rows={3}
                    disabled={isSubmitting}
                  />
                </div>

                <div className="neighbor-detail-page-field-row">
                  <div className="neighbor-detail-page-field">
                    <label htmlFor="email">{t('neighbors.detail.email')}</label>
                    <input
                      type="email"
                      id="email"
                      name="email"
                      value={editedNeighbor.email || ''}
                      onChange={handleInputChange}
                      className="neighbor-detail-page-input"
                      disabled={isSubmitting}
                    />
                  </div>
                  <div className="neighbor-detail-page-field">
                    <label htmlFor="phone">{t('neighbors.detail.phone')}</label>
                    <input
                      type="tel"
                      id="phone"
                      name="phone"
                      value={editedNeighbor.phone || ''}
                      onChange={handleInputChange}
                      className="neighbor-detail-page-input"
                      disabled={isSubmitting}
                    />
                  </div>
                </div>
              </div>
            ) : (
              <>
                <h1 className="neighbor-detail-page-name">{displayName}</h1>
                {neighbor.unitNumber && (
                  <p className="neighbor-detail-page-unit">
                    {t('neighbors.unit')} {neighbor.unitNumber}
                    {neighbor.floor !== undefined && (
                      <span className="neighbor-detail-page-floor">
                        {' '}
                        ({t('neighbors.floor')} {neighbor.floor})
                      </span>
                    )}
                  </p>
                )}
                {neighbor.isOwner !== undefined && (
                  <span className="neighbor-detail-page-badge">
                    {neighbor.isOwner ? t('neighbors.owner') : t('neighbors.tenant')}
                  </span>
                )}
              </>
            )}
          </div>

          <div className="neighbor-detail-page-header-actions">
            {isCurrentUser && onSave && (
              <button
                type="button"
                onClick={isEditing ? handleSave : handleEditToggle}
                className="neighbor-detail-page-button primary"
                disabled={isSubmitting}
              >
                {isSubmitting
                  ? t('common.saving')
                  : isEditing
                    ? t('common.save')
                    : t('common.edit')}
              </button>
            )}
            {isCurrentUser && isEditing && (
              <button
                type="button"
                onClick={handleEditToggle}
                className="neighbor-detail-page-button secondary"
                disabled={isSubmitting}
              >
                {t('common.cancel')}
              </button>
            )}
          </div>
        </div>

        {!isEditing && (
          <div className="neighbor-detail-page-sections">
            {neighbor.bio && (
              <section className="neighbor-detail-page-section">
                <h2>{t('neighbors.detail.aboutSection')}</h2>
                <p className="neighbor-detail-page-bio">{neighbor.bio}</p>
              </section>
            )}

            <section className="neighbor-detail-page-section">
              <h2>{t('neighbors.detail.contactSection')}</h2>
              <div className="neighbor-detail-page-contact-list">
                {neighbor.email && (
                  <div className="neighbor-detail-page-contact-item">
                    <span className="neighbor-detail-page-contact-label">
                      {t('neighbors.email')}:
                    </span>
                    <a
                      href={`mailto:${neighbor.email}`}
                      className="neighbor-detail-page-contact-value link"
                    >
                      {neighbor.email}
                    </a>
                  </div>
                )}
                {neighbor.phone && (
                  <div className="neighbor-detail-page-contact-item">
                    <span className="neighbor-detail-page-contact-label">
                      {t('neighbors.phone')}:
                    </span>
                    <a
                      href={`tel:${neighbor.phone}`}
                      className="neighbor-detail-page-contact-value link"
                    >
                      {neighbor.phone}
                    </a>
                  </div>
                )}
                {neighbor.moveInDate && (
                  <div className="neighbor-detail-page-contact-item">
                    <span className="neighbor-detail-page-contact-label">
                      {t('neighbors.moveInDate')}:
                    </span>
                    <span className="neighbor-detail-page-contact-value">
                      {new Date(neighbor.moveInDate).toLocaleDateString(getActiveLocale())}
                    </span>
                  </div>
                )}
                {!neighbor.email && !neighbor.phone && !neighbor.moveInDate && (
                  <p className="neighbor-detail-page-no-contact">
                    {t('neighbors.detail.noContactInfo')}
                  </p>
                )}
              </div>
            </section>

            {isCurrentUser && onUpdatePrivacy && (
              <section className="neighbor-detail-page-section">
                <h2>{t('neighbors.detail.privacySection')}</h2>
                <p className="neighbor-detail-page-section-description">
                  {t('neighbors.detail.privacyDescription')}
                </p>
                <div className="neighbor-detail-page-privacy-grid">
                  {(Object.keys(privacySettings) as Array<keyof PrivacySettings>).map((key) => {
                    if (key === 'listedInDirectory') return null;
                    return (
                      <div key={key} className="neighbor-detail-page-privacy-item">
                        <span className="neighbor-detail-page-privacy-label">
                          {t(`neighbors.privacy.${key}`)}
                        </span>
                        <span className="neighbor-detail-page-privacy-value">
                          {t(
                            VISIBILITY_OPTIONS.find((opt) => opt.value === privacySettings[key])
                              ?.labelKey || ''
                          )}
                        </span>
                      </div>
                    );
                  })}
                </div>
              </section>
            )}
          </div>
        )}

        {isCurrentUser && isEditing && onUpdatePrivacy && (
          <section className="neighbor-detail-page-section">
            <h2>{t('neighbors.detail.privacySection')}</h2>
            <p className="neighbor-detail-page-section-description">
              {t('neighbors.detail.privacyDescription')}
            </p>
            <div className="neighbor-detail-page-privacy-edit">
              {(Object.keys(editedPrivacy) as Array<keyof PrivacySettings>).map((key) => {
                if (key === 'listedInDirectory') {
                  return (
                    <div key={key} className="neighbor-detail-page-privacy-field checkbox">
                      <label>
                        <input
                          type="checkbox"
                          checked={editedPrivacy.listedInDirectory}
                          onChange={(e) =>
                            handlePrivacyChange('listedInDirectory', e.target.checked)
                          }
                          disabled={isSubmitting}
                        />
                        <span>{t('neighbors.privacy.listedInDirectory')}</span>
                      </label>
                    </div>
                  );
                }
                return (
                  <div key={key} className="neighbor-detail-page-privacy-field">
                    <label htmlFor={`privacy-${key}`}>{t(`neighbors.privacy.${key}`)}</label>
                    <select
                      id={`privacy-${key}`}
                      value={editedPrivacy[key] as string}
                      onChange={(e) => handlePrivacyChange(key, e.target.value)}
                      disabled={isSubmitting}
                    >
                      {VISIBILITY_OPTIONS.map((option) => (
                        <option key={option.value} value={option.value}>
                          {t(option.labelKey)}
                        </option>
                      ))}
                    </select>
                  </div>
                );
              })}
            </div>
          </section>
        )}

        <div className="neighbor-detail-page-actions">
          {!isCurrentUser && onMessage && neighbor.email && (
            <button
              type="button"
              onClick={() => onMessage(neighbor)}
              className="neighbor-detail-page-button primary"
            >
              {t('neighbors.detail.messageNeighbor')}
            </button>
          )}
          {!isCurrentUser && onRemove && (
            <button
              type="button"
              onClick={handleRemoveClick}
              className="neighbor-detail-page-button danger"
            >
              {t('neighbors.detail.removeFromBuilding')}
            </button>
          )}
        </div>
      </div>

      {showRemoveConfirm && (
        <div className="neighbor-detail-page-modal-overlay">
          <div className="neighbor-detail-page-modal">
            <h2>{t('neighbors.detail.removeConfirmTitle')}</h2>
            <p>{t('neighbors.detail.removeConfirmMessage', { name: displayName })}</p>
            <div className="neighbor-detail-page-modal-actions">
              <button
                type="button"
                onClick={handleRemoveCancel}
                className="neighbor-detail-page-button secondary"
              >
                {t('common.cancel')}
              </button>
              <button
                type="button"
                onClick={handleRemoveConfirm}
                className="neighbor-detail-page-button danger"
              >
                {t('neighbors.detail.confirmRemove')}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

NeighborDetailPage.displayName = 'NeighborDetailPage';
