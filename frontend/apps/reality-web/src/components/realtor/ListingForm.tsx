'use client';

/**
 * Reusable listing form (UC-51.4 / 51.5).
 *
 * Used by both the create and edit pages. Stays UI-only — the parent owns
 * the submit handler and any post-submit navigation.
 */

import { useTranslations } from 'next-intl';
import { type FormEvent, useState } from 'react';
import type { ListingDraft } from '@/lib/realtor-api';

interface ListingFormProps {
  initialValue?: Partial<ListingDraft>;
  submitLabel: string;
  isSubmitting?: boolean;
  generalError?: string;
  onSubmit: (draft: ListingDraft) => void | Promise<void>;
}

const PROPERTY_TYPES: ReadonlyArray<{ value: ListingDraft['propertyType']; labelKey: string }> = [
  { value: 'apartment', labelKey: 'propertyTypeApartment' },
  { value: 'house', labelKey: 'propertyTypeHouse' },
  { value: 'land', labelKey: 'propertyTypeLand' },
  { value: 'commercial', labelKey: 'propertyTypeCommercial' },
  { value: 'other', labelKey: 'propertyTypeOther' },
];

const TRANSACTION_TYPES: ReadonlyArray<{
  value: ListingDraft['transactionType'];
  labelKey: string;
}> = [
  { value: 'sale', labelKey: 'transactionSale' },
  { value: 'rent', labelKey: 'transactionRent' },
];

interface FieldErrors {
  title?: string;
  description?: string;
  price?: string;
  city?: string;
  area?: string;
  rooms?: string;
}

/**
 * Parse an optional numeric field from raw input.
 *
 * Empty input is valid and yields `undefined` (the field is optional).
 * Non-numeric input (which `Number()` would coerce to `NaN`) and negative
 * values are rejected so the form never submits `NaN` or a negative measure.
 */
function parseOptionalNonNegative(raw: string): {
  value?: number;
  error?: 'invalid' | 'negative';
} {
  const trimmed = raw.trim();
  if (!trimmed) return { value: undefined };
  const num = Number(trimmed);
  if (!Number.isFinite(num)) return { error: 'invalid' };
  if (num < 0) return { error: 'negative' };
  return { value: num };
}

