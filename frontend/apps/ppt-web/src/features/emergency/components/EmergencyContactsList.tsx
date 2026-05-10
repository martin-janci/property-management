/**
 * Emergency Contacts List component for Epic 62.
 *
 * Displays a list of emergency contacts grouped by type.
 */

import type { EmergencyContact } from '@ppt/api-client';
import { CONTACT_TYPE_LABELS } from '@ppt/api-client';
import type React from 'react';
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
  if (contacts.length === 0) {
    return (
      <div className="emergency-contacts-empty">
        <p>No emergency contacts found.</p>
        <p>Add your first emergency contact to get started.</p>
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
    const labelA = CONTACT_TYPE_LABELS[a as keyof typeof CONTACT_TYPE_LABELS] || a;
    const labelB = CONTACT_TYPE_LABELS[b as keyof typeof CONTACT_TYPE_LABELS] || b;
    return labelA.localeCompare(labelB);
  });

  return (
    <div className="emergency-contacts-list grouped">
      {sortedTypes.map((type) => {
        const typeLabel = CONTACT_TYPE_LABELS[type as keyof typeof CONTACT_TYPE_LABELS] || type;
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
