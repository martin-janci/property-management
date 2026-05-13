'use client';

/**
 * Report a listing — problem report form.
 * Screen-map: docs/screens/reality/report-listing.md
 */

import { useTranslations } from 'next-intl';
import { useState } from 'react';
import { Footer, Header } from '@/components/ui';
import { REPORT_PROBLEMS, type ReportProblem } from './_mock';

// TODO: replace problem cards with @ppt/ui-kit/RadioCards once available
// TODO: replace file attachment with @ppt/ui-kit/FileUpload once available

export default function ReportPage() {
  const t = useTranslations('pages.report');
  const [problem, setProblem] = useState<ReportProblem | null>(null);
  const [listingRef, setListingRef] = useState('');
  const [description, setDescription] = useState('');
  const [attachments, setAttachments] = useState<File[]>([]);
  const [gdprAccepted, setGdprAccepted] = useState(false);
  const [submitted, setSubmitted] = useState(false);

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

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!problem || !description.trim() || !gdprAccepted) return;
    setSubmitted(true);
  };

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
            <div style={{ fontSize: '3.5rem', marginBottom: 16 }}>✅</div>
            <h1
              style={{
                fontSize: '1.75rem',
                fontWeight: 800,
                color: 'var(--ppt-fg-primary)',
                marginBottom: 12,
              }}
            >
              Nahlásenie bolo odoslané
            </h1>
            <p
              style={{
                color: 'var(--ppt-fg-secondary)',
                maxWidth: 480,
                margin: '0 auto 28px',
                lineHeight: 1.7,
              }}
            >
              Váš podnet sme prijali. Náš tím ho preverí do 24 hodín. Ďakujeme, že pomáhate
              udržiavať portál bezpečným.
            </p>
            <a
              href="/listings"
              style={{
                display: 'inline-block',
                padding: '12px 28px',
                background: 'var(--ppt-color-primary, #2563eb)',
                color: '#fff',
                borderRadius: 8,
                fontWeight: 700,
                textDecoration: 'none',
              }}
            >
              Späť na ponuky
            </a>
          </div>
        </main>
        <Footer />
      </div>
    );
  }

  return (
    <div
      data-i18n="pages.report.root"
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
          maxWidth: 720,
          margin: '0 auto',
          padding: '48px 24px',
          width: '100%',
          boxSizing: 'border-box',
        }}
      >
        <h1
          style={{
            fontSize: '1.75rem',
            fontWeight: 800,
            color: 'var(--ppt-fg-primary)',
            marginBottom: 8,
          }}
        >
          {t('h1')}
        </h1>
        <p style={{ color: 'var(--ppt-fg-secondary)', marginBottom: 36, lineHeight: 1.6 }}>
          {t('subtitle')}
        </p>

        <form
          onSubmit={handleSubmit}
          noValidate
          style={{ display: 'flex', flexDirection: 'column', gap: 28 }}
        >
          {/* Problem radio cards */}
          <fieldset style={{ border: 'none', padding: 0, margin: 0 }}>
            <legend
              style={{
                fontWeight: 700,
                color: 'var(--ppt-fg-primary)',
                fontSize: '1rem',
                marginBottom: 14,
              }}
            >
              Dôvod nahlásenia <span style={{ color: 'var(--ppt-color-danger, #ef4444)' }}>*</span>
            </legend>
            {/* TODO: replace with @ppt/ui-kit/RadioCards once available */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
              {REPORT_PROBLEMS.map((opt) => (
                <label
                  key={opt.value}
                  style={{
                    display: 'flex',
                    gap: 14,
                    padding: '14px 16px',
                    borderRadius: 10,
                    border: `2px solid ${problem === opt.value ? 'var(--ppt-color-primary, #2563eb)' : 'var(--ppt-border-default, #e5e7eb)'}`,
                    background:
                      problem === opt.value
                        ? 'var(--ppt-color-primary-light, #dbeafe)'
                        : 'var(--ppt-bg-surface)',
                    cursor: 'pointer',
                    transition: 'border-color .15s',
                  }}
                >
                  <input
                    type="radio"
                    name="problem"
                    value={opt.value}
                    checked={problem === opt.value}
                    onChange={() => setProblem(opt.value)}
                    style={{ marginTop: 3, accentColor: 'var(--ppt-color-primary, #2563eb)' }}
                  />
                  <div>
                    <div
                      style={{ fontWeight: 600, color: 'var(--ppt-fg-primary)', marginBottom: 3 }}
                    >
                      {opt.label}
                    </div>
                    <div style={{ fontSize: '0.875rem', color: 'var(--ppt-fg-secondary)' }}>
                      {opt.description}
                    </div>
                  </div>
                </label>
              ))}
            </div>
          </fieldset>

          {/* Listing reference */}
          <div>
            <label
              style={{
                display: 'block',
                fontWeight: 600,
                color: 'var(--ppt-fg-primary)',
                marginBottom: 6,
              }}
            >
              Odkaz na inzerát (URL alebo ID)
            </label>
            <input
              type="text"
              placeholder="napr. https://rlt.sk/listings/ba-byt-3i alebo #12345"
              value={listingRef}
              onChange={(e) => setListingRef(e.target.value)}
              style={inputStyle}
            />
          </div>

          {/* Description */}
          <div>
            <label
              style={{
                display: 'block',
                fontWeight: 600,
                color: 'var(--ppt-fg-primary)',
                marginBottom: 6,
              }}
            >
              Popis problému <span style={{ color: 'var(--ppt-color-danger, #ef4444)' }}>*</span>
            </label>
            <textarea
              rows={5}
              required
              placeholder="Opíšte, prečo nahlasujete tento inzerát. Čím viac detailov uvediete, tým rýchlejšie to preveríme."
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              style={{ ...inputStyle, resize: 'vertical' }}
            />
          </div>

          {/* Attachments */}
          <div>
            <label
              style={{
                display: 'block',
                fontWeight: 600,
                color: 'var(--ppt-fg-primary)',
                marginBottom: 6,
              }}
            >
              Prílohy (voliteľné)
            </label>
            {/* TODO: replace with @ppt/ui-kit/FileUpload once available */}
            <label
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 10,
                padding: '14px 18px',
                border: '2px dashed var(--ppt-border-default, #e5e7eb)',
                borderRadius: 8,
                cursor: 'pointer',
                color: 'var(--ppt-fg-secondary)',
                fontSize: '0.9375rem',
              }}
            >
              <span style={{ fontSize: '1.25rem' }}>📎</span>
              {attachments.length > 0
                ? `${attachments.length} ${attachments.length === 1 ? 'súbor' : 'súbory'} vybrané`
                : 'Priložiť screenshoty alebo dôkazy'}
              <input
                type="file"
                multiple
                accept="image/*,.pdf"
                style={{ display: 'none' }}
                onChange={(e) => {
                  if (e.target.files) setAttachments(Array.from(e.target.files));
                }}
              />
            </label>
            <p
              style={{ fontSize: '0.8125rem', color: 'var(--ppt-fg-muted, #9ca3af)', marginTop: 6 }}
            >
              PNG, JPG, PDF · max. 5 MB na súbor · max. 5 súborov
            </p>
          </div>

          {/* GDPR */}
          <label style={{ display: 'flex', alignItems: 'flex-start', gap: 10, cursor: 'pointer' }}>
            <input
              type="checkbox"
              required
              checked={gdprAccepted}
              onChange={(e) => setGdprAccepted(e.target.checked)}
              style={{
                width: 18,
                height: 18,
                marginTop: 2,
                accentColor: 'var(--ppt-color-primary, #2563eb)',
              }}
            />
            <span
              style={{ fontSize: '0.875rem', color: 'var(--ppt-fg-secondary)', lineHeight: 1.5 }}
            >
              Súhlasím so spracovaním osobných údajov za účelom preverenia môjho nahlásenia v súlade
              s{' '}
              <a href="/privacy" style={{ color: 'var(--ppt-color-primary, #2563eb)' }}>
                Zásadami ochrany súkromia
              </a>
              . <span style={{ color: 'var(--ppt-color-danger, #ef4444)' }}>*</span>
            </span>
          </label>

          {/* Submit */}
          <button
            type="submit"
            disabled={!problem || !description.trim() || !gdprAccepted}
            style={{
              alignSelf: 'flex-start',
              padding: '13px 32px',
              background:
                problem && description.trim() && gdprAccepted
                  ? 'var(--ppt-color-danger, #ef4444)'
                  : 'var(--ppt-border-default, #e5e7eb)',
              color:
                problem && description.trim() && gdprAccepted
                  ? '#fff'
                  : 'var(--ppt-fg-muted, #9ca3af)',
              border: 'none',
              borderRadius: 8,
              fontWeight: 700,
              cursor: problem && description.trim() && gdprAccepted ? 'pointer' : 'not-allowed',
              fontSize: '1rem',
            }}
          >
            Odoslať nahlásenie
          </button>
        </form>
      </main>

      <Footer />
    </div>
  );
}
