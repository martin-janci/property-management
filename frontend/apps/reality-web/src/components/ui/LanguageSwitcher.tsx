'use client';

import { useLocale } from 'next-intl';
import { useTransition } from 'react';
import { type Locale, localeFlags, localeNames, locales } from '../../i18n';
import { usePathname, useRouter } from '../../i18n/routing';

export function LanguageSwitcher() {
  const locale = useLocale() as Locale;
  const router = useRouter();
  const pathname = usePathname();
  const [isPending, startTransition] = useTransition();

  const handleLocaleChange = (newLocale: Locale) => {
    startTransition(() => {
      router.replace(pathname, { locale: newLocale });
    });
  };

  return (
    <div className="lang-wrap">
      <select
        value={locale}
        onChange={(e) => handleLocaleChange(e.target.value as Locale)}
        disabled={isPending}
        className="lang-select"
        aria-label="Select language"
      >
        {locales.map((loc) => (
          <option key={loc} value={loc}>
            {localeFlags[loc]} {localeNames[loc]}
          </option>
        ))}
      </select>
      {/* Width/height attributes are explicit so the chevron stays at 16px
          even before any stylesheet has loaded — without them the SVG
          expands to ~300px when CSS is still in flight, which was the
          dominant CLS contributor on the homepage (single 0.92 shift). */}
      <svg
        className="lang-chevron"
        width="16"
        height="16"
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
        aria-hidden="true"
      >
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
      </svg>
      <style jsx>{`
        .lang-wrap {
          position: relative;
          display: inline-flex;
          align-items: center;
        }
        .lang-select {
          appearance: none;
          -webkit-appearance: none;
          background: transparent;
          border: 1px solid var(--ppt-border-default);
          border-radius: var(--ppt-radius-md);
          padding: 6px 28px 6px 12px;
          font-size: var(--ppt-font-size-sm);
          font-family: var(--ppt-font-family);
          color: var(--ppt-fg-secondary);
          cursor: pointer;
          transition: border-color var(--ppt-transition-fast);
        }
        .lang-select:hover {
          border-color: var(--ppt-border-strong);
        }
        .lang-select:focus-visible {
          outline: none;
          box-shadow: var(--ppt-focus-ring-shadow);
        }
        .lang-select:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }
        .lang-chevron {
          position: absolute;
          right: 8px;
          top: 50%;
          transform: translateY(-50%);
          color: var(--ppt-fg-muted);
          pointer-events: none;
        }
      `}</style>
    </div>
  );
}
