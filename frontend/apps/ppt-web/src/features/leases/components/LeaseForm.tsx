/**
 * LeaseForm component - form for creating/editing leases.
 * Epic 19: Lease Management & Tenant Screening
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { CreateLeaseData, LeaseTemplate } from '../types';

interface Unit {
  id: string;
  number: string;
  buildingName: string;
}

interface Tenant {
  id: string;
  name: string;
  email: string;
}

interface LeaseFormProps {
  initialData?: Partial<CreateLeaseData>;
  units?: Unit[];
  tenants?: Tenant[];
  templates?: LeaseTemplate[];
  isSubmitting?: boolean;
  onSubmit: (data: CreateLeaseData) => void;
  onCancel: () => void;
}

interface FormErrors {
  unitId?: string;
  tenantId?: string;
  startDate?: string;
  endDate?: string;
  rentAmount?: string;
  paymentDayOfMonth?: string;
}

const CURRENCIES = ['EUR', 'USD', 'GBP', 'CZK', 'PLN', 'HUF'];

export function LeaseForm({
  initialData,
  units = [],
  tenants = [],
  templates = [],
  isSubmitting,
  onSubmit,
  onCancel,
}: LeaseFormProps) {
  const { t } = useTranslation();

  const [formData, setFormData] = useState<CreateLeaseData>({
    unitId: initialData?.unitId || '',
    templateId: initialData?.templateId || undefined,
    tenantId: initialData?.tenantId || '',
    startDate: initialData?.startDate || '',
    endDate: initialData?.endDate || '',
    rentAmount: initialData?.rentAmount || 0,
    currency: initialData?.currency || 'EUR',
    depositAmount: initialData?.depositAmount || undefined,
    paymentDayOfMonth: initialData?.paymentDayOfMonth || 1,
    terms: initialData?.terms || '',
    notes: initialData?.notes || '',
  });
  const [errors, setErrors] = useState<FormErrors>({});

  const validate = (): boolean => {
    const newErrors: FormErrors = {};

    if (!formData.unitId) {
      newErrors.unitId = t('leases.form.errors.unitRequired');
    }
    if (!formData.tenantId) {
      newErrors.tenantId = t('leases.form.errors.tenantRequired');
    }
    if (!formData.startDate) {
      newErrors.startDate = t('leases.form.errors.startDateRequired');
    }
    if (!formData.endDate) {
      newErrors.endDate = t('leases.form.errors.endDateRequired');
    }
    if (formData.startDate && formData.endDate && formData.startDate >= formData.endDate) {
      newErrors.endDate = t('leases.form.errors.endDateAfterStart');
    }
    if (!formData.rentAmount || formData.rentAmount <= 0) {
      newErrors.rentAmount = t('leases.form.errors.rentAmountRequired');
    }
    if (formData.paymentDayOfMonth < 1 || formData.paymentDayOfMonth > 28) {
      newErrors.paymentDayOfMonth = t('leases.form.errors.paymentDayInvalid');
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (validate()) {
      onSubmit(formData);
    }
  };

  const handleChange = (
    e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>
  ) => {
    const { name, value, type } = e.target;
    setFormData((prev) => ({
      ...prev,
      [name]: type === 'number' ? (value === '' ? undefined : Number(value)) : value || undefined,
    }));
    if (errors[name as keyof FormErrors]) {
      setErrors((prev) => ({ ...prev, [name]: undefined }));
    }
  };

  const handleTemplateChange = (templateId: string) => {
    const template = templates.find((t) => t.id === templateId);
    if (template) {
      setFormData((prev) => ({
        ...prev,
        templateId,
        rentAmount: template.defaultRentAmount || prev.rentAmount,
        currency: template.defaultCurrency || prev.currency,
        depositAmount: template.defaultDepositMonths
          ? (template.defaultRentAmount || prev.rentAmount) * template.defaultDepositMonths
          : prev.depositAmount,
        terms: template.content || prev.terms,
      }));
    } else {
      setFormData((prev) => ({ ...prev, templateId: undefined }));
    }
  };

  return (
    <form onSubmit={handleSubmit} className="space-y-6">
      {/* Template Selection */}
      {templates.length > 0 && (
        <div>
          <label htmlFor="templateId" className="block text-sm font-medium text-gray-700">
            {t('leases.form.template')} ({t('common.optional')})
          </label>
          <select
            id="templateId"
            name="templateId"
            value={formData.templateId || ''}
            onChange={(e) => handleTemplateChange(e.target.value)}
            className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="">{t('leases.form.noTemplate')}</option>
            {templates.map((template) => (
              <option key={template.id} value={template.id}>
                {template.name}
              </option>
            ))}
          </select>
        </div>
      )}

      {/* Unit Selection */}
      <div>
        <label htmlFor="unitId" className="block text-sm font-medium text-gray-700">
          {t('leases.form.unit')} *
        </label>
        <select
          id="unitId"
          name="unitId"
          value={formData.unitId}
          onChange={handleChange}
          className={`mt-1 block w-full rounded-md border ${
            errors.unitId ? 'border-red-500' : 'border-gray-300'
          } px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500`}
        >
          <option value="">{t('leases.form.selectUnit')}</option>
          {units.map((unit) => (
            <option key={unit.id} value={unit.id}>
              {unit.buildingName} - {unit.number}
            </option>
          ))}
        </select>
        {errors.unitId && <p className="mt-1 text-sm text-red-500">{errors.unitId}</p>}
        {!errors.unitId && units.length === 0 && (
          <p className="mt-1 text-sm text-amber-600">{t('leases.form.noUnits')}</p>
        )}
      </div>

      {/* Tenant Selection */}
      <div>
        <label htmlFor="tenantId" className="block text-sm font-medium text-gray-700">
          {t('leases.form.tenant')} *
        </label>
        <select
          id="tenantId"
          name="tenantId"
          value={formData.tenantId}
          onChange={handleChange}
          className={`mt-1 block w-full rounded-md border ${
            errors.tenantId ? 'border-red-500' : 'border-gray-300'
          } px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500`}
        >
          <option value="">{t('leases.form.selectTenant')}</option>
          {tenants.map((tenant) => (
            <option key={tenant.id} value={tenant.id}>
              {tenant.name} ({tenant.email})
            </option>
          ))}
        </select>
        {errors.tenantId && <p className="mt-1 text-sm text-red-500">{errors.tenantId}</p>}
        {!errors.tenantId && tenants.length === 0 && (
          <p className="mt-1 text-sm text-amber-600">{t('leases.form.noTenants')}</p>
        )}
      </div>

      {/* Date Range */}
      <div className="grid grid-cols-2 gap-4">
        <div>
          <label htmlFor="startDate" className="block text-sm font-medium text-gray-700">
            {t('leases.form.startDate')} *
          </label>
          <input
            type="date"
            id="startDate"
            name="startDate"
            value={formData.startDate}
            onChange={handleChange}
            className={`mt-1 block w-full rounded-md border ${
              errors.startDate ? 'border-red-500' : 'border-gray-300'
            } px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500`}
          />
          {errors.startDate && <p className="mt-1 text-sm text-red-500">{errors.startDate}</p>}
        </div>
        <div>
          <label htmlFor="endDate" className="block text-sm font-medium text-gray-700">
            {t('leases.form.endDate')} *
          </label>
          <input
            type="date"
            id="endDate"
            name="endDate"
            value={formData.endDate}
            onChange={handleChange}
            className={`mt-1 block w-full rounded-md border ${
              errors.endDate ? 'border-red-500' : 'border-gray-300'
            } px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500`}
          />
          {errors.endDate && <p className="mt-1 text-sm text-red-500">{errors.endDate}</p>}
        </div>
      </div>

      {/* Rent Amount and Currency */}
      <div className="grid grid-cols-2 gap-4">
        <div>
          <label htmlFor="rentAmount" className="block text-sm font-medium text-gray-700">
            {t('leases.form.rentAmount')} *
          </label>
          <input
            type="number"
            id="rentAmount"
            name="rentAmount"
            value={formData.rentAmount || ''}
            onChange={handleChange}
            min="0"
            step="0.01"
            className={`mt-1 block w-full rounded-md border ${
              errors.rentAmount ? 'border-red-500' : 'border-gray-300'
            } px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500`}
          />
          {errors.rentAmount && <p className="mt-1 text-sm text-red-500">{errors.rentAmount}</p>}
        </div>
        <div>
          <label htmlFor="currency" className="block text-sm font-medium text-gray-700">
            {t('leases.form.currency')}
          </label>
          <select
            id="currency"
            name="currency"
            value={formData.currency}
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
      </div>

      {/* Deposit Amount */}
      <div>
        <label htmlFor="depositAmount" className="block text-sm font-medium text-gray-700">
          {t('leases.form.depositAmount')} ({t('common.optional')})
        </label>
        <input
          type="number"
          id="depositAmount"
          name="depositAmount"
          value={formData.depositAmount || ''}
          onChange={handleChange}
          min="0"
          step="0.01"
          className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>

      {/* Payment Day */}
      <div>
        <label htmlFor="paymentDayOfMonth" className="block text-sm font-medium text-gray-700">
          {t('leases.form.paymentDay')} *
        </label>
        <input
          type="number"
          id="paymentDayOfMonth"
          name="paymentDayOfMonth"
          value={formData.paymentDayOfMonth}
          onChange={handleChange}
          min="1"
          max="28"
          className={`mt-1 block w-full rounded-md border ${
            errors.paymentDayOfMonth ? 'border-red-500' : 'border-gray-300'
          } px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500`}
        />
        <p className="mt-1 text-xs text-gray-500">{t('leases.form.paymentDayHelp')}</p>
        {errors.paymentDayOfMonth && (
          <p className="mt-1 text-sm text-red-500">{errors.paymentDayOfMonth}</p>
        )}
      </div>

      {/* Terms */}
      <div>
        <label htmlFor="terms" className="block text-sm font-medium text-gray-700">
          {t('leases.form.terms')} ({t('common.optional')})
        </label>
        <textarea
          id="terms"
          name="terms"
          value={formData.terms || ''}
          onChange={handleChange}
          rows={4}
          placeholder={t('leases.form.termsPlaceholder')}
          className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>

      {/* Notes */}
      <div>
        <label htmlFor="notes" className="block text-sm font-medium text-gray-700">
          {t('leases.form.notes')} ({t('common.optional')})
        </label>
        <textarea
          id="notes"
          name="notes"
          value={formData.notes || ''}
          onChange={handleChange}
          rows={2}
          placeholder={t('leases.form.notesPlaceholder')}
          className="mt-1 block w-full rounded-md border border-gray-300 px-3 py-2 focus:outline-none focus:ring-2 focus:ring-blue-500"
        />
      </div>

      {/* Actions */}
      <div className="flex items-center justify-end gap-3 pt-4 border-t">
        <button
          type="button"
          onClick={onCancel}
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
          {isSubmitting ? t('common.saving') : t('leases.form.createLease')}
        </button>
      </div>
    </form>
  );
}
