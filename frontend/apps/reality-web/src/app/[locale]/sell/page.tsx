'use client';

/**
 * Sell / publish listing wizard — Reality Portal.
 * Screen-map: docs/screens/reality/sell.md
 *
 * 5-step wizard: Type+location → Details → Photos → Price → Contact+summary
 *
 * All user-facing strings come from `pages.sell.*` in `messages/<locale>.json`.
 */

import { useTranslations } from 'next-intl';
import { useState } from 'react';
import { Footer, Header } from '@/components/ui';
import { INITIAL_FORM_DATA, PROPERTY_TYPES, SELL_STEPS, type SellFormData } from './_mock';

// TODO: replace stepper with @ppt/ui-kit/Stepper once available
// TODO: replace file upload with @ppt/ui-kit/FileUpload once available
// TODO: replace radio cards with @ppt/ui-kit/RadioCards once available

function StepIndicator({ current }: { current: number }) {
  return (
    <div style={{ display: 'flex', gap: 0, alignItems: 'center', marginBottom: 40 }}>
      {SELL_STEPS.map((step, idx) => (
        <div
          key={step.id}
          style={{
            display: 'flex',
            alignItems: 'center',
            flex: idx < SELL_STEPS.length - 1 ? 1 : 'initial',
          }}
        >
          <div
            style={{
              width: 36,
              height: 36,
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
              fontSize: '0.875rem',
              flexShrink: 0,
            }}
          >
            {step.id < current ? '✓' : step.id}
          </div>
          {idx < SELL_STEPS.length - 1 && (
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

const labelStyle: React.CSSProperties = {
  display: 'block',
  fontWeight: 600,
  color: 'var(--ppt-fg-primary)',
  marginBottom: 6,
  fontSize: '0.9375rem',
};

export default function SellPage() {
  const t = useTranslations('pages.sell');
  const [step, setStep] = useState(1);
  const [form, setForm] = useState<SellFormData>(INITIAL_FORM_DATA);
  const [submitted, setSubmitted] = useState(false);

  const update = (patch: Partial<SellFormData>) => setForm((f) => ({ ...f, ...patch }));

  const currentStepInfo = SELL_STEPS[step - 1];
  const currentStepTitle = currentStepInfo ? t(`steps.${currentStepInfo.key}.title`) : '';

  if (submitted) {
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
        <main
          style={{
            flex: 1,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            padding: '48px 24px',
            textAlign: 'center',
          }}
        >
          <div>
            <div style={{ fontSize: '4rem', marginBottom: 16 }}>🎉</div>
            <h1
              style={{
                fontSize: '1.75rem',
                fontWeight: 800,
                color: 'var(--ppt-fg-primary)',
                marginBottom: 12,
              }}
            >
              {t('submittedTitle')}
            </h1>
            <p style={{ color: 'var(--ppt-fg-secondary)', marginBottom: 28 }}>
              {t('submittedBody')}
            </p>
            <button
              type="button"
              onClick={() => {
                setSubmitted(false);
                setStep(1);
                setForm(INITIAL_FORM_DATA);
              }}
              style={{
                padding: '12px 28px',
                background: 'var(--ppt-color-primary, #2563eb)',
                color: '#fff',
                border: 'none',
                borderRadius: 8,
                fontWeight: 700,
                cursor: 'pointer',
              }}
            >
              {t('submittedCta')}
            </button>
          </div>
        </main>
        <Footer />
      </div>
    );
  }

  return (
    <div
      data-i18n="pages.sell.root"
      style={{
        minHeight: '100vh',
        display: 'flex',
        flexDirection: 'column',
        background: 'var(--ppt-bg-app)',
      }}
    >
      <Header />

      <main style={{ flex: 1, padding: '48px 24px' }}>
        <div className="sell-grid">
          {/* Wizard form */}
          <div
            style={{
              background: 'var(--ppt-bg-surface)',
              borderRadius: 14,
              padding: '36px 32px',
              boxShadow: '0 1px 6px rgba(0,0,0,.08)',
            }}
          >
            <h1
              style={{
                fontSize: '1.5rem',
                fontWeight: 800,
                color: 'var(--ppt-fg-primary)',
                margin: '0 0 6px',
              }}
            >
              {t('title')}
            </h1>
            <p
              style={{ color: 'var(--ppt-fg-secondary)', marginBottom: 28, fontSize: '0.9375rem' }}
            >
              {t('stepLabel', {
                step,
                total: SELL_STEPS.length,
                stepTitle: currentStepTitle,
              })}
            </p>

            {/* TODO: replace with @ppt/ui-kit/Stepper once available */}
            <StepIndicator current={step} />

            {/* Step 1: Type + Location */}
            {step === 1 && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
                <div>
                  <label style={{ ...labelStyle, marginBottom: 8 }}>
                    {t('fields.transactionType')}
                  </label>
                  <div style={{ display: 'flex', gap: 10 }}>
                    {(['sale', 'rent'] as const).map((tx) => (
                      <button
                        key={tx}
                        type="button"
                        onClick={() => update({ transactionType: tx })}
                        style={{
                          flex: 1,
                          padding: '12px',
                          borderRadius: 8,
                          border: `2px solid ${form.transactionType === tx ? 'var(--ppt-color-primary, #2563eb)' : 'var(--ppt-border-default, #e5e7eb)'}`,
                          background:
                            form.transactionType === tx
                              ? 'var(--ppt-color-primary-light, #dbeafe)'
                              : 'var(--ppt-bg-surface)',
                          color:
                            form.transactionType === tx
                              ? 'var(--ppt-color-primary, #2563eb)'
                              : 'var(--ppt-fg-secondary)',
                          fontWeight: 600,
                          cursor: 'pointer',
                          fontSize: '0.9375rem',
                        }}
                      >
                        {t(`transactionOptions.${tx}`)}
                      </button>
                    ))}
                  </div>
                </div>

                <div>
                  <label style={{ ...labelStyle, marginBottom: 8 }}>
                    {t('fields.propertyType')}
                  </label>
                  <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: 10 }}>
                    {PROPERTY_TYPES.map((value) => (
                      <button
                        key={value}
                        type="button"
                        onClick={() => update({ propertyType: value })}
                        style={{
                          padding: '12px',
                          borderRadius: 8,
                          border: `2px solid ${form.propertyType === value ? 'var(--ppt-color-primary, #2563eb)' : 'var(--ppt-border-default, #e5e7eb)'}`,
                          background:
                            form.propertyType === value
                              ? 'var(--ppt-color-primary-light, #dbeafe)'
                              : 'var(--ppt-bg-surface)',
                          color:
                            form.propertyType === value
                              ? 'var(--ppt-color-primary, #2563eb)'
                              : 'var(--ppt-fg-secondary)',
                          fontWeight: 600,
                          cursor: 'pointer',
                        }}
                      >
                        {t(`propertyOptions.${value}`)}
                      </button>
                    ))}
                  </div>
                </div>

                <div>
                  <label style={labelStyle}>{t('fields.address')}</label>
                  <input
                    type="text"
                    placeholder={t('fields.addressPlaceholder')}
                    value={form.address}
                    onChange={(e) => update({ address: e.target.value })}
                    style={inputStyle}
                  />
                </div>

                <div>
                  <label style={labelStyle}>{t('fields.city')}</label>
                  <input
                    type="text"
                    placeholder={t('fields.cityPlaceholder')}
                    value={form.city}
                    onChange={(e) => update({ city: e.target.value })}
                    style={inputStyle}
                  />
                </div>
              </div>
            )}

            {/* Step 2: Details */}
            {step === 2 && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
                {(
                  [
                    { key: 'area', labelKey: 'area', placeholderKey: 'areaPlaceholder' },
                    { key: 'rooms', labelKey: 'rooms', placeholderKey: 'roomsPlaceholder' },
                    { key: 'floor', labelKey: 'floor', placeholderKey: 'floorPlaceholder' },
                    {
                      key: 'totalFloors',
                      labelKey: 'totalFloors',
                      placeholderKey: 'totalFloorsPlaceholder',
                    },
                    {
                      key: 'yearBuilt',
                      labelKey: 'yearBuilt',
                      placeholderKey: 'yearBuiltPlaceholder',
                    },
                  ] as const
                ).map((field) => (
                  <div key={field.key}>
                    <label style={labelStyle}>{t(`fields.${field.labelKey}`)}</label>
                    <input
                      type="number"
                      placeholder={t(`fields.${field.placeholderKey}`)}
                      value={form[field.key as keyof SellFormData] as string | number}
                      onChange={(e) =>
                        update({
                          [field.key]: e.target.value === '' ? '' : Number(e.target.value),
                        })
                      }
                      style={inputStyle}
                    />
                  </div>
                ))}
                <div>
                  <label style={labelStyle}>{t('fields.description')}</label>
                  <textarea
                    rows={5}
                    placeholder={t('fields.descriptionPlaceholder')}
                    value={form.description}
                    onChange={(e) => update({ description: e.target.value })}
                    style={{ ...inputStyle, resize: 'vertical' }}
                  />
                </div>
              </div>
            )}

            {/* Step 3: Photos */}
            {step === 3 && (
              <div>
                <p style={{ color: 'var(--ppt-fg-secondary)', marginBottom: 16 }}>
                  {t('photos.intro')}
                </p>
                <label
                  style={{
                    display: 'flex',
                    flexDirection: 'column',
                    alignItems: 'center',
                    justifyContent: 'center',
                    gap: 12,
                    padding: '40px 20px',
                    border: '2px dashed var(--ppt-border-default, #e5e7eb)',
                    borderRadius: 10,
                    cursor: 'pointer',
                    color: 'var(--ppt-fg-secondary)',
                    textAlign: 'center',
                  }}
                >
                  <span style={{ fontSize: '2.5rem' }}>📷</span>
                  <span style={{ fontWeight: 600 }}>{t('photos.cta')}</span>
                  <span style={{ fontSize: '0.875rem' }}>{t('photos.formats')}</span>
                  <input
                    type="file"
                    multiple
                    accept="image/*"
                    style={{ display: 'none' }}
                    onChange={(e) => {
                      if (e.target.files) update({ photos: Array.from(e.target.files) });
                    }}
                  />
                </label>
                {form.photos.length > 0 && (
                  <p
                    style={{
                      marginTop: 12,
                      color: 'var(--ppt-color-success, #10b981)',
                      fontWeight: 500,
                    }}
                  >
                    {t(form.photos.length === 1 ? 'photos.selected_one' : 'photos.selected_other', {
                      count: form.photos.length,
                    })}
                  </p>
                )}
              </div>
            )}

            {/* Step 4: Price */}
            {step === 4 && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
                <div>
                  <label style={labelStyle}>{t('fields.price')}</label>
                  <input
                    type="number"
                    placeholder={t(
                      form.transactionType === 'rent'
                        ? 'fields.priceRentPlaceholder'
                        : 'fields.priceSalePlaceholder',
                    )}
                    value={form.price}
                    onChange={(e) =>
                      update({ price: e.target.value === '' ? '' : Number(e.target.value) })
                    }
                    style={inputStyle}
                  />
                </div>
                <label
                  style={{ display: 'flex', alignItems: 'center', gap: 10, cursor: 'pointer' }}
                >
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
                  <span style={{ color: 'var(--ppt-fg-primary)', fontWeight: 500 }}>
                    {t('fields.priceNegotiable')}
                  </span>
                </label>
              </div>
            )}

            {/* Step 5: Contact + summary */}
            {step === 5 && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
                {/* Summary */}
                <div
                  style={{
                    background: 'var(--ppt-bg-subtle, #f8fafc)',
                    borderRadius: 8,
                    padding: '16px',
                    fontSize: '0.875rem',
                    color: 'var(--ppt-fg-secondary)',
                    display: 'flex',
                    flexDirection: 'column',
                    gap: 6,
                  }}
                >
                  <div>
                    <strong>{t('summary.type')}:</strong>{' '}
                    {t(`transactionOptions.${form.transactionType}`)} ·{' '}
                    {t(`propertyOptions.${form.propertyType}`)}
                  </div>
                  <div>
                    <strong>{t('summary.location')}:</strong> {form.address || '–'},{' '}
                    {form.city || '–'}
                  </div>
                  <div>
                    <strong>{t('summary.area')}:</strong> {form.area || '–'} m² ·{' '}
                    <strong>{t('summary.rooms')}:</strong> {form.rooms || '–'}
                  </div>
                  <div>
                    <strong>{t('summary.price')}:</strong>{' '}
                    {form.price ? `${Number(form.price).toLocaleString('sk-SK')} €` : '–'}{' '}
                    {form.priceNegotiable ? t('summary.negotiable') : ''}
                  </div>
                  <div>
                    <strong>{t('summary.photos')}:</strong> {form.photos.length}
                  </div>
                </div>

                {(
                  [
                    {
                      key: 'contactName',
                      labelKey: 'contactName',
                      placeholderKey: 'contactNamePlaceholder',
                      inputType: 'text',
                    },
                    {
                      key: 'contactPhone',
                      labelKey: 'contactPhone',
                      placeholderKey: 'contactPhonePlaceholder',
                      inputType: 'tel',
                    },
                    {
                      key: 'contactEmail',
                      labelKey: 'contactEmail',
                      placeholderKey: 'contactEmailPlaceholder',
                      inputType: 'email',
                    },
                  ] as const
                ).map((field) => (
                  <div key={field.key}>
                    <label style={labelStyle}>{t(`fields.${field.labelKey}`)}</label>
                    <input
                      type={field.inputType}
                      placeholder={t(`fields.${field.placeholderKey}`)}
                      value={form[field.key as keyof SellFormData] as string}
                      onChange={(e) => update({ [field.key]: e.target.value })}
                      style={inputStyle}
                    />
                  </div>
                ))}

                <label
                  style={{ display: 'flex', alignItems: 'flex-start', gap: 10, cursor: 'pointer' }}
                >
                  <input
                    type="checkbox"
                    checked={form.termsAccepted}
                    onChange={(e) => update({ termsAccepted: e.target.checked })}
                    style={{
                      width: 18,
                      height: 18,
                      marginTop: 2,
                      accentColor: 'var(--ppt-color-primary, #2563eb)',
                    }}
                  />
                  <span
                    style={{
                      color: 'var(--ppt-fg-secondary)',
                      fontSize: '0.875rem',
                      lineHeight: 1.5,
                    }}
                  >
                    {t.rich('terms.label', {
                      termsLink: (chunks) => (
                        <a href="/terms" style={{ color: 'var(--ppt-color-primary, #2563eb)' }}>
                          {chunks}
                        </a>
                      ),
                      privacyLink: (chunks) => (
                        <a href="/privacy" style={{ color: 'var(--ppt-color-primary, #2563eb)' }}>
                          {chunks}
                        </a>
                      ),
                    })}
                  </span>
                </label>
              </div>
            )}

            {/* Navigation */}
            <div
              style={{ display: 'flex', justifyContent: 'space-between', marginTop: 32, gap: 12 }}
            >
              {step > 1 && (
                <button
                  type="button"
                  onClick={() => setStep((s) => s - 1)}
                  style={{
                    padding: '11px 22px',
                    background: 'var(--ppt-bg-app, #f1f5f9)',
                    border: '1px solid var(--ppt-border-default, #e5e7eb)',
                    borderRadius: 8,
                    fontWeight: 600,
                    cursor: 'pointer',
                    color: 'var(--ppt-fg-primary)',
                  }}
                >
                  {t('back')}
                </button>
              )}
              <div style={{ flex: 1 }} />
              {step < SELL_STEPS.length ? (
                <button
                  type="button"
                  onClick={() => setStep((s) => s + 1)}
                  style={{
                    padding: '11px 28px',
                    background: 'var(--ppt-color-primary, #2563eb)',
                    color: '#fff',
                    border: 'none',
                    borderRadius: 8,
                    fontWeight: 700,
                    cursor: 'pointer',
                    fontSize: '0.9375rem',
                  }}
                >
                  {t('next')}
                </button>
              ) : (
                <button
                  type="button"
                  disabled={!form.termsAccepted}
                  onClick={() => setSubmitted(true)}
                  style={{
                    padding: '11px 28px',
                    background: form.termsAccepted
                      ? 'var(--ppt-color-success, #10b981)'
                      : 'var(--ppt-border-default, #e5e7eb)',
                    color: form.termsAccepted ? '#fff' : 'var(--ppt-fg-muted, #9ca3af)',
                    border: 'none',
                    borderRadius: 8,
                    fontWeight: 700,
                    cursor: form.termsAccepted ? 'pointer' : 'not-allowed',
                    fontSize: '0.9375rem',
                  }}
                >
                  {t('publish')}
                </button>
              )}
            </div>
          </div>

          {/* Progress aside */}
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
                fontSize: '1rem',
                fontWeight: 700,
                color: 'var(--ppt-fg-primary)',
                margin: '0 0 16px',
              }}
            >
              {t('asideTitle')}
            </h2>
            <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
              {SELL_STEPS.map((s) => (
                <div
                  key={s.id}
                  style={{
                    display: 'flex',
                    gap: 12,
                    alignItems: 'center',
                    opacity: s.id > step ? 0.5 : 1,
                  }}
                >
                  <div
                    style={{
                      width: 28,
                      height: 28,
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
                      fontSize: '0.75rem',
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
                        fontSize: '0.875rem',
                      }}
                    >
                      {t(`steps.${s.key}.title`)}
                    </div>
                    <div style={{ fontSize: '0.75rem', color: 'var(--ppt-fg-muted, #9ca3af)' }}>
                      {t(`steps.${s.key}.description`)}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </aside>
        </div>
      </main>

      <Footer />

      <style jsx>{`
        /* Sell-wizard 2-col layout: form + Steps sidebar. Stacks to 1 col
           below 1024 px so the 260 px aside doesn't push past the viewport
           (was +225 px overflow on 375 px before this rule). */
        .sell-grid {
          max-width: 800px;
          margin: 0 auto;
          display: grid;
          grid-template-columns: 1fr;
          gap: 40px;
          align-items: start;
        }
        @media (min-width: 1024px) {
          .sell-grid {
            grid-template-columns: 1fr 260px;
          }
        }
      `}</style>
    </div>
  );
}
