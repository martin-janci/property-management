'use client';

/**
 * Report a listing — problem report form.
 * Screen-map: docs/screens/reality/report-listing.md
 */

import { SubmitReportError, useSubmitReport } from '@ppt/reality-api-client';
import { useSearchParams } from 'next/navigation';
import { useTranslations } from 'next-intl';
import { useState } from 'react';
import { Footer, Header } from '@/components/ui';
import { REPORT_PROBLEMS, type ReportProblem } from './_mock';

// TODO: replace problem cards with @ppt/ui-kit/RadioCards once available
// TODO(#3017): the attachment control is disabled pending an upload pipeline
// — see the "Attachments" block below for why it collects nothing today.

// reality-server's `POST /api/v1/reports` requires a UUID `listing_id` that
// resolves to an active listing. A deep link from a listing ("report this
// listing") passes `?listing=<uuid>`; otherwise we fish a UUID out of the
// free-text reference the reporter pasted.
const UUID_RE = /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i;

// Status copy. This page ships its user-facing text hardcoded in Slovak
// (only the title/subtitle go through next-intl today), so the submit
// states follow the same pattern until the page is fully i18n-extracted.
const MSG = {
  submitting: 'Odosiela sa…',
  referenceLabel: 'Referenčné číslo',
  referenceHint: 'Uveďte ho pri ďalšej komunikácii — pomôže nám vaše hlásenie rýchlo dohľadať.',
  errorNoListing:
    'Nevedeli sme určiť, ktorého inzerátu sa hlásenie týka. Otvorte nahlásenie z detailu inzerátu alebo vložte odkaz či ID inzerátu vyššie.',
  errorRateLimit: 'Priveľa nahlásení z tohto zariadenia. Skúste to o chvíľu znova.',
  errorNotFound: 'Inzerát sa nenašiel. Skontrolujte odkaz alebo ID a skúste to znova.',
  errorGeneric: 'Nahlásenie sa nepodarilo odoslať. Skúste to prosím znova.',
} as const;

