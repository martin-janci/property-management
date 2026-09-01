/**
 * Emergency Contact Directory Page for Epic 62.
 *
 * Main page for viewing and managing emergency contacts.
 */

import type { CreateEmergencyContact, EmergencyContact } from '@ppt/api-client';
import {
  CONTACT_TYPE_LABELS,
  createEmergencyContact,
  deleteEmergencyContact,
  listEmergencyContacts,
  updateEmergencyContact,
} from '@ppt/api-client';
import type React from 'react';
import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ConfirmationDialog } from '../../../components';
import { useOrganization } from '../../../hooks';
import { EmergencyContactForm, EmergencyContactsList } from '../components';
import '../styles/emergency-contacts.css';

export const EmergencyContactDirectoryPage: React.FC = () => {
  const { t } = useTranslation();
  const { organizationId } = useOrganization();

  const [contacts, setContacts] = useState<EmergencyContact[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showForm, setShowForm] = useState(false);
  const [editingContact, setEditingContact] = useState<EmergencyContact | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  // Delete confirmation dialog state
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [contactToDelete, setContactToDelete] = useState<EmergencyContact | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);

  // Filters
  const [filterType, setFilterType] = useState<string>('');
  const [showInactive, setShowInactive] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');

  // Load contacts
  useEffect(() => {
    const loadContacts = async () => {
      try {
        setIsLoading(true);
        setError(null);

        const data = await listEmergencyContacts({
          organization_id: organizationId,
          ...(filterType && { contact_type: filterType }),
          ...(!showInactive && { is_active: true }),
        });

        setContacts(data);
      } catch (err) {
        setError(t('emergency.errors.failedToLoad'));
        console.error('Failed to load emergency contacts:', err);
      } finally {
        setIsLoading(false);
      }
    };

    loadContacts();
  }, [organizationId, filterType, showInactive, t]);

  // Filter contacts by search query
  const filteredContacts = contacts.filter((contact) => {
    if (!searchQuery) return true;

    const query = searchQuery.toLowerCase();
    return (
      contact.name.toLowerCase().includes(query) ||
      contact.role.toLowerCase().includes(query) ||
      contact.phone?.toLowerCase().includes(query) ||
      contact.email?.toLowerCase().includes(query)
    );
  });

  // Reload contacts function for handlers
  const reloadContacts = useCallback(async () => {
    try {
      const data = await listEmergencyContacts({
        organization_id: organizationId,
        ...(filterType && { contact_type: filterType }),
        ...(!showInactive && { is_active: true }),
      });
      setContacts(data);
    } catch (err) {
      setError(t('emergency.errors.failedToReload'));
      console.error('Failed to reload emergency contacts:', err);
    }
  }, [organizationId, filterType, showInactive, t]);

  const handleCreate = async (data: CreateEmergencyContact) => {
    try {
      setIsSubmitting(true);
      setError(null);

      await createEmergencyContact(organizationId, data);

      setShowForm(false);
      await reloadContacts();
    } catch (err) {
      setError(t('emergency.errors.failedToCreate'));
      console.error('Failed to create emergency contact:', err);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleUpdate = async (data: CreateEmergencyContact) => {
    if (!editingContact) return;

    try {
      setIsSubmitting(true);
      setError(null);

      await updateEmergencyContact(editingContact.id, organizationId, data);

      setEditingContact(null);
      setShowForm(false);
      await reloadContacts();
    } catch (err) {
      setError(t('emergency.errors.failedToUpdate'));
      console.error('Failed to update emergency contact:', err);
    } finally {
      setIsSubmitting(false);
    }
  };

  // Open delete confirmation dialog
  const handleDeleteRequest = useCallback((contact: EmergencyContact) => {
    setContactToDelete(contact);
    setDeleteDialogOpen(true);
  }, []);

  // Cancel delete action
  const handleDeleteCancel = useCallback(() => {
    setDeleteDialogOpen(false);
    setContactToDelete(null);
  }, []);

  // Confirm and execute delete
  const handleDeleteConfirm = useCallback(async () => {
    if (!contactToDelete) return;

    try {
      setIsDeleting(true);
      setError(null);
      await deleteEmergencyContact(contactToDelete.id, organizationId);
      setDeleteDialogOpen(false);
      setContactToDelete(null);
      await reloadContacts();
    } catch (err) {
      setError(t('emergency.errors.failedToDelete'));
      console.error('Failed to delete emergency contact:', err);
    } finally {
      setIsDeleting(false);
    }
  }, [contactToDelete, organizationId, reloadContacts, t]);

  const handleEdit = (contact: EmergencyContact) => {
    setEditingContact(contact);
    setShowForm(true);
  };

  const handleCancelForm = () => {
    setShowForm(false);
    setEditingContact(null);
  };

  const handleAddNew = () => {
    setEditingContact(null);
    setShowForm(true);
  };

  return (
    <div className="emergency-contact-directory-page">
      <div className="emergency-contact-directory-header">
        <h1>{t('emergency.title')}</h1>
        <p>{t('emergency.description')}</p>
      </div>

      {error && (
        <div className="emergency-contact-directory-error" role="alert">
          {error}
        </div>
      )}

      {!showForm && (
        <div className="emergency-contact-directory-controls">
          <div className="emergency-contact-directory-filters">
            <div className="emergency-contact-directory-filter">
              <label htmlFor="search">{t('common.search')}</label>
              <input
                type="search"
                id="search"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder={t('emergency.searchPlaceholder')}
                className="emergency-contact-directory-search"
              />
            </div>

            <div className="emergency-contact-directory-filter">
              <label htmlFor="filterType">{t('emergency.type')}</label>
              <select
                id="filterType"
                value={filterType}
                onChange={(e) => setFilterType(e.target.value)}
                className="emergency-contact-directory-select"
              >
                <option value="">{t('emergency.allTypes')}</option>
                {Object.entries(CONTACT_TYPE_LABELS).map(([value, label]) => (
                  <option key={value} value={value}>
                    {label}
                  </option>
                ))}
              </select>
            </div>

            <div className="emergency-contact-directory-filter checkbox">
              <label htmlFor="showInactive">
                <input
                  type="checkbox"
                  id="showInactive"
                  checked={showInactive}
                  onChange={(e) => setShowInactive(e.target.checked)}
                />
                {t('emergency.showInactive')}
              </label>
            </div>
          </div>

          <button
            type="button"
            onClick={handleAddNew}
            className="emergency-contact-directory-add-button"
          >
            {t('emergency.addContact')}
          </button>
        </div>
      )}

      {showForm ? (
        <div className="emergency-contact-directory-form-container">
          <EmergencyContactForm
            contact={editingContact || undefined}
            onSubmit={editingContact ? handleUpdate : handleCreate}
            onCancel={handleCancelForm}
            isSubmitting={isSubmitting}
          />
        </div>
      ) : isLoading ? (
        <div className="emergency-contact-directory-loading">{t('emergency.loading')}</div>
      ) : (
        <EmergencyContactsList
          contacts={filteredContacts}
          onEdit={handleEdit}
          onDelete={handleDeleteRequest}
          showActions={true}
          groupByType={true}
        />
      )}

      {/* Accessible delete confirmation dialog */}
      <ConfirmationDialog
        isOpen={deleteDialogOpen}
        title={t('emergency.deleteTitle')}
        message={
          contactToDelete ? t('emergency.deleteConfirmation', { name: contactToDelete.name }) : ''
        }
        confirmLabel={t('common.delete')}
        cancelLabel={t('common.cancel')}
        variant="danger"
        onConfirm={handleDeleteConfirm}
        onCancel={handleDeleteCancel}
        isLoading={isDeleting}
      />
    </div>
  );
};

EmergencyContactDirectoryPage.displayName = 'EmergencyContactDirectoryPage';
