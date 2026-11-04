/**
 * Lease lifecycle action dialogs: edit, renew, terminate and add amendment.
 * Epic 19: Lease Management & Tenant Screening
 *
 * Each dialog only collects + validates its form and hands typed data to the
 * caller; the route wrapper owns the API call, toasts and cache invalidation.
 * Terminate is a two-step flow: this form, then a `ConfirmationDialog` in the
 * route (it cannot be undone).
 */

import type { ReactNode } from 'react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

// ============================================================================
// Form data
// ============================================================================

export interface EditLeaseFormData {
  tenantPhone?: string;
  parkingSpaces: number;
  storageUnits: number;
  maxOccupants?: number;
  petsAllowed: boolean;
  smokingAllowed: boolean;
  notes?: string;
}

export interface RenewLeaseFormData {
  /** ISO date (YYYY-MM-DD). */
  newEndDate: string;
  newMonthlyRent: number;
  newSecurityDeposit?: number;
  termMonths: number;
  notes?: string;
}

export const TERMINATION_REASONS = [
  'end_of_term',
  'mutual_agreement',
  'tenant_breach',
  'landlord_breach',
  'property_sale',
  'renovation',
  'other',
] as const;

export type TerminationReason = (typeof TERMINATION_REASONS)[number];

export interface TerminateLeaseFormData {
  reason: TerminationReason;
  /** ISO date (YYYY-MM-DD). */
  effectiveDate: string;
  notes?: string;
}

export interface AmendmentFormData {
  title: string;
  description?: string;
  /** ISO date (YYYY-MM-DD). */
  effectiveDate: string;
  newMonthlyRent?: number;
}

// ============================================================================
// Helpers
// ============================================================================

export function todayIso(): string {
  return new Date().toISOString().split('T')[0];
}

/** Add whole days to an ISO date (UTC, no DST drift). */
export function addDaysIso(date: string, days: number): string {
  const d = new Date(`${date}T00:00:00Z`);
  d.setUTCDate(d.getUTCDate() + days);
  return d.toISOString().split('T')[0];
}

/** Add whole months to an ISO date (UTC). */
export function addMonthsIso(date: string, months: number): string {
  const d = new Date(`${date}T00:00:00Z`);
  d.setUTCMonth(d.getUTCMonth() + months);
  return d.toISOString().split('T')[0];
}

/**
 * Lease term in whole months from `start` up to and including `end`
 * (e.g. 2027-01-01 → 2027-12-31 is 12 months). Minimum 1.
 */
export function termMonthsBetween(start: string, end: string): number {
  const s = new Date(`${start}T00:00:00Z`);
  const e = new Date(`${addDaysIso(end, 1)}T00:00:00Z`);
  if (Number.isNaN(s.getTime()) || Number.isNaN(e.getTime())) return 1;
  const months =
    (e.getUTCFullYear() - s.getUTCFullYear()) * 12 + (e.getUTCMonth() - s.getUTCMonth());
  return Math.max(1, months);
}

const inputClass =
  'block w-full border border-gray-300 rounded-md shadow-sm py-2 px-3 focus:outline-none focus:ring-blue-500 focus:border-blue-500 sm:text-sm';
const labelClass = 'block text-sm font-medium text-gray-700 mb-1';

function ModalShell({
  id,
  title,
  description,
  children,
}: {
  id: string;
  title: string;
  description?: string;
  children: ReactNode;
}) {
  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div
        role="dialog"
        aria-modal="true"
        aria-labelledby={`${id}-title`}
        className="bg-white rounded-lg shadow-lg p-6 max-w-lg w-full mx-4 max-h-[90vh] overflow-y-auto"
      >
        <h2 id={`${id}-title`} className="text-lg font-medium text-gray-900 mb-1">
          {title}
        </h2>
        {description && <p className="text-sm text-gray-500 mb-4">{description}</p>}
        {children}
      </div>
    </div>
  );
}

