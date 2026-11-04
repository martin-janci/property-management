/**
 * TemplatesPage - Manage lease templates.
 * Epic 19: Lease Management & Tenant Screening
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { CreateTemplateData, LeaseTemplate } from '../types';

interface TemplatesPageProps {
  templates: LeaseTemplate[];
  isLoading?: boolean;
  isSubmitting?: boolean;
  onCreateTemplate: (data: CreateTemplateData) => void;
  onUpdateTemplate: (id: string, data: Partial<CreateTemplateData>) => void;
  /** Omit to hide the Delete control (the API has no template delete endpoint). */
  onDeleteTemplate?: (id: string) => void;
  onToggleActive: (id: string, isActive: boolean) => void;
  /**
   * Show the default rent / currency inputs. The lease template API does not
   * persist these, so the live route hides them rather than silently dropping
   * what the user typed.
   */
  showRentDefaults?: boolean;
}

interface TemplateFormData {
  name: string;
  description: string;
  content: string;
  defaultRentAmount: string;
  defaultCurrency: string;
  defaultDepositMonths: string;
  defaultLeaseDurationMonths: string;
  isActive: boolean;
}

const CURRENCIES = ['EUR', 'USD', 'GBP', 'CZK', 'PLN', 'HUF'];

export function TemplatesPage({
  templates,
  isLoading,
  isSubmitting,
  onCreateTemplate,
  onUpdateTemplate,
  onDeleteTemplate,
  onToggleActive,
  showRentDefaults = true,
}: TemplatesPageProps) {
  const { t } = useTranslation();
  const [showForm, setShowForm] = useState(false);
  const [editingTemplate, setEditingTemplate] = useState<LeaseTemplate | null>(null);
  const [formData, setFormData] = useState<TemplateFormData>({
    name: '',
    description: '',
    content: '',
    defaultRentAmount: '',
    defaultCurrency: 'EUR',
    defaultDepositMonths: '2',
    defaultLeaseDurationMonths: '12',
    isActive: true,
  });
  const [errors, setErrors] = useState<Partial<Record<keyof TemplateFormData, string>>>({});

  const resetForm = () => {
    setFormData({
      name: '',
      description: '',
      content: '',
      defaultRentAmount: '',
      defaultCurrency: 'EUR',
      defaultDepositMonths: '2',
      defaultLeaseDurationMonths: '12',
      isActive: true,
    });
    setErrors({});
    setEditingTemplate(null);
  };

  const handleEdit = (template: LeaseTemplate) => {
    setEditingTemplate(template);
    setFormData({
      name: template.name,
      description: template.description || '',
      content: template.content,
      defaultRentAmount: template.defaultRentAmount?.toString() || '',
      defaultCurrency: template.defaultCurrency || 'EUR',
      defaultDepositMonths: template.defaultDepositMonths?.toString() || '2',
      defaultLeaseDurationMonths: template.defaultLeaseDurationMonths?.toString() || '12',
      isActive: template.isActive,
    });
    setShowForm(true);
  };

  const handleCancel = () => {
    setShowForm(false);
    resetForm();
  };

  const validate = (): boolean => {
    const newErrors: Partial<Record<keyof TemplateFormData, string>> = {};

    if (!formData.name.trim()) {
      newErrors.name = t('leases.templates.errors.nameRequired');
    }
    if (!formData.content.trim()) {
      newErrors.content = t('leases.templates.errors.contentRequired');
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!validate()) return;

    const data: CreateTemplateData = {
      name: formData.name,
      description: formData.description || undefined,
      content: formData.content,
      defaultRentAmount: formData.defaultRentAmount
        ? Number(formData.defaultRentAmount)
        : undefined,
      defaultCurrency: formData.defaultCurrency || undefined,
      defaultDepositMonths: formData.defaultDepositMonths
        ? Number(formData.defaultDepositMonths)
        : undefined,
      defaultLeaseDurationMonths: formData.defaultLeaseDurationMonths
        ? Number(formData.defaultLeaseDurationMonths)
        : undefined,
      isActive: formData.isActive,
    };

    if (editingTemplate) {
      onUpdateTemplate(editingTemplate.id, data);
    } else {
      onCreateTemplate(data);
    }

    setShowForm(false);
    resetForm();
  };

  const handleChange = (
    e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>
  ) => {
    const { name, value, type } = e.target;
    const checked = (e.target as HTMLInputElement).checked;
    setFormData((prev) => ({
      ...prev,
      [name]: type === 'checkbox' ? checked : value,
    }));
    if (errors[name as keyof TemplateFormData]) {
      setErrors((prev) => ({ ...prev, [name]: undefined }));
    }
  };

  const formatCurrency = (amount?: number, currency?: string) => {
    if (!amount) return '-';
    return new Intl.NumberFormat(undefined, {
      style: 'currency',
      currency: currency || 'EUR',
    }).format(amount);
  };

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-12">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600" />
      </div>
    );
  }

  return (
    <div className="max-w-4xl mx-auto px-4 py-8">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">{t('leases.templates.title')}</h1>
          <p className="mt-1 text-sm text-gray-500">{t('leases.templates.subtitle')}</p>
        </div>
        {!showForm && (
          <button
            type="button"
            onClick={() => setShowForm(true)}
            className="px-4 py-2 text-sm font-medium text-white bg-blue-600 hover:bg-blue-700 rounded-md"
          >
            {t('leases.templates.createNew')}
          </button>
        )}
      </div>

      {/* Form */}
      {showForm && (
        <div className="bg-white rounded-lg shadow p-6 mb-6">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">
            {editingTemplate
              ? t('leases.templates.editTemplate')
              : t('leases.templates.createTemplate')}
          </h2>
          <form onSubmit={handleSubmit} className="space-y-4">
            {/* Name */}
            <div>
              <label htmlFor="name" className="block text-sm font-medium text-gray-700">
                {t('leases.templates.form.name')} *
              </label>
              <input
                type="text"
                id="name"
                name="name"
                value={formData.name}
                onChange={handleChange}
                placeholder={t('leases.templates.form.namePlaceholder')}
                className={`mt-1 block w-full rounded-md border ${
                  errors.name ? 'border-red-500' : 'border-gray-300'
                } px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500`}
              />
              {errors.name && <p className="mt-1 text-sm text-red-500">{errors.name}</p>}
            </div>

            {/* Description */}
            <div>
              <label htmlFor="description" className="block text-sm font-medium text-gray-700">
                {t('leases.templates.form.description')}
              </label>
              <input
                type="text"
                id="description"
                name="description"
                value={formData.description}
                onChange={handleChange}
                placeholder={t('leases.templates.form.descriptionPlaceholder')}
                className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>

            {/* Content */}
            <div>
              <label htmlFor="content" className="block text-sm font-medium text-gray-700">
                {t('leases.templates.form.content')} *
              </label>
              <textarea
                id="content"
                name="content"
                value={formData.content}
                onChange={handleChange}
                rows={8}
                placeholder={t('leases.templates.form.contentPlaceholder')}
                className={`mt-1 block w-full rounded-md border ${
                  errors.content ? 'border-red-500' : 'border-gray-300'
                } px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500 font-mono text-sm`}
              />
              {errors.content && <p className="mt-1 text-sm text-red-500">{errors.content}</p>}
            </div>

            {/* Defaults Grid */}
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
              {showRentDefaults && (
                <>
                  <div>
                    <label
                      htmlFor="defaultRentAmount"
                      className="block text-sm font-medium text-gray-700"
                    >
                      {t('leases.templates.form.defaultRent')}
                    </label>
                    <input
                      type="number"
                      id="defaultRentAmount"
                      name="defaultRentAmount"
                      value={formData.defaultRentAmount}
                      onChange={handleChange}
                      min="0"
                      step="0.01"
                      className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                    />
                  </div>
                  <div>
                    <label
                      htmlFor="defaultCurrency"
                      className="block text-sm font-medium text-gray-700"
                    >
                      {t('leases.templates.form.currency')}
                    </label>
                    <select
                      id="defaultCurrency"
                      name="defaultCurrency"
                      value={formData.defaultCurrency}
                      onChange={handleChange}
                      className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                    >
                      {CURRENCIES.map((currency) => (
                        <option key={currency} value={currency}>
                          {currency}
                        </option>
                      ))}
                    </select>
                  </div>
                </>
              )}
              <div>
                <label
                  htmlFor="defaultDepositMonths"
                  className="block text-sm font-medium text-gray-700"
                >
                  {t('leases.templates.form.depositMonths')}
                </label>
                <input
                  type="number"
                  id="defaultDepositMonths"
                  name="defaultDepositMonths"
                  value={formData.defaultDepositMonths}
                  onChange={handleChange}
                  min="0"
                  max="12"
                  className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>
              <div>
                <label
                  htmlFor="defaultLeaseDurationMonths"
                  className="block text-sm font-medium text-gray-700"
                >
                  {t('leases.templates.form.durationMonths')}
                </label>
                <input
                  type="number"
                  id="defaultLeaseDurationMonths"
                  name="defaultLeaseDurationMonths"
                  value={formData.defaultLeaseDurationMonths}
                  onChange={handleChange}
                  min="1"
                  max="120"
                  className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>
            </div>

            {/* Active Toggle */}
            <div className="flex items-center gap-2">
              <input
                type="checkbox"
                id="isActive"
                name="isActive"
                checked={formData.isActive}
                onChange={handleChange}
                className="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
              />
              <label htmlFor="isActive" className="text-sm font-medium text-gray-700">
                {t('leases.templates.form.isActive')}
              </label>
            </div>

            {/* Actions */}
            <div className="flex items-center justify-end gap-3 pt-4 border-t">
              <button
                type="button"
                onClick={handleCancel}
                disabled={isSubmitting}
                className="px-4 py-2 text-gray-700 border border-gray-300 rounded-lg hover:bg-gray-50 disabled:opacity-50"
              >
                {t('common.cancel')}
              </button>
              <button
                type="submit"
                disabled={isSubmitting}
                className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 disabled:opacity-50 flex items-center gap-2"
              >
                {isSubmitting && (
                  <div className="animate-spin rounded-full h-4 w-4 border-b-2 border-white" />
                )}
                {editingTemplate ? t('common.save') : t('common.create')}
              </button>
            </div>
          </form>
        </div>
      )}

      {/* Templates List */}
      {templates.length === 0 && !showForm ? (
        <div className="text-center py-12 bg-white rounded-lg shadow">
          <svg
            className="mx-auto h-12 w-12 text-gray-400"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={1.5}
              d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
            />
          </svg>
          <p className="mt-4 text-gray-500">{t('leases.templates.noTemplates')}</p>
          <button
            type="button"
            onClick={() => setShowForm(true)}
            className="mt-4 px-4 py-2 text-sm font-medium text-blue-600 hover:text-blue-800"
          >
            {t('leases.templates.createFirst')}
          </button>
        </div>
      ) : (
        <div className="space-y-4">
          {templates.map((template) => (
            <div
              key={template.id}
              className={`bg-white rounded-lg shadow p-4 ${!template.isActive ? 'opacity-60' : ''}`}
            >
              <div className="flex items-start justify-between">
                <div className="flex-1">
                  <div className="flex items-center gap-2">
                    <h3 className="text-lg font-semibold text-gray-900">{template.name}</h3>
                    <span
                      className={`px-2 py-0.5 text-xs font-medium rounded ${
                        template.isActive
                          ? 'bg-green-100 text-green-800'
                          : 'bg-gray-100 text-gray-800'
                      }`}
                    >
                      {template.isActive
                        ? t('leases.templates.active')
                        : t('leases.templates.inactive')}
                    </span>
                  </div>
                  {template.description && (
                    <p className="text-gray-600 mt-1">{template.description}</p>
                  )}
                  <div className="flex items-center gap-4 mt-2 text-sm text-gray-500">
                    {template.defaultRentAmount && (
                      <span>
                        {t('leases.templates.defaultRent')}:{' '}
                        {formatCurrency(template.defaultRentAmount, template.defaultCurrency)}
                      </span>
                    )}
                    {template.defaultLeaseDurationMonths && (
                      <span>
                        {t('leases.templates.duration')}:{' '}
                        {t('leases.templates.months', {
                          count: template.defaultLeaseDurationMonths,
                        })}
                      </span>
                    )}
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <button
                    type="button"
                    onClick={() => onToggleActive(template.id, !template.isActive)}
                    className="text-sm text-gray-600 hover:text-gray-800"
                  >
                    {template.isActive
                      ? t('leases.templates.deactivate')
                      : t('leases.templates.activate')}
                  </button>
                  <button
                    type="button"
                    onClick={() => handleEdit(template)}
                    className="text-sm text-blue-600 hover:text-blue-800"
                  >
                    {t('common.edit')}
                  </button>
                  {onDeleteTemplate && (
                    <button
                      type="button"
                      onClick={() => onDeleteTemplate(template.id)}
                      className="text-sm text-red-600 hover:text-red-800"
                    >
                      {t('common.delete')}
                    </button>
                  )}
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
