/**
 * Emergency Contact Form component for Epic 62.
 *
 * Form for creating and editing emergency contacts.
 */

import type { CreateEmergencyContact, EmergencyContact } from '@ppt/api-client';
import { CONTACT_TYPE_LABELS, CONTACT_TYPES } from '@ppt/api-client';
import type React from 'react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

export interface EmergencyContactFormProps {
  contact?: EmergencyContact;
  onSubmit: (data: CreateEmergencyContact) => void;
  onCancel: () => void;
  isSubmitting?: boolean;
}

export const EmergencyContactForm: React.FC<EmergencyContactFormProps> = ({
  contact,
  onSubmit,
  onCancel,
  isSubmitting = false,
}) => {
  const { t } = useTranslation();
  const [formData, setFormData] = useState<CreateEmergencyContact>({
    name: contact?.name || '',
    role: contact?.role || '',
    contact_type: contact?.contact_type || CONTACT_TYPES.OTHER,
    phone: contact?.phone || '',
    phone_secondary: contact?.phone_secondary || '',
    email: contact?.email || '',
    address: contact?.address || '',
    available_hours: contact?.available_hours || '',
    notes: contact?.notes || '',
    priority_order: contact?.priority_order || 0,
    building_id: contact?.building_id,
  });

  const [errors, setErrors] = useState<Record<string, string>>({});

  const handleChange = (
    e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement | HTMLTextAreaElement>
  ) => {
    const { name, value } = e.target;
    setFormData((prev) => ({ ...prev, [name]: value }));
    // Clear error when user starts typing
    if (errors[name]) {
      setErrors((prev) => {
        const newErrors = { ...prev };
        delete newErrors[name];
        return newErrors;
      });
    }
  };

  const validate = (): boolean => {
    const newErrors: Record<string, string> = {};

    if (!formData.name.trim()) {
      newErrors.name = t('emergency.form.errors.nameRequired');
    }

    if (!formData.role.trim()) {
      newErrors.role = t('emergency.form.errors.roleRequired');
    }

    if (!formData.contact_type) {
      newErrors.contact_type = t('emergency.form.errors.contactTypeRequired');
    }

    // Validate at least one contact method - only set error on the first empty field
    // to avoid confusing UX with duplicate error messages
    if (!formData.phone && !formData.email) {
      newErrors.contactMethod = t('emergency.form.errors.contactMethodRequired');
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();

    if (!validate()) {
      return;
    }

    // Clean up empty optional fields
    const cleanedData: CreateEmergencyContact = {
      name: formData.name.trim(),
      role: formData.role.trim(),
      contact_type: formData.contact_type,
      ...(formData.phone && { phone: formData.phone.trim() }),
      ...(formData.phone_secondary && { phone_secondary: formData.phone_secondary.trim() }),
      ...(formData.email && { email: formData.email.trim() }),
      ...(formData.address && { address: formData.address.trim() }),
      ...(formData.available_hours && { available_hours: formData.available_hours.trim() }),
      ...(formData.notes && { notes: formData.notes.trim() }),
      ...(formData.priority_order !== undefined && {
        priority_order: Number(formData.priority_order),
      }),
      ...(formData.building_id && { building_id: formData.building_id }),
    };

    onSubmit(cleanedData);
  };

  return (
    <form className="emergency-contact-form" onSubmit={handleSubmit}>
      <h2>{contact ? t('emergency.form.editTitle') : t('emergency.form.addTitle')}</h2>

      <div className="emergency-contact-form-field">
        <label htmlFor="name" className="emergency-contact-form-label required">
          {t('emergency.form.name')}
        </label>
        <input
          type="text"
          id="name"
          name="name"
          value={formData.name}
          onChange={handleChange}
          className={`emergency-contact-form-input ${errors.name ? 'error' : ''}`}
          aria-invalid={!!errors.name}
          aria-describedby={errors.name ? 'name-error' : undefined}
          disabled={isSubmitting}
        />
        {errors.name && (
          <span id="name-error" className="emergency-contact-form-error">
            {errors.name}
          </span>
        )}
      </div>

      <div className="emergency-contact-form-field">
        <label htmlFor="role" className="emergency-contact-form-label required">
          {t('emergency.form.role')}
        </label>
        <input
          type="text"
          id="role"
          name="role"
          value={formData.role}
          onChange={handleChange}
          className={`emergency-contact-form-input ${errors.role ? 'error' : ''}`}
          placeholder={t('emergency.form.rolePlaceholder')}
          aria-invalid={!!errors.role}
          aria-describedby={errors.role ? 'role-error' : undefined}
          disabled={isSubmitting}
        />
        {errors.role && (
          <span id="role-error" className="emergency-contact-form-error">
            {errors.role}
          </span>
        )}
      </div>

      <div className="emergency-contact-form-field">
        <label htmlFor="contact_type" className="emergency-contact-form-label required">
          {t('emergency.form.contactType')}
        </label>
        <select
          id="contact_type"
          name="contact_type"
          value={formData.contact_type}
          onChange={handleChange}
          className={`emergency-contact-form-select ${errors.contact_type ? 'error' : ''}`}
          aria-invalid={!!errors.contact_type}
          aria-describedby={errors.contact_type ? 'contact_type-error' : undefined}
          disabled={isSubmitting}
        >
          {Object.entries(CONTACT_TYPE_LABELS).map(([value, label]) => (
            <option key={value} value={value}>
              {label}
            </option>
          ))}
        </select>
        {errors.contact_type && (
          <span id="contact_type-error" className="emergency-contact-form-error">
            {errors.contact_type}
          </span>
        )}
      </div>

      {errors.contactMethod && (
        <div id="contact-method-error" className="emergency-contact-form-error-banner" role="alert">
          {errors.contactMethod}
        </div>
      )}

      <div className="emergency-contact-form-field">
        <label htmlFor="phone" className="emergency-contact-form-label">
          {t('emergency.form.phone')}
        </label>
        <input
          type="tel"
          id="phone"
          name="phone"
          value={formData.phone}
          onChange={handleChange}
          className={`emergency-contact-form-input ${errors.contactMethod ? 'error' : ''}`}
          placeholder={t('emergency.form.phonePlaceholder')}
          aria-invalid={!!errors.contactMethod}
          aria-describedby={errors.contactMethod ? 'contact-method-error' : undefined}
          disabled={isSubmitting}
        />
      </div>

      <div className="emergency-contact-form-field">
        <label htmlFor="phone_secondary" className="emergency-contact-form-label">
          {t('emergency.form.phoneSecondary')}
        </label>
        <input
          type="tel"
          id="phone_secondary"
          name="phone_secondary"
          value={formData.phone_secondary}
          onChange={handleChange}
          className="emergency-contact-form-input"
          placeholder={t('emergency.form.phoneSecondaryPlaceholder')}
          disabled={isSubmitting}
        />
      </div>

      <div className="emergency-contact-form-field">
        <label htmlFor="email" className="emergency-contact-form-label">
          {t('emergency.form.email')}
        </label>
        <input
          type="email"
          id="email"
          name="email"
          value={formData.email}
          onChange={handleChange}
          className={`emergency-contact-form-input ${errors.contactMethod ? 'error' : ''}`}
          placeholder={t('emergency.form.emailPlaceholder')}
          aria-invalid={!!errors.contactMethod}
          aria-describedby={errors.contactMethod ? 'contact-method-error' : undefined}
          disabled={isSubmitting}
        />
      </div>

      <div className="emergency-contact-form-field">
        <label htmlFor="address" className="emergency-contact-form-label">
          {t('emergency.form.address')}
        </label>
        <input
          type="text"
          id="address"
          name="address"
          value={formData.address}
          onChange={handleChange}
          className="emergency-contact-form-input"
          placeholder={t('emergency.form.addressPlaceholder')}
          disabled={isSubmitting}
        />
      </div>

      <div className="emergency-contact-form-field">
        <label htmlFor="available_hours" className="emergency-contact-form-label">
          {t('emergency.form.availableHours')}
        </label>
        <input
          type="text"
          id="available_hours"
          name="available_hours"
          value={formData.available_hours}
          onChange={handleChange}
          className="emergency-contact-form-input"
          placeholder={t('emergency.form.availableHoursPlaceholder')}
          disabled={isSubmitting}
        />
      </div>

      <div className="emergency-contact-form-field">
        <label htmlFor="priority_order" className="emergency-contact-form-label">
          {t('emergency.form.priorityOrder')}
        </label>
        <input
          type="number"
          id="priority_order"
          name="priority_order"
          value={formData.priority_order}
          onChange={handleChange}
          className="emergency-contact-form-input"
          min="0"
          disabled={isSubmitting}
        />
        <span className="emergency-contact-form-help">{t('emergency.form.priorityOrderHelp')}</span>
      </div>

      <div className="emergency-contact-form-field">
        <label htmlFor="notes" className="emergency-contact-form-label">
          {t('emergency.form.notes')}
        </label>
        <textarea
          id="notes"
          name="notes"
          value={formData.notes}
          onChange={handleChange}
          className="emergency-contact-form-textarea"
          rows={4}
          placeholder={t('emergency.form.notesPlaceholder')}
          disabled={isSubmitting}
        />
      </div>

      <div className="emergency-contact-form-actions">
        <button
          type="button"
          onClick={onCancel}
          className="emergency-contact-form-button cancel"
          disabled={isSubmitting}
        >
          {t('common.cancel')}
        </button>
        <button
          type="submit"
          className="emergency-contact-form-button submit"
          disabled={isSubmitting}
        >
          {isSubmitting
            ? t('common.loading')
            : contact
              ? t('emergency.form.updateButton')
              : t('emergency.form.addButton')}
        </button>
      </div>
    </form>
  );
};

EmergencyContactForm.displayName = 'EmergencyContactForm';