function FormActions({
  isSubmitting,
  submitLabel,
  onClose,
  tone = 'primary',
}: {
  isSubmitting?: boolean;
  submitLabel: string;
  onClose: () => void;
  tone?: 'primary' | 'success' | 'danger';
}) {
  const { t } = useTranslation();
  const toneClass =
    tone === 'danger'
      ? 'bg-red-600 hover:bg-red-700'
      : tone === 'success'
        ? 'bg-green-600 hover:bg-green-700'
        : 'bg-blue-600 hover:bg-blue-700';
  return (
    <div className="flex justify-end gap-2 mt-6">
      <button
        type="button"
        onClick={onClose}
        disabled={isSubmitting}
        className="px-4 py-2 text-sm font-medium text-gray-700 bg-white border border-gray-300 rounded-md hover:bg-gray-50 disabled:opacity-50"
      >
        {t('common.cancel')}
      </button>
      <button
        type="submit"
        disabled={isSubmitting}
        className={`px-4 py-2 text-sm font-medium text-white rounded-md disabled:opacity-50 ${toneClass}`}
      >
        {isSubmitting ? t('common.processing') : submitLabel}
      </button>
    </div>
  );
}

function FieldError({ id, message }: { id: string; message?: string | null }) {
  if (!message) return null;
  return (
    <p id={id} role="alert" className="mt-1 text-sm text-red-600">
      {message}
    </p>
  );
}

/** Parse an optional non-negative integer field; `null` = invalid. */
function parseCount(value: string): number | undefined | null {
  if (value.trim() === '') return undefined;
  const n = Number(value);
  return Number.isInteger(n) && n >= 0 ? n : null;
}

/** Parse an optional positive decimal field; `null` = invalid. */
function parseAmount(value: string): number | undefined | null {
  if (value.trim() === '') return undefined;
  const n = Number(value);
  return Number.isFinite(n) && n > 0 ? n : null;
}

// ============================================================================
// Edit
// ============================================================================

interface EditLeaseModalProps {
  initial: EditLeaseFormData;
  isSubmitting?: boolean;
  onSubmit: (data: EditLeaseFormData) => void;
  onClose: () => void;
}

export function EditLeaseModal({ initial, isSubmitting, onSubmit, onClose }: EditLeaseModalProps) {
  const { t } = useTranslation();
  const [tenantPhone, setTenantPhone] = useState(initial.tenantPhone ?? '');
  const [parking, setParking] = useState(String(initial.parkingSpaces));
  const [storage, setStorage] = useState(String(initial.storageUnits));
  const [maxOccupants, setMaxOccupants] = useState(
    initial.maxOccupants != null ? String(initial.maxOccupants) : ''
  );
  const [petsAllowed, setPetsAllowed] = useState(initial.petsAllowed);
  const [smokingAllowed, setSmokingAllowed] = useState(initial.smokingAllowed);
  const [notes, setNotes] = useState(initial.notes ?? '');
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const parkingSpaces = parseCount(parking);
    const storageUnits = parseCount(storage);
    const occupants = parseCount(maxOccupants);
    if (parkingSpaces === null || storageUnits === null || occupants === null) {
      setError(t('leases.actions.edit.invalidNumber'));
      return;
    }
    setError(null);
    onSubmit({
      tenantPhone: tenantPhone.trim() || undefined,
      parkingSpaces: parkingSpaces ?? 0,
      storageUnits: storageUnits ?? 0,
      maxOccupants: occupants,
      petsAllowed,
      smokingAllowed,
      notes: notes.trim() || undefined,
    });
  };

  return (
    <ModalShell
      id="edit-lease"
      title={t('leases.actions.edit.title')}
      description={t('leases.actions.edit.description')}
    >
      <form onSubmit={handleSubmit} noValidate>
        <div className="mb-4">
          <label htmlFor="edit-lease-phone" className={labelClass}>
            {t('leases.actions.edit.tenantPhone')}
          </label>
          <input
            id="edit-lease-phone"
            type="tel"
            value={tenantPhone}
            onChange={(e) => setTenantPhone(e.target.value)}
            className={inputClass}
          />
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 mb-4">
          <div>
            <label htmlFor="edit-lease-parking" className={labelClass}>
              {t('leases.actions.edit.parkingSpaces')}
            </label>
            <input
              id="edit-lease-parking"
              type="number"
              min="0"
              step="1"
              value={parking}
              onChange={(e) => setParking(e.target.value)}
              className={inputClass}
            />
          </div>
          <div>
            <label htmlFor="edit-lease-storage" className={labelClass}>
              {t('leases.actions.edit.storageUnits')}
            </label>
            <input
              id="edit-lease-storage"
              type="number"
              min="0"
              step="1"
              value={storage}
              onChange={(e) => setStorage(e.target.value)}
              className={inputClass}
            />
          </div>
          <div>
            <label htmlFor="edit-lease-occupants" className={labelClass}>
              {t('leases.actions.edit.maxOccupants')}
            </label>
            <input
              id="edit-lease-occupants"
              type="number"
              min="0"
              step="1"
              value={maxOccupants}
              onChange={(e) => setMaxOccupants(e.target.value)}
              className={inputClass}
            />
          </div>
        </div>
        <FieldError id="edit-lease-error" message={error} />
        <div className="flex flex-wrap gap-6 mb-4">
          <label className="inline-flex items-center gap-2 text-sm text-gray-700">
            <input
              type="checkbox"
              checked={petsAllowed}
              onChange={(e) => setPetsAllowed(e.target.checked)}
            />
            {t('leases.actions.edit.petsAllowed')}
          </label>
          <label className="inline-flex items-center gap-2 text-sm text-gray-700">
            <input
              type="checkbox"
              checked={smokingAllowed}
              onChange={(e) => setSmokingAllowed(e.target.checked)}
            />
            {t('leases.actions.edit.smokingAllowed')}
          </label>
        </div>
        <div>
          <label htmlFor="edit-lease-notes" className={labelClass}>
            {t('leases.detail.notes')}
          </label>
          <textarea
            id="edit-lease-notes"
            rows={3}
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            className={inputClass}
          />
        </div>
        <FormActions
          isSubmitting={isSubmitting}
          submitLabel={t('leases.actions.edit.submit')}
          onClose={onClose}
        />
      </form>
    </ModalShell>
  );
}

