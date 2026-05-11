'use client';

/**
 * Sell / publish listing wizard — Reality Portal.
 * Screen-map: docs/screens/reality/sell.md
 *
 * 5-step wizard: Type+location → Details → Photos → Price → Contact+summary
 */

import { useState } from 'react';
import { Footer, Header } from '@/components/ui';
import { INITIAL_FORM_DATA, PROPERTY_TYPE_OPTIONS, SELL_STEPS, type SellFormData } from './_mock';

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

export default function SellPage() {
  const [step, setStep] = useState(1);
  const [form, setForm] = useState<SellFormData>(INITIAL_FORM_DATA);
  const [submitted, setSubmitted] = useState(false);

  const update = (patch: Partial<SellFormData>) => setForm((f) => ({ ...f, ...patch }));

  const currentStepInfo = SELL_STEPS[step - 1];

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
              Váš inzerát bol odoslaný!
            </h1>
            <p style={{ color: 'var(--ppt-fg-secondary)', marginBottom: 28 }}>
              Po overení bude zverejnený do 2 hodín.
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
              Pridať ďalší inzerát
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
              Pridať inzerát
            </h1>
            <p
              style={{ color: 'var(--ppt-fg-secondary)', marginBottom: 28, fontSize: '0.9375rem' }}
            >
              Krok {step} z {SELL_STEPS.length}: {currentStepInfo?.title}
            </p>

            {/* TODO: replace with @ppt/ui-kit/Stepper once available */}
            <StepIndicator current={step} />

            {/* Step 1: Type + Location */}
            {step === 1 && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
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
                          padding: '12px',
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
                          fontSize: '0.9375rem',
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
                  {/* TODO: replace with @ppt/ui-kit/RadioCards once available */}
                  <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: 10 }}>
                    {PROPERTY_TYPE_OPTIONS.map((opt) => (
                      <button
                        key={opt.value}
                        type="button"
                        onClick={() => update({ propertyType: opt.value })}
                        style={{
                          padding: '12px',
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
                    Adresa
                  </label>
                  <input
                    type="text"
                    placeholder="Ulica a číslo"
                    value={form.address}
                    onChange={(e) => update({ address: e.target.value })}
                    style={inputStyle}
                  />
                </div>

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
                    Mesto / Obec
                  </label>
                  <input
                    type="text"
                    placeholder="Napr. Bratislava"
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
                {[
                  { key: 'area', label: 'Plocha (m²)', placeholder: 'Napr. 65' },
                  { key: 'rooms', label: 'Počet izieb', placeholder: 'Napr. 3' },
                  { key: 'floor', label: 'Poschodie', placeholder: 'Napr. 2' },
                  { key: 'totalFloors', label: 'Celkový počet poschodí', placeholder: 'Napr. 7' },
                  { key: 'yearBuilt', label: 'Rok výstavby', placeholder: 'Napr. 1995' },
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
                      value={form[field.key as keyof SellFormData] as string | number}
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
                    Popis nehnuteľnosti
                  </label>
                  <textarea
                    rows={5}
                    placeholder="Opíšte nehnuteľnosť, jej vlastnosti, vybavenie..."
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
                  Nahrajte fotografie (odporúčané min. 5, max. 30).
                </p>
                {/* TODO: replace with @ppt/ui-kit/FileUpload once available */}
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
                  <span style={{ fontWeight: 600 }}>Kliknite alebo pretiahnite fotografie sem</span>
                  <span style={{ fontSize: '0.875rem' }}>JPG, PNG, WEBP · max. 10 MB na fotku</span>
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
                    ✓ {form.photos.length} {form.photos.length === 1 ? 'fotografia' : 'fotografie'}{' '}
                    vybraté
                  </p>
                )}
              </div>
            )}

            {/* Step 4: Price */}
            {step === 4 && (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 20 }}>
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
                    Požadovaná cena (€)
                  </label>
                  <input
                    type="number"
                    placeholder={
                      form.transactionType === 'rent' ? 'Mesačný nájom napr. 800' : 'Napr. 250000'
                    }
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
                    Cena je dohodou
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
                    <strong>Typ:</strong> {form.transactionType === 'sale' ? 'Predaj' : 'Prenájom'}{' '}
                    · {PROPERTY_TYPE_OPTIONS.find((o) => o.value === form.propertyType)?.label}
                  </div>
                  <div>
                    <strong>Poloha:</strong> {form.address || '–'}, {form.city || '–'}
                  </div>
                  <div>
                    <strong>Plocha:</strong> {form.area || '–'} m² · <strong>Izby:</strong>{' '}
                    {form.rooms || '–'}
                  </div>
                  <div>
                    <strong>Cena:</strong>{' '}
                    {form.price ? `${Number(form.price).toLocaleString('sk-SK')} €` : '–'}{' '}
                    {form.priceNegotiable ? '(dohodou)' : ''}
                  </div>
                  <div>
                    <strong>Fotografie:</strong> {form.photos.length}
                  </div>
                </div>

                {[
                  {
                    key: 'contactName',
                    label: 'Meno',
                    type: 'text',
                    placeholder: 'Vaše meno a priezvisko',
                  },
                  {
                    key: 'contactPhone',
                    label: 'Telefón',
                    type: 'tel',
                    placeholder: '+421 9XX XXX XXX',
                  },
                  {
                    key: 'contactEmail',
                    label: 'E-mail',
                    type: 'email',
                    placeholder: 'vas@email.sk',
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
                    Súhlasím s{' '}
                    <a href="/terms" style={{ color: 'var(--ppt-color-primary, #2563eb)' }}>
                      podmienkami používania
                    </a>{' '}
                    a{' '}
                    <a href="/privacy" style={{ color: 'var(--ppt-color-primary, #2563eb)' }}>
                      ochranou osobných údajov
                    </a>
                    .
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
                  ← Späť
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
                  Ďalej →
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
                  Zverejniť inzerát
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
              Postup
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
