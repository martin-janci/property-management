/**
 * Emergency Contacts List component for Epic 62.
 *
 * Displays a list of emergency contacts grouped by type.
 */

import type { EmergencyContact } from '@ppt/api-client';
import { CONTACT_TYPE_LABELS } from '@ppt/api-client';
import type React from 'react';
import { useTranslation } from 'react-i18next';
import { EmergencyContactCard } from './EmergencyContactCard';

export interface EmergencyContactsListProps {
  contacts: EmergencyContact[];
  onEdit?: (contact: EmergencyContact) => void;
  onDelete?: (contact: EmergencyContact) => void;
  showActions?: boolean;
  groupByType?: boolean;
}

export const EmergencyContactsList: React.FC<EmergencyContactsListProps> = ({
  contacts,
  onEdit,
  onDelete,
  showActions = true,
  groupByType = true,
}) => {
  const { t } = useTranslation();
  const typeLabelOf = (type: string) => {
    const english = CONTACT_TYPE_LABELS[type as keyof typeof CONTACT_TYPE_LABELS];
    return english ? t(`emergency.contactTypes.${type}`, { defaultValue: english }) : type;
  };

  if (contacts.length === 0) {
    return (
      <div className="emergency-contacts-empty">
        <p>{t('emergency.list.empty')}</p>
        <p>{t('emergency.list.emptyHint')}</p>
      </div>
    );
  }

  if (!groupByType) {
    // Sort by priority order
    const sortedContacts = [...contacts].sort((a, b) => a.priority_order - b.priority_order);

    return (
      <div className="emergency-contacts-list">
        {sortedContacts.map((contact) => (
          <EmergencyContactCard
            key={contact.id}
            contact={contact}
            onEdit={onEdit}
            onDelete={onDelete}
            showActions={showActions}
          />
        ))}
      </div>
    );
  }

  // Group contacts by type
  const groupedContacts: Record<string, EmergencyContact[]> = {};

  for (const contact of contacts) {
    const type = contact.contact_type;
    if (!groupedContacts[type]) {
      groupedContacts[type] = [];
    }
    groupedContacts[type].push(contact);
  }

  // Sort each group by priority order
  for (const type of Object.keys(groupedContacts)) {
    groupedContacts[type].sort((a, b) => a.priority_order - b.priority_order);
  }

  // Sort groups by the label
  const sortedTypes = Object.keys(groupedContacts).sort((a, b) => {
    return typeLabelOf(a).localeCompare(typeLabelOf(b));
  });

  return (
    <div className="emergency-contacts-list grouped">
      {sortedTypes.map((type) => {
        const typeLabel = typeLabelOf(type);
        const typeContacts = groupedContacts[type];

        return (
          <div key={type} className="emergency-contacts-group">
            <h3 className="emergency-contacts-group-title">
              {typeLabel}
              <span className="emergency-contacts-group-count">({typeContacts.length})</span>
            </h3>
            <div className="emergency-contacts-group-items">
              {typeContacts.map((contact) => (
                <EmergencyContactCard
                  key={contact.id}
                  contact={contact}
                  onEdit={onEdit}
                  onDelete={onDelete}
                  showActions={showActions}
                />
              ))}
            </div>
          </div>
        );
      })}
    </div>
  );
};

EmergencyContactsList.displayName = 'EmergencyContactsList';