// ============================================================================
// Renew
// ============================================================================

interface RenewLeaseModalProps {
  /** Current lease end date (ISO); the renewal starts the day after. */
  currentEndDate: string;
  currentRent: number;
  currency: string;
  isSubmitting?: boolean;
  onSubmit: (data: RenewLeaseFormData) => void;
  onClose: () => void;
}

export function RenewLeaseModal({
  currentEndDate,
  currentRent,
  currency,
  isSubmitting,
  onSubmit,
  onClose,
}: RenewLeaseModalProps) {
  const { t } = useTranslation();
  const newStart = addDaysIso(currentEndDate, 1);
  const [endDate, setEndDate] = useState(() => addDaysIso(addMonthsIso(newStart, 12), -1));
  const [rent, setRent] = useState(String(currentRent));
  const [deposit, setDeposit] = useState('');
  const [notes, setNotes] = useState('');
  const [errors, setErrors] = useState<{ endDate?: string; rent?: string; deposit?: string }>({});

  const termMonths = endDate > newStart ? termMonthsBetween(newStart, endDate) : 0;
  const startLabel = new Date(`${newStart}T00:00:00Z`).toLocaleDateString(undefined, {
    timeZone: 'UTC',
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const next: typeof errors = {};
    if (!endDate || endDate <= newStart) {
      next.endDate = t('leases.actions.renew.endDateInvalid', { date: startLabel });
    }
    const rentValue = parseAmount(rent);
    if (rentValue == null) next.rent = t('leases.actions.renew.rentInvalid');
    const depositValue = parseAmount(deposit);
    if (depositValue === null) next.deposit = t('leases.actions.renew.depositInvalid');
    setErrors(next);
    if (Object.keys(next).length > 0 || rentValue == null) return;
    onSubmit({
      newEndDate: endDate,
      newMonthlyRent: rentValue,
      newSecurityDeposit: depositValue ?? undefined,
      termMonths,
      notes: notes.trim() || undefined,
    });
  };

  return (
    <ModalShell
      id="renew-lease"
      title={t('leases.actions.renew.title')}
      description={t('leases.actions.renew.description', { date: startLabel })}
    >
      <form onSubmit={handleSubmit} noValidate>
        <div className="mb-4">
          <label htmlFor="renew-end-date" className={labelClass}>
            {t('leases.actions.renew.newEndDate')}
          </label>
          <input
            id="renew-end-date"
            type="date"
            min={addDaysIso(newStart, 1)}
            value={endDate}
            onChange={(e) => setEndDate(e.target.value)}
            aria-invalid={errors.endDate ? true : undefined}
            className={inputClass}
          />
          <FieldError id="renew-end-date-error" message={errors.endDate} />
          {termMonths > 0 && (
            <p className="mt-1 text-xs text-gray-500">
              {t('leases.actions.renew.term', { count: termMonths })}
            </p>
          )}
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 mb-4">
          <div>
            <label htmlFor="renew-rent" className={labelClass}>
              {t('leases.actions.renew.newMonthlyRent')} ({currency})
            </label>
            <input
              id="renew-rent"
              type="number"
              min="0"
              step="0.01"
              value={rent}
              onChange={(e) => setRent(e.target.value)}
              aria-invalid={errors.rent ? true : undefined}
              className={inputClass}
            />
            <FieldError id="renew-rent-error" message={errors.rent} />
          </div>
          <div>
            <label htmlFor="renew-deposit" className={labelClass}>
              {t('leases.actions.renew.newSecurityDeposit')} ({currency})
            </label>
            <input
              id="renew-deposit"
              type="number"
              min="0"
              step="0.01"
              value={deposit}
              onChange={(e) => setDeposit(e.target.value)}
              aria-invalid={errors.deposit ? true : undefined}
              className={inputClass}
            />
            <p className="mt-1 text-xs text-gray-500">{t('leases.actions.renew.depositHint')}</p>
            <FieldError id="renew-deposit-error" message={errors.deposit} />
          </div>
        </div>
        <div>
          <label htmlFor="renew-notes" className={labelClass}>
            {t('leases.detail.notes')}
          </label>
          <textarea
            id="renew-notes"
            rows={3}
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            className={inputClass}
          />
        </div>
        <FormActions
          isSubmitting={isSubmitting}
          submitLabel={t('leases.actions.renew.submit')}
          onClose={onClose}
          tone="success"
        />
      </form>
    </ModalShell>
  );
}

// ============================================================================
// Terminate (form step — confirmation is a ConfirmationDialog in the route)
// ============================================================================

interface TerminateLeaseModalProps {
  /** Pre-fill (e.g. when going back from the confirmation step). */
  initial?: Partial<TerminateLeaseFormData>;
  isSubmitting?: boolean;
  onSubmit: (data: TerminateLeaseFormData) => void;
  onClose: () => void;
}

export function TerminateLeaseModal({
  initial,
  isSubmitting,
  onSubmit,
  onClose,
}: TerminateLeaseModalProps) {
  const { t } = useTranslation();
  const [reason, setReason] = useState<TerminationReason | ''>(initial?.reason ?? '');
  const [effectiveDate, setEffectiveDate] = useState(initial?.effectiveDate ?? todayIso());
  const [notes, setNotes] = useState(initial?.notes ?? '');
  const [errors, setErrors] = useState<{ reason?: string; date?: string }>({});

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const next: typeof errors = {};
    if (!reason) next.reason = t('leases.actions.terminate.reasonRequired');
    if (!effectiveDate) next.date = t('leases.actions.terminate.dateRequired');
    setErrors(next);
    if (!reason || !effectiveDate) return;
    onSubmit({ reason, effectiveDate, notes: notes.trim() || undefined });
  };

  return (
    <ModalShell
      id="terminate-lease"
      title={t('leases.actions.terminate.title')}
      description={t('leases.actions.terminate.description')}
    >
      <form onSubmit={handleSubmit} noValidate>
        <div className="mb-4">
          <label htmlFor="terminate-reason" className={labelClass}>
            {t('leases.actions.terminate.reason')}
          </label>
          <select
            id="terminate-reason"
            value={reason}
            onChange={(e) => setReason(e.target.value as TerminationReason)}
            aria-invalid={errors.reason ? true : undefined}
            className={inputClass}
          >
            <option value="">{t('leases.actions.terminate.selectReason')}</option>
            {TERMINATION_REASONS.map((r) => (
              <option key={r} value={r}>
                {t(`leases.actions.terminate.reasons.${r}`)}
              </option>
            ))}
          </select>
          <FieldError id="terminate-reason-error" message={errors.reason} />
        </div>
        <div className="mb-4">
          <label htmlFor="terminate-date" className={labelClass}>
            {t('leases.actions.terminate.effectiveDate')}
          </label>
          <input
            id="terminate-date"
            type="date"
            value={effectiveDate}
            onChange={(e) => setEffectiveDate(e.target.value)}
            aria-invalid={errors.date ? true : undefined}
            className={inputClass}
          />
          <FieldError id="terminate-date-error" message={errors.date} />
        </div>
        <div>
          <label htmlFor="terminate-notes" className={labelClass}>
            {t('leases.detail.notes')}
          </label>
          <textarea
            id="terminate-notes"
            rows={3}
            value={notes}
            onChange={(e) => setNotes(e.target.value)}
            className={inputClass}
          />
        </div>
        <FormActions
          isSubmitting={isSubmitting}
          submitLabel={t('leases.actions.terminate.continue')}
          onClose={onClose}
          tone="danger"
        />
      </form>
    </ModalShell>
  );
}

