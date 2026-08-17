/**
 * Shared agency dashboard error / empty states.
 *
 * Extracted from AgencyDashboard (PR #2280) so every agency route —
 * dashboard, listings, realtors, branding — renders the same
 * error/retry and "no agency" surfaces instead of each re-implementing
 * (or omitting) them (Issue #2343, findings 1–2).
 */

'use client';

import Link from 'next/link';
import { useTranslations } from 'next-intl';

/**
 * Read the HTTP status off a thrown fetch error without importing the
 * client's `ApiError` class here (the agency components mock
 * `@ppt/reality-api-client` wholesale in their unit tests, so importing a
 * runtime symbol from it would be `undefined` under test). The client
 * attaches `status` to every non-2xx error, so a plain structural read is
 * enough to tell a genuine 404 ("no agency") from a 5xx/network failure.
 */
export function isNotFoundError(error: unknown): boolean {
  return (error as { status?: number } | null | undefined)?.status === 404;
}

/**
 * Full-page load-error state with a Retry affordance. Shown when the
 * agency query fails with a transport/server error (i.e. NOT a 404).
 */
export function AgencyLoadError({ onRetry }: { onRetry: () => void }) {
  const t = useTranslations('agency');
  return (
    <div className="agency-error" role="alert">
      <svg
        width="64"
        height="64"
        viewBox="0 0 24 24"
        fill="none"
        stroke="var(--ppt-color-danger, #ef4444)"
        strokeWidth="1.5"
        aria-hidden="true"
      >
        <circle cx="12" cy="12" r="10" />
        <line x1="12" y1="8" x2="12" y2="12" />
        <line x1="12" y1="16" x2="12.01" y2="16" />
      </svg>
      <h2>{t('loadErrorTitle')}</h2>
      <p>{t('loadErrorMessage')}</p>
      <button type="button" className="retry-button" onClick={onRetry}>
        {t('retry')}
      </button>
      <style jsx>{`
        .agency-error {
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
          padding: 64px 24px;
          text-align: center;
          min-height: 50vh;
        }
        h2 {
          font-size: 1.5rem;
          color: var(--ppt-fg-primary);
          margin: 24px 0 8px;
        }
        p {
          color: var(--ppt-fg-muted);
          margin: 0 0 24px;
        }
        .retry-button {
          padding: 12px 24px;
          background: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent);
          border: none;
          border-radius: 8px;
          font-weight: 500;
          cursor: pointer;
        }
        .retry-button:hover {
          background: var(--ppt-color-primary-hover);
        }
      `}</style>
    </div>
  );
}

/** Inline per-section error used inside an otherwise-loaded dashboard. */
export function SectionError({ message }: { message: string }) {
  return (
    <div className="section-error" role="alert">
      <span>{message}</span>
      <style jsx>{`
        .section-error {
          padding: 24px;
          text-align: center;
          color: var(--ppt-fg-muted);
          font-size: 14px;
        }
      `}</style>
    </div>
  );
}

/**
 * "No agency" onboarding state. Reached when the agency query genuinely
 * resolves to no agency (a 404 or a null body) — the point being that this
 * must NOT be shown on a transport/server error.
 */
export function NoAgencyMessage() {
  const t = useTranslations('agency');
  return (
    <div className="no-agency">
      <svg
        width="64"
        height="64"
        viewBox="0 0 24 24"
        fill="none"
        stroke="var(--ppt-fg-subtle)"
        strokeWidth="1.5"
        aria-hidden="true"
      >
        <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
        <polyline points="9 22 9 12 15 12 15 22" />
      </svg>
      <h2>{t('noAgencyTitle')}</h2>
      <p>{t('noAgencyMessage')}</p>
      <Link href="/agency/create" className="create-button">
        {t('createAgency')}
      </Link>
      <style jsx>{`
        .no-agency {
          display: flex;
          flex-direction: column;
          align-items: center;
          justify-content: center;
          padding: 64px 24px;
          text-align: center;
          min-height: 50vh;
        }
        h2 {
          font-size: 1.5rem;
          color: var(--ppt-fg-primary);
          margin: 24px 0 8px;
        }
        p {
          color: var(--ppt-fg-muted);
          margin: 0 0 24px;
        }
        .create-button {
          padding: 12px 24px;
          background: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent);
          border-radius: 8px;
          text-decoration: none;
          font-weight: 500;
        }
        .create-button:hover {
          background: var(--ppt-color-primary-hover);
        }
      `}</style>
    </div>
  );
}
