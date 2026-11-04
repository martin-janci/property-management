/**
 * Emergency Contact Card component for Epic 62.
 *
 * Displays an emergency contact in a card format with action buttons.
 */

import type { EmergencyContact } from '@ppt/api-client';
import { CONTACT_TYPE_LABELS } from '@ppt/api-client';
import type React from 'react';
import { useTranslation } from 'react-i18next';

export interface EmergencyContactCardProps {
  contact: EmergencyContact;
  onEdit?: (contact: EmergencyContact) => void;
  onDelete?: (contact: EmergencyContact) => void;
  showActions?: boolean;
}

export const EmergencyContactCard: React.FC<EmergencyContactCardProps> = ({
  contact,
  onEdit,
  onDelete,
  showActions = true,
}) => {
  const { t } = useTranslation();
  const englishTypeLabel =
    CONTACT_TYPE_LABELS[contact.contact_type as keyof typeof CONTACT_TYPE_LABELS];
  const contactTypeLabel = englishTypeLabel
    ? t(`emergency.contactTypes.${contact.contact_type}`, { defaultValue: englishTypeLabel })
    : contact.role;

  return (
    <div className="emergency-contact-card">
      <div className="emergency-contact-card-header">
        <div>
          <h3 className="emergency-contact-card-name">{contact.name}</h3>
          <span className="emergency-contact-card-type">{contactTypeLabel}</span>
        </div>
        {!contact.is_active && (
          <span className="emergency-contact-card-badge">{t('emergency.card.inactive')}</span>
        )}
      </div>

      <div className="emergency-contact-card-body">
        <div className="emergency-contact-card-field">
          <span className="emergency-contact-card-label">{t('emergency.card.role')}</span>
          <span className="emergency-contact-card-value">{contact.role}</span>
        </div>

        {contact.phone && (
          <div className="emergency-contact-card-field">
            <span className="emergency-contact-card-label">{t('emergency.card.phone')}</span>
            <a href={`tel:${contact.phone}`} className="emergency-contact-card-value link">
              {contact.phone}
            </a>
          </div>
        )}

        {contact.phone_secondary && (
          <div className="emergency-contact-card-field">
            <span className="emergency-contact-card-label">
              {t('emergency.card.phoneSecondary')}
            </span>
            <a
              href={`tel:${contact.phone_secondary}`}
              className="emergency-contact-card-value link"
            >
              {contact.phone_secondary}
            </a>
          </div>
        )}

        {contact.email && (
          <div className="emergency-contact-card-field">
            <span className="emergency-contact-card-label">{t('emergency.card.email')}</span>
            <a href={`mailto:${contact.email}`} className="emergency-contact-card-value link">
              {contact.email}
            </a>
          </div>
        )}

        {contact.address && (
          <div className="emergency-contact-card-field">
            <span className="emergency-contact-card-label">{t('emergency.card.address')}</span>
            <span className="emergency-contact-card-value">{contact.address}</span>
          </div>
        )}

        {contact.available_hours && (
          <div className="emergency-contact-card-field">
            <span className="emergency-contact-card-label">
              {t('emergency.card.availableHours')}
            </span>
            <span className="emergency-contact-card-value">{contact.available_hours}</span>
          </div>
        )}

        {contact.notes && (
          <div className="emergency-contact-card-field">
            <span className="emergency-contact-card-label">{t('emergency.card.notes')}</span>
            <span className="emergency-contact-card-value">{contact.notes}</span>
          </div>
        )}

        <div className="emergency-contact-card-field">
          <span className="emergency-contact-card-label">{t('emergency.card.priority')}</span>
          <span className="emergency-contact-card-value">{contact.priority_order}</span>
        </div>
      </div>

      {showActions && (onEdit || onDelete) && (
        <div className="emergency-contact-card-actions">
          {onEdit && (
            <button
              type="button"
              onClick={() => onEdit(contact)}
              className="emergency-contact-card-button edit"
              aria-label={t('emergency.card.editAria', { name: contact.name })}
            >
              {t('common.edit')}
            </button>
          )}
          {onDelete && (
            <button
              type="button"
              onClick={() => onDelete(contact)}
              className="emergency-contact-card-button delete"
              aria-label={t('emergency.card.deleteAria', { name: contact.name })}
            >
              {t('common.delete')}
            </button>
          )}
        </div>
      )}
    </div>
  );
};

EmergencyContactCard.displayName = 'EmergencyContactCard';
