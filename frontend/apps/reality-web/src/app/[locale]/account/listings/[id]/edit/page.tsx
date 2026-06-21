'use client';

/**
 * Listing-edit multi-step wizard — pre-populated from existing listing.
 * Route: /[locale]/account/listings/[id]/edit
 * Screen-map: docs/screens/reality/listing-edit.md
 *
 * 5 steps: Type+location → Details → Photos → Price → Summary
 */

import { useParams } from 'next/navigation';
import { useEffect, useState } from 'react';
import { ProtectedRoute } from '@/components/auth';
import { Footer, Header } from '@/components/ui';
import { getMyListing, RealtorApiError, updateListing } from '@/lib/realtor-api';
import { EDIT_STEPS, type ListingEditFormData, PROPERTY_TYPE_OPTIONS } from './_mock';

// TODO: replace stepper with @ppt/ui-kit/Stepper once available
// TODO: replace file upload with @ppt/ui-kit/FileUpload once available
// TODO: replace radio cards with @ppt/ui-kit/RadioCards once available

const inputStyle: React.CSSProperties = {
  width: '100%',
  padding: '11px 14px',
  borderRadius: 8,
  border: '1px solid var(--ppt-border-default, #e5e7eb)',
  fontSize: '0.9375rem',
  background: 'var(--ppt-bg-surface)',
  color: 'var(--ppt-fg-primary)',
  boxSizing: 'border-box',
  outline: 'none',
};

function StepIndicator({ current }: { current: number }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', marginBottom: 32 }}>
      {EDIT_STEPS.map((step, idx) => (
        <div
          key={step.id}
          style={{
            display: 'flex',
            alignItems: 'center',
            flex: idx < EDIT_STEPS.length - 1 ? 1 : 'initial',
          }}
        >
          <div
            style={{
              width: 32,
              height: 32,
              borderRadius: '50%',
              background:
                step.id < current
                  ? 'var(--ppt-color-success, #10b981)'
                  : step.id === current
                    ? 'var(--ppt-color-primary, #2563eb)'
                    : 'var(--ppt-border-default, #e5e7eb)',
              color: step.id <= current ? '#fff' : 'var(--ppt-fg-muted, #9ca3af)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              fontWeight: 700,
              fontSize: '0.8125rem',
              flexShrink: 0,
            }}
          >
            {step.id < current ? '✓' : step.id}
          </div>
          {idx < EDIT_STEPS.length - 1 && (
            <div
              style={{
                flex: 1,
                height: 2,
                background:
                  step.id < current
                    ? 'var(--ppt-color-success, #10b981)'
                    : 'var(--ppt-border-default, #e5e7eb)',
                margin: '0 4px',
              }}
            />
          )}
        </div>
      ))}
    </div>
  );
}

const EMPTY_FORM: ListingEditFormData = {
  transactionType: 'sale',
  propertyType: 'apartment',
  address: '',
  city: '',
  area: '',
  rooms: '',
  floor: '',
  totalFloors: '',
  yearBuilt: '',
  description: '',
  existingPhotoUrls: [],
  newPhotos: [],
  price: '',
  currency: 'EUR',
  priceNegotiable: false,
  status: 'draft',
};

