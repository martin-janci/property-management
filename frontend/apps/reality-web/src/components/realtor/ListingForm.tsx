'use client';

/**
 * Reusable listing form (UC-51.4 / 51.5).
 *
 * Used by both the create and edit pages. Stays UI-only — the parent owns
 * the submit handler and any post-submit navigation.
 */

import { type FormEvent, useState } from 'react';
import type { ListingDraft } from '@/lib/realtor-api';

interface ListingFormProps {
  initialValue?: Partial<ListingDraft>;
  submitLabel: string;
  isSubmitting?: boolean;
  generalError?: string;
  onSubmit: (draft: ListingDraft) => void | Promise<void>;
}

const PROPERTY_TYPES: ReadonlyArray<{ value: ListingDraft['propertyType']; label: string }> = [
  { value: 'apartment', label: 'Apartment' },
  { value: 'house', label: 'House' },
  { value: 'land', label: 'Land' },
  { value: 'commercial', label: 'Commercial' },
  { value: 'other', label: 'Other' },
];

const TRANSACTION_TYPES: ReadonlyArray<{ value: ListingDraft['transactionType']; label: string }> =
  [
    { value: 'sale', label: 'For sale' },
    { value: 'rent', label: 'For rent' },
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
function parseOptionalNonNegative(raw: string): { value?: number; error?: string } {
  const trimmed = raw.trim();
  if (!trimmed) return { value: undefined };
  const num = Number(trimmed);
  if (!Number.isFinite(num)) return { error: 'must be a valid number' };
  if (num < 0) return { error: 'must not be negative' };
  return { value: num };
}

export function ListingForm({
  initialValue,
  submitLabel,
  isSubmitting,
  generalError,
  onSubmit,
}: ListingFormProps) {
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
    if (!title.trim()) next.title = 'Title is required';
    if (!description.trim()) next.description = 'Description is required';
    if (!city.trim()) next.city = 'City is required';
    const priceNum = Number(price);
    if (!Number.isFinite(priceNum) || priceNum <= 0) next.price = 'Price must be a positive number';
    const areaResult = parseOptionalNonNegative(area);
    if (areaResult.error) next.area = `Area ${areaResult.error}`;
    const roomsResult = parseOptionalNonNegative(rooms);
    if (roomsResult.error) next.rooms = `Rooms ${roomsResult.error}`;
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
        <span className="label">Title</span>
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
        <span className="label">Description</span>
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
          <span className="label">Property type</span>
          <select
            value={propertyType}
            onChange={(e) => setPropertyType(e.target.value as ListingDraft['propertyType'])}
            disabled={isSubmitting}
            className="input"
          >
            {PROPERTY_TYPES.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </label>

        <label className="field">
          <span className="label">Transaction</span>
          <select
            value={transactionType}
            onChange={(e) => setTransactionType(e.target.value as ListingDraft['transactionType'])}
            disabled={isSubmitting}
            className="input"
          >
            {TRANSACTION_TYPES.map((opt) => (
              <option key={opt.value} value={opt.value}>
                {opt.label}
              </option>
            ))}
          </select>
        </label>
      </div>

      <div className="row">
        <label className="field">
          <span className="label">Price</span>
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
          <span className="label">Currency</span>
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
          <span className="label">City</span>
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
          <span className="label">Postal code</span>
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
        <span className="label">Street</span>
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
          <span className="label">Area (m²)</span>
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
          <span className="label">Rooms</span>
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
        {isSubmitting ? 'Saving…' : submitLabel}
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
