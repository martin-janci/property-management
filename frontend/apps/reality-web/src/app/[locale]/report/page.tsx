'use client';

/**
 * Report a listing — problem report form.
 * Screen-map: docs/screens/reality/report-listing.md
 */

import {
  SubmitReportError,
  type SubmitReportRequest,
  useSubmitReport,
} from '@ppt/reality-api-client';
import { useSearchParams } from 'next/navigation';
import { useTranslations } from 'next-intl';
import { useState } from 'react';
import { Footer, Header } from '@/components/ui';
import { Link } from '@/i18n/routing';
import { REPORT_PROBLEMS, type ReportProblem } from './_mock';

// TODO: replace problem cards with @ppt/ui-kit/RadioCards once available

// reality-server's `POST /api/v1/reports` requires a UUID `listing_id` that
// resolves to an active listing. A deep link from a listing ("report this
// listing") passes `?listing=<uuid>`; otherwise we fish a UUID out of the
// free-text reference the reporter pasted.
const UUID_RE = /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i;

// Attachment bounds mirror reality-server's `submit_report` validation
// (`MAX_REPORT_ATTACHMENTS` + `validate_image_or_link_url`): at most 10
// http(s) URLs, each <= 2048 chars. We enforce the same client-side so a
// reporter sees an inline hint instead of a 400 after submit. No upload
// endpoint exists on the public portal (#3017), so attachments are evidence
// *links* the reporter already hosts elsewhere — exactly what the endpoint's
// `attachments: string[]` contract accepts.
const MAX_ATTACHMENTS = 10;
const MAX_ATTACHMENT_URL_LEN = 2048;

// Client mirror of reality-server's `validate_image_or_link_url`: http(s)
// scheme only (blocks javascript:/data:/file: stored-XSS vectors) and a
// length cap. The server re-validates — this is a UX shortcut, not the
// security boundary.
function isValidAttachmentUrl(value: string): boolean {
  if (value.length > MAX_ATTACHMENT_URL_LEN) return false;
  let parsed: URL;
  try {
    parsed = new URL(value);
  } catch {
    return false;
  }
  return parsed.protocol === 'http:' || parsed.protocol === 'https:';
}