function EditWizardContent() {
  const params = useParams<{ id: string }>();
  const listingId = params?.id ?? '';

  const [step, setStep] = useState(1);
  const [form, setForm] = useState<ListingEditFormData>(EMPTY_FORM);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [removedPhotos, setRemovedPhotos] = useState<Set<string>>(new Set());

  useEffect(() => {
    if (!listingId) return;
    setLoading(true);
    getMyListing(listingId)
      .then((listing) => {
        setForm({
          transactionType: (listing.transactionType as 'sale' | 'rent') ?? 'sale',
          propertyType:
            (listing.propertyType as ListingEditFormData['propertyType']) ?? 'apartment',
          address: listing.street ?? '',
          city: listing.city ?? '',
          area: listing.area ?? '',
          rooms: listing.rooms ?? '',
          floor: listing.floor ?? '',
          totalFloors: listing.totalFloors ?? '',
          yearBuilt: '',
          description: listing.description ?? '',
          existingPhotoUrls: [],
          newPhotos: [],
          price: listing.price ?? '',
          currency: listing.currency ?? 'EUR',
          priceNegotiable: listing.isNegotiable ?? false,
          status: (listing.status as ListingEditFormData['status']) ?? 'draft',
        });
      })
      .catch((err) => {
        setLoadError(err instanceof RealtorApiError ? err.message : 'Načítanie inzerátu zlyhalo.');
      })
      .finally(() => setLoading(false));
  }, [listingId]);

  const update = (patch: Partial<ListingEditFormData>) => setForm((f) => ({ ...f, ...patch }));

  const handleSave = async () => {
    setSaving(true);
    setSaveError(null);
    try {
      await updateListing(listingId, {
        title: form.description?.slice(0, 80) || `${form.propertyType} ${form.city}`,
        description: form.description,
        propertyType: form.propertyType,
        transactionType: form.transactionType,
        price: typeof form.price === 'number' ? form.price : Number(form.price) || 0,
        currency: form.currency,
        street: form.address,
        city: form.city,
        area: typeof form.area === 'number' ? form.area : Number(form.area) || undefined,
        rooms: typeof form.rooms === 'number' ? form.rooms : Number(form.rooms) || undefined,
      });
      setSaved(true);
    } catch (err) {
      setSaveError(
        err instanceof RealtorApiError ? err.message : 'Uloženie zlyhalo, skúste znova.'
      );
    } finally {
      setSaving(false);
    }
  };

  if (loading) {
    return (
      <div style={{ maxWidth: 600, margin: '80px auto', textAlign: 'center', padding: '0 24px' }}>
        <p style={{ color: 'var(--ppt-fg-secondary)' }}>Načítavam inzerát…</p>
      </div>
    );
  }

  if (loadError) {
    return (
      <div style={{ maxWidth: 600, margin: '80px auto', textAlign: 'center', padding: '0 24px' }}>
        <p style={{ color: 'var(--ppt-color-danger, #ef4444)' }}>{loadError}</p>
        <a href="/account" style={{ color: 'var(--ppt-color-primary, #2563eb)' }}>
          ← Späť na účet
        </a>
      </div>
    );
  }

  if (saved) {
    return (
      <div style={{ maxWidth: 600, margin: '80px auto', textAlign: 'center', padding: '0 24px' }}>
        <div style={{ fontSize: '3rem', marginBottom: 16 }}>✅</div>
        <h2
          style={{
            fontSize: '1.5rem',
            fontWeight: 800,
            color: 'var(--ppt-fg-primary)',
            marginBottom: 12,
          }}
        >
          Inzerát bol uložený!
        </h2>
        <p style={{ color: 'var(--ppt-fg-secondary)', marginBottom: 24 }}>
          Zmeny v inzeráte #{listingId} boli úspešne uložené.
        </p>
        <div style={{ display: 'flex', gap: 12, justifyContent: 'center', flexWrap: 'wrap' }}>
          <a
            href={`/listings/${listingId}`}
            style={{
              padding: '11px 24px',
              background: 'var(--ppt-color-primary, #2563eb)',
              color: '#fff',
              borderRadius: 8,
              fontWeight: 700,
              textDecoration: 'none',
            }}
          >
            Zobraziť inzerát
          </a>
          <button
            type="button"
            onClick={() => {
              setSaved(false);
              setStep(1);
            }}
            style={{
              padding: '11px 24px',
              background: 'var(--ppt-bg-app)',
              border: '1px solid var(--ppt-border-default, #e5e7eb)',
              borderRadius: 8,
              fontWeight: 600,
              cursor: 'pointer',
            }}
          >
            Ďalšie úpravy
          </button>
        </div>
      </div>
    );
  }

  return (
    <div style={{ maxWidth: 900, margin: '0 auto', padding: '40px 24px' }}>
      {/* Header */}
      <div style={{ marginBottom: 28 }}>
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 12,
            marginBottom: 8,
            flexWrap: 'wrap',
          }}
        >
          <a
            href="/account"
            style={{
              color: 'var(--ppt-color-primary, #2563eb)',
              fontSize: '0.875rem',
              textDecoration: 'none',
            }}
          >
            ← Môj účet
          </a>
          <span style={{ color: 'var(--ppt-fg-muted, #9ca3af)', fontSize: '0.875rem' }}>/</span>
          <span style={{ fontSize: '0.875rem', color: 'var(--ppt-fg-secondary)' }}>
            Upraviť inzerát #{listingId}
          </span>
        </div>
        <h1
          style={{ fontSize: '1.5rem', fontWeight: 800, color: 'var(--ppt-fg-primary)', margin: 0 }}
        >
          Upraviť inzerát
        </h1>
      </div>

      <div
        style={{ display: 'grid', gridTemplateColumns: '1fr 240px', gap: 32, alignItems: 'start' }}
      >
        {/* Wizard */}
        <div
          style={{
            background: 'var(--ppt-bg-surface)',
            borderRadius: 14,
            padding: '32px',
            boxShadow: '0 1px 5px rgba(0,0,0,.07)',
          }}
        >
          <p style={{ color: 'var(--ppt-fg-secondary)', marginBottom: 20, fontSize: '0.9375rem' }}>
            Krok {step} z {EDIT_STEPS.length}: {EDIT_STEPS[step - 1]?.title}
          </p>
          {/* TODO: replace with @ppt/ui-kit/Stepper once available */}
          <StepIndicator current={step} />

          {/* Step 1 */}
          {step === 1 && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 18 }}>
              <div>
                <label
                  style={{
                    display: 'block',
                    fontWeight: 600,
                    color: 'var(--ppt-fg-primary)',
                    marginBottom: 8,
                    fontSize: '0.9375rem',
                  }}
                >
                  Typ transakcie
                </label>
                {/* TODO: replace with @ppt/ui-kit/RadioCards once available */}
                <div style={{ display: 'flex', gap: 10 }}>
                  {(['sale', 'rent'] as const).map((t) => (
                    <button
                      key={t}
                      type="button"
                      onClick={() => update({ transactionType: t })}
                      style={{
                        flex: 1,
                        padding: '11px',
                        borderRadius: 8,
                        border: `2px solid ${form.transactionType === t ? 'var(--ppt-color-primary, #2563eb)' : 'var(--ppt-border-default, #e5e7eb)'}`,
                        background:
                          form.transactionType === t
                            ? 'var(--ppt-color-primary-light, #dbeafe)'
                            : 'var(--ppt-bg-surface)',
                        color:
                          form.transactionType === t
                            ? 'var(--ppt-color-primary, #2563eb)'
                            : 'var(--ppt-fg-secondary)',
                        fontWeight: 600,
                        cursor: 'pointer',
                      }}
                    >
                      {t === 'sale' ? 'Predaj' : 'Prenájom'}
                    </button>
                  ))}
                </div>
              </div>
              <div>
                <label
                  style={{
                    display: 'block',
                    fontWeight: 600,
                    color: 'var(--ppt-fg-primary)',
                    marginBottom: 8,
                    fontSize: '0.9375rem',
                  }}
                >
                  Typ nehnuteľnosti
                </label>
                <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: 10 }}>
                  {PROPERTY_TYPE_OPTIONS.map((opt) => (
                    <button
                      key={opt.value}
                      type="button"
                      onClick={() => update({ propertyType: opt.value })}
                      style={{
                        padding: '11px',
                        borderRadius: 8,
                        border: `2px solid ${form.propertyType === opt.value ? 'var(--ppt-color-primary, #2563eb)' : 'var(--ppt-border-default, #e5e7eb)'}`,
                        background:
                          form.propertyType === opt.value
                            ? 'var(--ppt-color-primary-light, #dbeafe)'
                            : 'var(--ppt-bg-surface)',
                        color:
                          form.propertyType === opt.value
                            ? 'var(--ppt-color-primary, #2563eb)'
                            : 'var(--ppt-fg-secondary)',
                        fontWeight: 600,
                        cursor: 'pointer',
                      }}
                    >
                      {opt.label}
                    </button>
                  ))}
                </div>
              </div>
              {[
                { key: 'address', label: 'Adresa', type: 'text', placeholder: 'Ulica a číslo' },
                {
                  key: 'city',
                  label: 'Mesto / Obec',
                  type: 'text',
                  placeholder: 'Napr. Bratislava',
                },
              ].map((field) => (
                <div key={field.key}>
                  <label
                    style={{
                      display: 'block',
                      fontWeight: 600,
                      color: 'var(--ppt-fg-primary)',
                      marginBottom: 6,
                      fontSize: '0.9375rem',
                    }}
                  >
                    {field.label}
                  </label>
                  <input
                    type={field.type}
                    placeholder={field.placeholder}
                    value={form[field.key as keyof ListingEditFormData] as string}
                    onChange={(e) => update({ [field.key]: e.target.value })}
                    style={inputStyle}
                  />
                </div>
              ))}
            </div>
          )}

          {/* Step 2 */}
          {step === 2 && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
              {[
                { key: 'area', label: 'Plocha (m²)', placeholder: '72' },
                { key: 'rooms', label: 'Počet izieb', placeholder: '3' },
                { key: 'floor', label: 'Poschodie', placeholder: '4' },
                { key: 'totalFloors', label: 'Celkový počet poschodí', placeholder: '8' },
                { key: 'yearBuilt', label: 'Rok výstavby', placeholder: '2005' },
              ].map((field) => (
                <div key={field.key}>
                  <label
                    style={{
                      display: 'block',
                      fontWeight: 600,
                      color: 'var(--ppt-fg-primary)',
                      marginBottom: 6,
                      fontSize: '0.9375rem',
                    }}
                  >
                    {field.label}
                  </label>
                  <input
                    type="number"
                    placeholder={field.placeholder}
                    value={form[field.key as keyof ListingEditFormData] as string | number}
                    onChange={(e) =>
                      update({ [field.key]: e.target.value === '' ? '' : Number(e.target.value) })
                    }
                    style={inputStyle}
                  />
                </div>
              ))}
              <div>
                <label
                  style={{
                    display: 'block',
                    fontWeight: 600,
                    color: 'var(--ppt-fg-primary)',
                    marginBottom: 6,
                    fontSize: '0.9375rem',
                  }}
                >
                  Popis
                </label>
                <textarea
                  rows={5}
                  value={form.description}
                  onChange={(e) => update({ description: e.target.value })}
                  style={{ ...inputStyle, resize: 'vertical' }}
                />
              </div>
            </div>
          )}

          {/* Step 3 */}
          {step === 3 && (
            <div>
              <p style={{ color: 'var(--ppt-fg-secondary)', marginBottom: 16 }}>
                Existujúce fotografie
              </p>
              {/* TODO: replace with @ppt/ui-kit/FileUpload once available */}
              <div style={{ display: 'flex', flexWrap: 'wrap', gap: 10, marginBottom: 20 }}>
                {form.existingPhotoUrls
                  .filter((url) => !removedPhotos.has(url))
                  .map((url) => (
                    <div key={url} style={{ position: 'relative', width: 100, height: 80 }}>
                      {/* eslint-disable-next-line @next/next/no-img-element */}
                      <img
                        src={url}
                        alt="Foto"
                        style={{
                          width: '100%',
                          height: '100%',
                          objectFit: 'cover',
                          borderRadius: 6,
                        }}
                      />
                      <button
                        type="button"
                        onClick={() => setRemovedPhotos((s) => new Set([...s, url]))}
                        aria-label="Odstrániť fotografiu"
                        style={{
                          position: 'absolute',
                          top: 4,
                          right: 4,
                          width: 20,
                          height: 20,
                          borderRadius: '50%',
                          background: 'rgba(0,0,0,.6)',
                          color: '#fff',
                          border: 'none',
                          cursor: 'pointer',
                          fontSize: '0.6875rem',
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'center',
                          lineHeight: 1,
                        }}
                      >
                        ×
                      </button>
                    </div>
                  ))}
              </div>
              <label
                style={{
                  display: 'flex',
                  flexDirection: 'column',
                  alignItems: 'center',
                  justifyContent: 'center',
                  gap: 10,
                  padding: '28px 20px',
                  border: '2px dashed var(--ppt-border-default, #e5e7eb)',
                  borderRadius: 10,
                  cursor: 'pointer',
                  color: 'var(--ppt-fg-secondary)',
                  textAlign: 'center',
                }}
              >
                <span style={{ fontSize: '2rem' }}>📷</span>
                <span style={{ fontWeight: 600, fontSize: '0.9375rem' }}>
                  Pridať nové fotografie
                </span>
                <input
                  type="file"
                  multiple
                  accept="image/*"
                  style={{ display: 'none' }}
                  onChange={(e) => {
                    if (e.target.files)
                      update({ newPhotos: [...form.newPhotos, ...Array.from(e.target.files)] });
                  }}
                />
              </label>
              {form.newPhotos.length > 0 && (
                <p
                  style={{
                    marginTop: 10,
                    color: 'var(--ppt-color-success, #10b981)',
                    fontSize: '0.875rem',
                  }}
                >
                  + {form.newPhotos.length} nových fotografií
                </p>
              )}
            </div>
          )}

          {/* Step 4 */}
          {step === 4 && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 18 }}>
              <div>
                <label
                  style={{
                    display: 'block',
                    fontWeight: 600,
                    color: 'var(--ppt-fg-primary)',
                    marginBottom: 6,
                    fontSize: '0.9375rem',
                  }}
                >
                  Cena (€)
                </label>
                <input
                  type="number"
                  value={form.price}
                  onChange={(e) =>
                    update({ price: e.target.value === '' ? '' : Number(e.target.value) })
                  }
                  style={inputStyle}
                />
              </div>
              <label style={{ display: 'flex', alignItems: 'center', gap: 10, cursor: 'pointer' }}>
                <input
                  type="checkbox"
                  checked={form.priceNegotiable}
                  onChange={(e) => update({ priceNegotiable: e.target.checked })}
                  style={{
                    width: 18,
                    height: 18,
                    accentColor: 'var(--ppt-color-primary, #2563eb)',
                  }}
                />
                <span style={{ fontWeight: 500, color: 'var(--ppt-fg-primary)' }}>
                  Cena je dohodou
                </span>
              </label>
              <div>
                <label
                  style={{
                    display: 'block',
                    fontWeight: 600,
                    color: 'var(--ppt-fg-primary)',
                    marginBottom: 8,
                    fontSize: '0.9375rem',
                  }}
                >
                  Stav inzerátu
                </label>
                {/* TODO: replace with @ppt/ui-kit/RadioCards once available */}
                <div style={{ display: 'flex', gap: 10, flexWrap: 'wrap' }}>
                  {(
                    [
                      { value: 'active', label: 'Aktívny' },
                      { value: 'inactive', label: 'Neaktívny' },
                      { value: 'draft', label: 'Koncept' },
                    ] as const
                  ).map((opt) => (
                    <button
                      key={opt.value}
                      type="button"
                      onClick={() => update({ status: opt.value })}
                      style={{
                        padding: '9px 18px',
                        borderRadius: 8,
                        border: `2px solid ${form.status === opt.value ? 'var(--ppt-color-primary, #2563eb)' : 'var(--ppt-border-default, #e5e7eb)'}`,
                        background:
                          form.status === opt.value
                            ? 'var(--ppt-color-primary-light, #dbeafe)'
                            : 'var(--ppt-bg-surface)',
                        color:
                          form.status === opt.value
                            ? 'var(--ppt-color-primary, #2563eb)'
                            : 'var(--ppt-fg-secondary)',
                        fontWeight: 600,
                        cursor: 'pointer',
                        fontSize: '0.875rem',
                      }}
                    >
                      {opt.label}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          )}

          {/* Step 5 — Summary */}
          {step === 5 && (
            <div>
              <h3
                style={{
                  fontSize: '1rem',
                  fontWeight: 700,
                  color: 'var(--ppt-fg-primary)',
                  marginBottom: 14,
                }}
              >
                Súhrn zmien
              </h3>
              <div
                style={{
                  background: 'var(--ppt-bg-subtle, #f8fafc)',
                  borderRadius: 8,
                  padding: '16px',
                  fontSize: '0.875rem',
                  color: 'var(--ppt-fg-secondary)',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: 8,
                }}
              >
                <div>
                  <strong>Typ:</strong> {form.transactionType === 'sale' ? 'Predaj' : 'Prenájom'} ·{' '}
                  {PROPERTY_TYPE_OPTIONS.find((o) => o.value === form.propertyType)?.label}
                </div>
                <div>
                  <strong>Poloha:</strong> {form.address}, {form.city}
                </div>
                <div>
                  <strong>Plocha:</strong> {form.area} m² · <strong>Izby:</strong> {form.rooms}
                </div>
                <div>
                  <strong>Poschodie:</strong> {form.floor}/{form.totalFloors} ·{' '}
                  <strong>Rok:</strong> {form.yearBuilt}
                </div>
                <div>
                  <strong>Cena:</strong>{' '}
                  {typeof form.price === 'number' ? form.price.toLocaleString('sk-SK') : '–'} €{' '}
                  {form.priceNegotiable ? '(dohodou)' : ''}
                </div>
                <div>
                  <strong>Stav:</strong>{' '}
                  {form.status === 'active'
                    ? 'Aktívny'
                    : form.status === 'inactive'
                      ? 'Neaktívny'
                      : 'Koncept'}
                </div>
                <div>
                  <strong>Fotografie:</strong> {form.existingPhotoUrls.length - removedPhotos.size}{' '}
                  existujúcich + {form.newPhotos.length} nových
                </div>
              </div>
            </div>
          )}

          {/* Navigation */}
          <div style={{ display: 'flex', justifyContent: 'space-between', marginTop: 28, gap: 10 }}>
            {step > 1 ? (
              <button
                type="button"
                onClick={() => setStep((s) => s - 1)}
                style={{
                  padding: '11px 20px',
                  background: 'var(--ppt-bg-app)',
                  border: '1px solid var(--ppt-border-default, #e5e7eb)',
                  borderRadius: 8,
                  fontWeight: 600,
                  cursor: 'pointer',
                  color: 'var(--ppt-fg-primary)',
                }}
              >
                ← Späť
              </button>
            ) : (
              <div />
            )}
            {step < EDIT_STEPS.length ? (
              <button
                type="button"
                onClick={() => setStep((s) => s + 1)}
                style={{
                  padding: '11px 24px',
                  background: 'var(--ppt-color-primary, #2563eb)',
                  color: '#fff',
                  border: 'none',
                  borderRadius: 8,
                  fontWeight: 700,
                  cursor: 'pointer',
                }}
              >
                Ďalej →
              </button>
            ) : (
              <div
                style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-end', gap: 8 }}
              >
                {saveError && (
                  <p
                    style={{
                      color: 'var(--ppt-color-danger, #ef4444)',
                      fontSize: '0.875rem',
                      margin: 0,
                    }}
                  >
                    {saveError}
                  </p>
                )}
                <button
                  type="button"
                  onClick={handleSave}
                  disabled={saving}
                  style={{
                    padding: '11px 28px',
                    background: saving
                      ? 'var(--ppt-border-default, #e5e7eb)'
                      : 'var(--ppt-color-success, #10b981)',
                    color: saving ? 'var(--ppt-fg-muted, #9ca3af)' : '#fff',
                    border: 'none',
                    borderRadius: 8,
                    fontWeight: 700,
                    cursor: saving ? 'not-allowed' : 'pointer',
                  }}
                >
                  {saving ? 'Ukladám…' : 'Uložiť zmeny'}
                </button>
              </div>
            )}
          </div>
        </div>

        {/* Aside — step list */}
        <aside
          style={{
            background: 'var(--ppt-bg-surface)',
            borderRadius: 12,
            padding: '24px',
            boxShadow: '0 1px 4px rgba(0,0,0,.07)',
            position: 'sticky',
            top: 24,
          }}
        >
          <h2
            style={{
              fontSize: '0.9375rem',
              fontWeight: 700,
              color: 'var(--ppt-fg-primary)',
              margin: '0 0 14px',
            }}
          >
            Kroky
          </h2>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
            {EDIT_STEPS.map((s) => (
              <div
                key={s.id}
                style={{
                  display: 'flex',
                  gap: 10,
                  alignItems: 'center',
                  opacity: s.id > step ? 0.5 : 1,
                }}
              >
                <div
                  style={{
                    width: 24,
                    height: 24,
                    borderRadius: '50%',
                    flexShrink: 0,
                    background:
                      s.id < step
                        ? 'var(--ppt-color-success, #10b981)'
                        : s.id === step
                          ? 'var(--ppt-color-primary, #2563eb)'
                          : 'var(--ppt-border-default, #e5e7eb)',
                    color: s.id <= step ? '#fff' : 'var(--ppt-fg-muted, #9ca3af)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    fontSize: '0.6875rem',
                    fontWeight: 700,
                  }}
                >
                  {s.id < step ? '✓' : s.id}
                </div>
                <div>
                  <div
                    style={{
                      fontWeight: s.id === step ? 700 : 500,
                      color: 'var(--ppt-fg-primary)',
                      fontSize: '0.8125rem',
                    }}
                  >
                    {s.title}
                  </div>
                  <div style={{ fontSize: '0.75rem', color: 'var(--ppt-fg-muted, #9ca3af)' }}>
                    {s.description}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </aside>
      </div>
    </div>
  );
}

export default function ListingEditPage() {
  return (
    <div
      style={{
        minHeight: '100vh',
        display: 'flex',
        flexDirection: 'column',
        background: 'var(--ppt-bg-app)',
      }}
    >
      <Header />
      <main style={{ flex: 1 }}>
        <ProtectedRoute>
          <EditWizardContent />
        </ProtectedRoute>
      </main>
      <Footer />
    </div>
  );
}