export default function ReportPage() {
  const t = useTranslations('pages.report');
  const searchParams = useSearchParams();
  const submitReport = useSubmitReport();
  const [problem, setProblem] = useState<ReportProblem | null>(null);
  const [listingRef, setListingRef] = useState('');
  const [description, setDescription] = useState('');
  const [gdprAccepted, setGdprAccepted] = useState(false);
  const [submitted, setSubmitted] = useState(false);
  const [reportId, setReportId] = useState<string | null>(null);
  const [formError, setFormError] = useState<string | null>(null);

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

  // Resolve the target listing UUID: a `?listing=`/`?listingId=` deep-link
  // param wins, otherwise accept a UUID embedded in the free-text reference
  // (bare id or a listing URL). Returns null when none is present.
  const resolveListingId = (): string | null => {
    const fromQuery = searchParams.get('listing') ?? searchParams.get('listingId');
    const queryMatch = fromQuery?.match(UUID_RE);
    if (queryMatch) return queryMatch[0];
    const refMatch = listingRef.match(UUID_RE);
    if (refMatch) return refMatch[0];
    return null;
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setFormError(null);
    if (!problem || !description.trim() || !gdprAccepted) return;

    const listingId = resolveListingId();
    if (!listingId) {
      setFormError(MSG.errorNoListing);
      return;
    }

    // Preserve whatever the reporter typed in the reference field inside the
    // description when it isn't the resolved UUID — the endpoint has no
    // separate free-text reference field, so moderators would otherwise lose
    // the original URL/ID.
    const trimmedRef = listingRef.trim();
    const fullDescription =
      trimmedRef && trimmedRef !== listingId
        ? `${description.trim()}\n\n[ref: ${trimmedRef}]`
        : description.trim();

    try {
      // Read the created report id back from the 201 `{ report }` envelope so
      // the success card can surface a reference the reporter can quote in a
      // follow-up. The server returns a raw UUID today (no "R-…" scheme is
      // defined server-side yet), so we surface the id verbatim.
      const result = await submitReport.mutateAsync({
        listing_id: listingId,
        problem_type: problem,
        description: fullDescription,
      });
      setReportId(result.report?.id ?? null);
      setSubmitted(true);
    } catch (err) {
      const status = err instanceof SubmitReportError ? err.status : 0;
      if (status === 429) setFormError(MSG.errorRateLimit);
      else if (status === 404) setFormError(MSG.errorNotFound);
      else setFormError(MSG.errorGeneric);
    }
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
            {reportId && (
              <div
                style={{
                  display: 'inline-block',
                  maxWidth: 480,
                  margin: '0 auto 28px',
                  padding: '14px 18px',
                  borderRadius: 10,
                  background: 'var(--ppt-bg-surface)',
                  border: '1px solid var(--ppt-border-default, #e5e7eb)',
                }}
              >
                <div
                  style={{
                    fontSize: '0.8125rem',
                    color: 'var(--ppt-fg-secondary)',
                    marginBottom: 4,
                  }}
                >
                  {MSG.referenceLabel}
                </div>
                <code
                  data-testid="report-reference-id"
                  style={{
                    fontSize: '1rem',
                    fontWeight: 700,
                    color: 'var(--ppt-fg-primary)',
                    wordBreak: 'break-all',
                  }}
                >
                  {reportId}
                </code>
                <p
                  style={{
                    fontSize: '0.8125rem',
                    color: 'var(--ppt-fg-muted, #9ca3af)',
                    margin: '8px 0 0',
                    lineHeight: 1.5,
                  }}
                >
                  {MSG.referenceHint}
                </p>
              </div>
            )}
            <div>
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

          {/* Attachments — intentionally NOT wired to submit (see #3017).
              reality-server's POST /api/v1/reports accepts `attachments` as
              pre-uploaded http(s) URLs, but no upload endpoint exists yet, so
              the old control collected File objects that handleSubmit never
              transmitted — a reporter saw a success screen while the evidence
              was silently dropped. Until the upload pipeline lands, the
              control is disabled with a "coming soon" hint and carries no file
              input, so the page no longer implies evidence is submitted.
              TODO(#3017): restore an active upload control wired to a presigned
              upload, then pass the returned URLs as `attachments`. */}
          <div>
            <label
              style={{
                display: 'block',
                fontWeight: 600,
                color: 'var(--ppt-fg-primary)',
                marginBottom: 6,
              }}
            >
              Prílohy (čoskoro)
            </label>
            {/* TODO(#3017): replace with @ppt/ui-kit/FileUpload wired to a
                presigned upload once available. */}
            <div
              aria-disabled="true"
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 10,
                padding: '14px 18px',
                border: '2px dashed var(--ppt-border-default, #e5e7eb)',
                borderRadius: 8,
                cursor: 'not-allowed',
                color: 'var(--ppt-fg-muted, #9ca3af)',
                fontSize: '0.9375rem',
                opacity: 0.6,
                userSelect: 'none',
              }}
            >
              <span style={{ fontSize: '1.25rem' }}>📎</span>
              Pridávanie príloh bude čoskoro dostupné
            </div>
            <p
              style={{ fontSize: '0.8125rem', color: 'var(--ppt-fg-muted, #9ca3af)', marginTop: 6 }}
            >
              Nahrávanie screenshotov zatiaľ nie je dostupné — prosím, opíšte dôkazy v popise
              vyššie. Pripravujeme ho.
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

          {/* Error */}
          {formError && (
            <div
              role="alert"
              style={{
                padding: '12px 14px',
                borderRadius: 8,
                background: 'var(--ppt-color-danger-light, #fef2f2)',
                border: '1px solid var(--ppt-color-danger, #ef4444)',
                color: 'var(--ppt-color-danger-hover, #b91c1c)',
                fontSize: '0.9375rem',
              }}
            >
              {formError}
            </div>
          )}

          {/* Submit */}
          {(() => {
            const canSubmit = Boolean(problem) && description.trim() !== '' && gdprAccepted;
            const enabled = canSubmit && !submitReport.isPending;
            return (
              <button
                type="submit"
                disabled={!enabled}
                style={{
                  alignSelf: 'flex-start',
                  padding: '13px 32px',
                  background: enabled
                    ? 'var(--ppt-color-danger, #ef4444)'
                    : 'var(--ppt-border-default, #e5e7eb)',
                  color: enabled ? '#fff' : 'var(--ppt-fg-muted, #9ca3af)',
                  border: 'none',
                  borderRadius: 8,
                  fontWeight: 700,
                  cursor: enabled ? 'pointer' : 'not-allowed',
                  fontSize: '1rem',
                }}
              >
                {submitReport.isPending ? MSG.submitting : 'Odoslať nahlásenie'}
              </button>
            );
          })()}
        </form>
      </main>

      <Footer />
    </div>
  );
}