export default function ReportPage() {
  const t = useTranslations('pages.report');
  const searchParams = useSearchParams();
  const submitReport = useSubmitReport();
  const [problem, setProblem] = useState<ReportProblem | null>(null);
  const [listingRef, setListingRef] = useState('');
  const [description, setDescription] = useState('');
  const [attachmentUrls, setAttachmentUrls] = useState<string[]>([]);
  const [attachmentInput, setAttachmentInput] = useState('');
  const [attachmentError, setAttachmentError] = useState<string | null>(null);
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

  // Add the typed evidence link to the list after mirroring the server's
  // http(s)/length checks. Silently de-dupes; surfaces an inline hint when the
  // cap is hit or the URL is malformed so the reporter fixes it before submit.
  const handleAddAttachment = () => {
    setAttachmentError(null);
    const value = attachmentInput.trim();
    if (!value) return;
    if (attachmentUrls.length >= MAX_ATTACHMENTS) {
      setAttachmentError(t('attachmentsMax', { max: MAX_ATTACHMENTS }));
      return;
    }
    if (!isValidAttachmentUrl(value)) {
      setAttachmentError(t('attachmentsInvalidUrl'));
      return;
    }
    if (attachmentUrls.includes(value)) {
      setAttachmentInput('');
      return;
    }
    setAttachmentUrls([...attachmentUrls, value]);
    setAttachmentInput('');
  };

  const handleRemoveAttachment = (url: string) => {
    setAttachmentUrls(attachmentUrls.filter((u) => u !== url));
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    // Re-entrancy guard: a second submit event (Enter key / fast double click)
    // while the POST is in flight would file a duplicate abuse report.
    if (submitReport.isPending) return;
    setFormError(null);
    if (!problem || !description.trim() || !gdprAccepted) return;

    const listingId = resolveListingId();
    if (!listingId) {
      setFormError(t('errorNoListing'));
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
      const payload: SubmitReportRequest = {
        listing_id: listingId,
        problem_type: problem,
        description: fullDescription,
      };
      // Only attach the key when the reporter actually added evidence links —
      // omitting it keeps the body minimal and matches the endpoint's optional
      // `attachments` contract.
      if (attachmentUrls.length > 0) {
        payload.attachments = attachmentUrls;
      }
      const result = await submitReport.mutateAsync(payload);
      setReportId(result.report?.id ?? null);
      setSubmitted(true);
    } catch (err) {
      const status = err instanceof SubmitReportError ? err.status : 0;
      if (status === 429) setFormError(t('errorRateLimit'));
      else if (status === 404) setFormError(t('errorNotFound'));
      else setFormError(t('errorGeneric'));
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
              {t('successTitle')}
            </h1>
            <p
              style={{
                color: 'var(--ppt-fg-secondary)',
                maxWidth: 480,
                margin: '0 auto 28px',
                lineHeight: 1.7,
              }}
            >
              {t('successBody')}
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
                  {t('referenceLabel')}
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
                  {t('referenceHint')}
                </p>
              </div>
            )}
            <div>
              <Link
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
                {t('backToListings')}
              </Link>
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
              {t('problemLegend')}{' '}
              <span style={{ color: 'var(--ppt-color-danger, #ef4444)' }}>*</span>
            </legend>
            {/* TODO: replace with @ppt/ui-kit/RadioCards once available */}
            <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
              {REPORT_PROBLEMS.map((value) => (
                <label
                  key={value}
                  style={{
                    display: 'flex',
                    gap: 14,
                    padding: '14px 16px',
                    borderRadius: 10,
                    border: `2px solid ${problem === value ? 'var(--ppt-color-primary, #2563eb)' : 'var(--ppt-border-default, #e5e7eb)'}`,
                    background:
                      problem === value
                        ? 'var(--ppt-color-primary-light, #dbeafe)'
                        : 'var(--ppt-bg-surface)',
                    cursor: 'pointer',
                    transition: 'border-color .15s',
                  }}
                >
                  <input
                    type="radio"
                    name="problem"
                    value={value}
                    checked={problem === value}
                    onChange={() => setProblem(value)}
                    style={{ marginTop: 3, accentColor: 'var(--ppt-color-primary, #2563eb)' }}
                  />
                  <div>
                    <div
                      style={{ fontWeight: 600, color: 'var(--ppt-fg-primary)', marginBottom: 3 }}
                    >
                      {t(`problems.${value}.label`)}
                    </div>
                    <div style={{ fontSize: '0.875rem', color: 'var(--ppt-fg-secondary)' }}>
                      {t(`problems.${value}.description`)}
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
              {t('listingRefLabel')}
            </label>
            <input
              type="text"
              placeholder={t('listingRefPlaceholder')}
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
              {t('descriptionLabel')}{' '}
              <span style={{ color: 'var(--ppt-color-danger, #ef4444)' }}>*</span>
            </label>
            <textarea
              rows={5}
              required
              placeholder={t('descriptionPlaceholder')}
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              style={{ ...inputStyle, resize: 'vertical' }}
            />
          </div>

          {/* Attachments — evidence links (#3017). The public portal has no
              file-upload endpoint, but reality-server's POST /api/v1/reports
              accepts up to 10 pre-hosted http(s) URLs as `attachments`, so the
              reporter pastes links to evidence they already host (a screenshot
              service, cloud drive, the offending external page). The links are
              collected into `attachmentUrls` and sent in the submit body.
              TODO: swap the URL input for @ppt/ui-kit/FileUpload wired to a
              presigned upload once the portal gains one. */}
          <div>
            <label
              htmlFor="attachment-url"
              style={{
                display: 'block',
                fontWeight: 600,
                color: 'var(--ppt-fg-primary)',
                marginBottom: 6,
              }}
            >
              {t('attachmentsLabel')}
            </label>
            <div style={{ display: 'flex', gap: 10 }}>
              <input
                id="attachment-url"
                type="url"
                inputMode="url"
                placeholder={t('attachmentsUrlPlaceholder')}
                value={attachmentInput}
                onChange={(e) => setAttachmentInput(e.target.value)}
                onKeyDown={(e) => {
                  // Enter adds the link instead of submitting the whole form.
                  if (e.key === 'Enter') {
                    e.preventDefault();
                    handleAddAttachment();
                  }
                }}
                style={{ ...inputStyle, flex: 1 }}
              />
              <button
                type="button"
                onClick={handleAddAttachment}
                disabled={attachmentUrls.length >= MAX_ATTACHMENTS}
                style={{
                  padding: '0 20px',
                  background: 'var(--ppt-bg-surface)',
                  border: '1px solid var(--ppt-border-default, #e5e7eb)',
                  borderRadius: 8,
                  fontWeight: 600,
                  color: 'var(--ppt-fg-primary)',
                  cursor: attachmentUrls.length >= MAX_ATTACHMENTS ? 'not-allowed' : 'pointer',
                  whiteSpace: 'nowrap',
                }}
              >
                {t('attachmentsAdd')}
              </button>
            </div>
            {attachmentError && (
              <p
                role="alert"
                style={{
                  fontSize: '0.8125rem',
                  color: 'var(--ppt-color-danger, #ef4444)',
                  marginTop: 6,
                }}
              >
                {attachmentError}
              </p>
            )}
            {attachmentUrls.length > 0 && (
              <ul
                style={{
                  listStyle: 'none',
                  padding: 0,
                  margin: '12px 0 0',
                  display: 'flex',
                  flexDirection: 'column',
                  gap: 8,
                }}
              >
                {attachmentUrls.map((url) => (
                  <li
                    key={url}
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 10,
                      padding: '8px 12px',
                      borderRadius: 8,
                      background: 'var(--ppt-bg-surface)',
                      border: '1px solid var(--ppt-border-default, #e5e7eb)',
                    }}
                  >
                    <span style={{ fontSize: '1rem' }}>📎</span>
                    <span
                      style={{
                        flex: 1,
                        fontSize: '0.875rem',
                        color: 'var(--ppt-fg-secondary)',
                        wordBreak: 'break-all',
                      }}
                    >
                      {url}
                    </span>
                    <button
                      type="button"
                      aria-label={t('attachmentsRemove')}
                      onClick={() => handleRemoveAttachment(url)}
                      style={{
                        background: 'none',
                        border: 'none',
                        color: 'var(--ppt-color-danger, #ef4444)',
                        cursor: 'pointer',
                        fontSize: '0.875rem',
                        fontWeight: 600,
                        padding: '2px 6px',
                      }}
                    >
                      {t('attachmentsRemove')}
                    </button>
                  </li>
                ))}
              </ul>
            )}
            <p
              style={{ fontSize: '0.8125rem', color: 'var(--ppt-fg-muted, #9ca3af)', marginTop: 6 }}
            >
              {t('attachmentsHint', { max: MAX_ATTACHMENTS })}
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
              {t.rich('gdpr.label', {
                // next-intl's locale-aware Link preserves the active locale
                // prefix so a SK user lands on /sk/privacy, not /privacy.
                privacyLink: (chunks) => (
                  <Link href="/privacy" style={{ color: 'var(--ppt-color-primary, #2563eb)' }}>
                    {chunks}
                  </Link>
                ),
              })}{' '}
              <span style={{ color: 'var(--ppt-color-danger, #ef4444)' }}>*</span>
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
                {submitReport.isPending ? t('submitting') : t('submit')}
              </button>
            );
          })()}
        </form>
      </main>

      <Footer />
    </div>
  );
}