export function ListingForm({
  initialValue,
  submitLabel,
  isSubmitting,
  generalError,
  onSubmit,
}: ListingFormProps) {
  const t = useTranslations('listingForm');
  const [title, setTitle] = useState(initialValue?.title ?? '');
  const [description, setDescription] = useState(initialValue?.description ?? '');
  const [propertyType, setPropertyType] = useState<ListingDraft['propertyType']>(
    initialValue?.propertyType ?? 'apartment'
  );
  const [transactionType, setTransactionType] = useState<ListingDraft['transactionType']>(
    initialValue?.transactionType ?? 'sale'
  );
  const [price, setPrice] = useState(initialValue?.price?.toString() ?? '');
  const [currency, setCurrency] = useState(initialValue?.currency ?? 'EUR');
  const [city, setCity] = useState(initialValue?.city ?? '');
  const [street, setStreet] = useState(initialValue?.street ?? '');
  const [postalCode, setPostalCode] = useState(initialValue?.postalCode ?? '');
  const [area, setArea] = useState(initialValue?.area?.toString() ?? '');
  const [rooms, setRooms] = useState(initialValue?.rooms?.toString() ?? '');
  const [errors, setErrors] = useState<FieldErrors>({});

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const next: FieldErrors = {};
    if (!title.trim()) next.title = t('errorTitleRequired');
    if (!description.trim()) next.description = t('errorDescriptionRequired');
    if (!city.trim()) next.city = t('errorCityRequired');
    const priceNum = Number(price);
    if (!Number.isFinite(priceNum) || priceNum <= 0) next.price = t('errorPricePositive');
    const areaResult = parseOptionalNonNegative(area);
    if (areaResult.error)
      next.area = t(areaResult.error === 'negative' ? 'errorAreaNegative' : 'errorAreaInvalid');
    const roomsResult = parseOptionalNonNegative(rooms);
    if (roomsResult.error)
      next.rooms = t(roomsResult.error === 'negative' ? 'errorRoomsNegative' : 'errorRoomsInvalid');
    setErrors(next);
    if (Object.keys(next).length > 0) return;

    await onSubmit({
      title: title.trim(),
      description: description.trim(),
      propertyType,
      transactionType,
      price: priceNum,
      currency,
      city: city.trim(),
      street: street.trim() || undefined,
      postalCode: postalCode.trim() || undefined,
      area: areaResult.value,
      rooms: roomsResult.value,
    });
  };

  return (
    <form className="form" onSubmit={handleSubmit} noValidate>
      {generalError && (
        <div className="alert" role="alert">
          {generalError}
        </div>
      )}

      <label className="field">
        <span className="label">{t('labelTitle')}</span>
        <input
          type="text"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          disabled={isSubmitting}
          className={`input ${errors.title ? 'input-error' : ''}`}
        />
        {errors.title && <span className="error">{errors.title}</span>}
      </label>

      <label className="field">
        <span className="label">{t('labelDescription')}</span>
        <textarea
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          disabled={isSubmitting}
          rows={5}
          className={`input ${errors.description ? 'input-error' : ''}`}
        />
        {errors.description && <span className="error">{errors.description}</span>}
      </label>

      <div className="row">
        <label className="field">
          <span className="label">{t('labelPropertyType')}</span>
          <select
            value={propertyType}
            onChange={(e) => setPropertyType(e.target.value as ListingDraft['propertyType'])}
            disabled={isSubmitting}
            className="input"
          >
            {PROPERTY_TYPES.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {t(opt.labelKey)}
              </option>
            ))}
          </select>
        </label>

        <label className="field">
          <span className="label">{t('labelTransaction')}</span>
          <select
            value={transactionType}
            onChange={(e) => setTransactionType(e.target.value as ListingDraft['transactionType'])}
            disabled={isSubmitting}
            className="input"
          >
            {TRANSACTION_TYPES.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {t(opt.labelKey)}
              </option>
            ))}
          </select>
        </label>
      </div>

      <div className="row">
        <label className="field">
          <span className="label">{t('labelPrice')}</span>
          <input
            type="number"
            min="0"
            step="0.01"
            value={price}
            onChange={(e) => setPrice(e.target.value)}
            disabled={isSubmitting}
            className={`input ${errors.price ? 'input-error' : ''}`}
          />
          {errors.price && <span className="error">{errors.price}</span>}
        </label>
        <label className="field">
          <span className="label">{t('labelCurrency')}</span>
          <input
            type="text"
            value={currency}
            onChange={(e) => setCurrency(e.target.value.toUpperCase().slice(0, 3))}
            disabled={isSubmitting}
            className="input"
          />
        </label>
      </div>

      <div className="row">
        <label className="field">
          <span className="label">{t('labelCity')}</span>
          <input
            type="text"
            value={city}
            onChange={(e) => setCity(e.target.value)}
            disabled={isSubmitting}
            className={`input ${errors.city ? 'input-error' : ''}`}
          />
          {errors.city && <span className="error">{errors.city}</span>}
        </label>
        <label className="field">
          <span className="label">{t('labelPostalCode')}</span>
          <input
            type="text"
            value={postalCode}
            onChange={(e) => setPostalCode(e.target.value)}
            disabled={isSubmitting}
            className="input"
          />
        </label>
      </div>

      <label className="field">
        <span className="label">{t('labelStreet')}</span>
        <input
          type="text"
          value={street}
          onChange={(e) => setStreet(e.target.value)}
          disabled={isSubmitting}
          className="input"
        />
      </label>

      <div className="row">
        <label className="field">
          <span className="label">{t('labelArea')}</span>
          <input
            type="number"
            min="0"
            value={area}
            onChange={(e) => setArea(e.target.value)}
            disabled={isSubmitting}
            className={`input ${errors.area ? 'input-error' : ''}`}
          />
          {errors.area && <span className="error">{errors.area}</span>}
        </label>
        <label className="field">
          <span className="label">{t('labelRooms')}</span>
          <input
            type="number"
            min="0"
            value={rooms}
            onChange={(e) => setRooms(e.target.value)}
            disabled={isSubmitting}
            className={`input ${errors.rooms ? 'input-error' : ''}`}
          />
          {errors.rooms && <span className="error">{errors.rooms}</span>}
        </label>
      </div>

      <button type="submit" className="submit" disabled={isSubmitting}>
        {isSubmitting ? t('saving') : submitLabel}
      </button>

      <style jsx>{`
        .form { display: flex; flex-direction: column; gap: 16px; max-width: 720px; }
        .alert { padding: 12px 16px; background: var(--ppt-color-danger-light); color: var(--ppt-color-danger-dark); border: 1px solid var(--ppt-color-danger); border-radius: 8px; font-size: 14px; }
        .row { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
        @media (max-width: 600px) { .row { grid-template-columns: 1fr; } }
        .field { display: flex; flex-direction: column; gap: 6px; }
        .label { font-size: 14px; font-weight: 500; color: var(--ppt-fg-secondary); }
        .input { padding: 10px 12px; font-size: 15px; border: 1px solid var(--ppt-border-strong); border-radius: 8px; background: var(--ppt-bg-surface); color: var(--ppt-fg-primary); font-family: inherit; }
        .input:focus { outline: none; border-color: var(--ppt-color-primary); box-shadow: 0 0 0 3px rgba(37,99,235,.1); }
        .input-error { border-color: var(--ppt-color-danger); }
        .error { color: var(--ppt-color-danger); font-size: 12px; }
        .submit {
          margin-top: 8px; padding: 12px 16px; background: var(--ppt-color-primary); color: var(--ppt-fg-on-accent);
          border: none; border-radius: 8px; font-size: 16px; font-weight: 600; cursor: pointer;
          align-self: flex-start;
        }
        .submit:hover:not(:disabled) { background: var(--ppt-color-primary-hover); }
        .submit:disabled { background: var(--ppt-brand-500); cursor: not-allowed; }
      `}</style>
    </form>
  );
}
