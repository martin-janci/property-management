'use client';

/**
 * Cookie consent banner — minimal first pass.
 *
 * Surfaces on first visit; choice is persisted to a first-party
 * `ppt-cookie-consent` localStorage entry (one of: `accepted`, `essential`).
 * Doesn't yet gate any actual analytics/marketing tags — that hookup
 * happens in whatever script-loader lands next; this component owns the
 * UI + persistence half of the contract.
 *
 * Why localStorage rather than a cookie:
 *   - Reading the choice on every server render to decide whether to inject
 *     trackers would require a cookie. We don't render trackers yet, so
 *     localStorage is sufficient and avoids the chicken-and-egg of setting
 *     a cookie about cookies.
 *   - When the GA / Meta pixel loaders land, mirror the choice into a
 *     `ppt_consent` cookie (SameSite=Lax, 13-month max) and gate the tags
 *     on it from the middleware / layout.
 *
 * GDPR: shows the banner up-front, gives a clear Reject path with equal
 * prominence to Accept (no dark patterns), and links to the cookie policy
 * for granular management.
 */

import { useTranslations } from 'next-intl';
import { useEffect, useState } from 'react';
import { Link } from '@/i18n/routing';

const STORAGE_KEY = 'ppt-cookie-consent';

export function CookieConsentBanner() {
  const t = useTranslations('cookieBanner');
  const [shown, setShown] = useState(false);

  useEffect(() => {
    if (typeof window === 'undefined') return;
    if (!window.localStorage.getItem(STORAGE_KEY)) setShown(true);
  }, []);

  const decide = (value: 'accepted' | 'essential') => {
    try {
      window.localStorage.setItem(STORAGE_KEY, value);
    } catch {
      // ignore: storage disabled / quota — we just won't remember the choice,
      // banner will reappear on next visit, which is acceptable.
    }
    setShown(false);
  };

  if (!shown) return null;

  return (
    <div
      className="cookie-banner"
      role="region"
      aria-label={t('title')}
      // dismissed on Esc so a keyboard user can get past it without
      // visiting the buttons. Treat Esc as "reject optional" — the
      // conservative default.
      onKeyDown={(e) => {
        if (e.key === 'Escape') decide('essential');
      }}
    >
      <div className="inner">
        <div className="text">
          <strong className="title">{t('title')}</strong>
          <p className="body">{t('body')}</p>
        </div>
        <div className="actions">
          <Link href="/cookies" className="manage">
            {t('manage')}
          </Link>
          <button type="button" className="btn ghost" onClick={() => decide('essential')}>
            {t('reject')}
          </button>
          <button type="button" className="btn primary" onClick={() => decide('accepted')}>
            {t('accept')}
          </button>
        </div>
      </div>

      <style jsx>{`
        .cookie-banner {
          position: fixed;
          left: 0;
          right: 0;
          bottom: 0;
          z-index: 9000;
          background: var(--ppt-bg-surface);
          border-top: 1px solid var(--ppt-border-default, #e5e7eb);
          box-shadow: 0 -4px 16px rgba(0, 0, 0, 0.08);
        }
        .inner {
          max-width: 1200px;
          margin: 0 auto;
          padding: 18px 24px;
          display: flex;
          gap: 24px;
          align-items: center;
          flex-wrap: wrap;
        }
        .text { flex: 1; min-width: 260px; }
        .title { display: block; font-size: 0.9375rem; font-weight: 700; color: var(--ppt-fg-primary); margin-bottom: 4px; }
        .body { font-size: 0.875rem; color: var(--ppt-fg-secondary); margin: 0; line-height: 1.5; }
        .actions { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; }
        .manage { font-size: 0.875rem; color: var(--ppt-color-primary); text-decoration: underline; padding: 4px 0; }
        .btn { padding: 9px 18px; border-radius: 8px; font-weight: 600; font-size: 0.875rem; cursor: pointer; border: 1px solid var(--ppt-border-strong); }
        .btn.ghost { background: transparent; color: var(--ppt-fg-primary); }
        .btn.primary { background: var(--ppt-color-primary); color: #fff; border-color: var(--ppt-color-primary); }
        .btn.primary:hover { background: var(--ppt-color-primary-hover); }
        @media (max-width: 640px) {
          .actions { width: 100%; justify-content: flex-end; }
        }
      `}</style>
    </div>
  );
}