// ============================================================================
// Amendment
// ============================================================================

interface AddAmendmentModalProps {
  currency: string;
  isSubmitting?: boolean;
  onSubmit: (data: AmendmentFormData) => void;
  onClose: () => void;
}

export function AddAmendmentModal({
  currency,
  isSubmitting,
  onSubmit,
  onClose,
}: AddAmendmentModalProps) {
  const { t } = useTranslation();
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [effectiveDate, setEffectiveDate] = useState(todayIso);
  const [rent, setRent] = useState('');
  const [errors, setErrors] = useState<{ title?: string; date?: string; rent?: string }>({});

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const next: typeof errors = {};
    if (!title.trim()) next.title = t('leases.actions.amendment.titleRequired');
    if (!effectiveDate) next.date = t('leases.actions.amendment.dateRequired');
    const rentValue = parseAmount(rent);
    if (rentValue === null) next.rent = t('leases.actions.renew.rentInvalid');
    setErrors(next);
    if (Object.keys(next).length > 0) return;
    onSubmit({
      title: title.trim(),
      description: description.trim() || undefined,
      effectiveDate,
      newMonthlyRent: rentValue ?? undefined,
    });
  };

  return (
    <ModalShell id="add-amendment" title={t('leases.actions.amendment.title')}>
      <form onSubmit={handleSubmit} noValidate>
        <div className="mb-4">
          <label htmlFor="amendment-title" className={labelClass}>
            {t('leases.actions.amendment.amendmentTitle')}
          </label>
          <input
            id="amendment-title"
            type="text"
            value={title}
            onChange={(e) => setTitle(e.target.value)}
            placeholder={t('leases.actions.amendment.titlePlaceholder')}
            aria-invalid={errors.title ? true : undefined}
            className={inputClass}
          />
          <FieldError id="amendment-title-error" message={errors.title} />
        </div>
        <div className="mb-4">
          <label htmlFor="amendment-description" className={labelClass}>
            {t('leases.actions.amendment.description')}
          </label>
          <textarea
            id="amendment-description"
            rows={3}
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            className={inputClass}
          />
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <div>
            <label htmlFor="amendment-date" className={labelClass}>
              {t('leases.detail.effectiveDate')}
            </label>
            <input
              id="amendment-date"
              type="date"
              value={effectiveDate}
              onChange={(e) => setEffectiveDate(e.target.value)}
              aria-invalid={errors.date ? true : undefined}
              className={inputClass}
            />
            <FieldError id="amendment-date-error" message={errors.date} />
          </div>
          <div>
            <label htmlFor="amendment-rent" className={labelClass}>
              {t('leases.actions.amendment.newMonthlyRent')} ({currency})
            </label>
            <input
              id="amendment-rent"
              type="number"
              min="0"
              step="0.01"
              value={rent}
              onChange={(e) => setRent(e.target.value)}
              aria-invalid={errors.rent ? true : undefined}
              className={inputClass}
            />
            <FieldError id="amendment-rent-error" message={errors.rent} />
          </div>
        </div>
        <p className="mt-2 text-xs text-gray-500">{t('leases.actions.amendment.rentHint')}</p>
        <FormActions
          isSubmitting={isSubmitting}
          submitLabel={t('leases.actions.amendment.submit')}
          onClose={onClose}
        />
      </form>
    </ModalShell>
  );
}
