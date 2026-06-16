'use client';

/**
 * LanguageSwitcher — custom popover dropdown.
 *
 * Replaces the previous native <select>, which (a) looked OS-dependent
 * (Windows vs macOS rendering wildly different) and (b) couldn't be themed
 * to match the design's pill-on-surface look. This is a self-contained
 * popover with a globe-icon trigger; the menu opens below-right, lists all
 * six locales with their flag and full name, and closes on outside click /
 * Escape / locale selection.
 */

import { useLocale } from 'next-intl';
import { useEffect, useRef, useState } from 'react';
import { type Locale, localeFlags, localeNames, locales } from '../../i18n';
import { usePathname, useRouter } from '../../i18n/routing';

export function LanguageSwitcher() {
  const locale = useLocale() as Locale;
  const router = useRouter();
  const pathname = usePathname();
  const [open, setOpen] = useState(false);
  // Roving highlight in the open menu — starts on the currently-active
  // locale and moves with Arrow keys. aria-activedescendant is set on the
  // listbox so the assistive-tech focus indicator follows it without us
  // having to move DOM focus around.
  const [activeIndex, setActiveIndex] = useState(() => Math.max(0, locales.indexOf(locale)));
  const wrapRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);

  // Re-sync the highlight to the active locale whenever it changes from
  // outside (e.g. URL-driven locale swap).
  useEffect(() => {
    const i = locales.indexOf(locale);
    if (i >= 0) setActiveIndex(i);
  }, [locale]);

  const choose = useCallback(
    (next: Locale) => {
      setOpen(false);
      triggerRef.current?.focus();
      if (next === locale) return;
      router.replace(pathname, { locale: next });
    },
    [locale, pathname, router]
  );

  // Keyboard navigation while the menu is open. Listening on the document
  // because the listbox itself doesn't take focus (the trigger keeps it,
  // per APG listbox-with-aria-activedescendant pattern).
  useEffect(() => {
    if (!open) return;
    const onClick = (e: MouseEvent) => {
      if (wrapRef.current && !wrapRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        setOpen(false);
        triggerRef.current?.focus();
        return;
      }
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setActiveIndex((i) => Math.min(locales.length - 1, i + 1));
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setActiveIndex((i) => Math.max(0, i - 1));
      } else if (e.key === 'Home') {
        e.preventDefault();
        setActiveIndex(0);
      } else if (e.key === 'End') {
        e.preventDefault();
        setActiveIndex(locales.length - 1);
      } else if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        setActiveIndex((i) => {
          const target = locales[i];
          if (target) choose(target);
          return i;
        });
      }
    };
    document.addEventListener('mousedown', onClick);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onClick);
      document.removeEventListener('keydown', onKey);
    };
    // choose isn't in deps because the underlying functions (router,
    // pathname) are referentially-stable from next-intl across renders.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, choose]);

  const handleTriggerKey = (e: React.KeyboardEvent<HTMLButtonElement>) => {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      setOpen(true);
    }
  };

  return (
    <div className="lang-wrap" ref={wrapRef}>
      <button
        ref={triggerRef}
        type="button"
        className="lang-trigger"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={`Language: ${localeNames[locale]}`}
        // aria-activedescendant points at the highlighted option while
        // the menu is open. Lives on the trigger (which keeps focus) per
        // APG combobox pattern.
        aria-activedescendant={open ? `lang-opt-${locales[activeIndex]}` : undefined}
        onClick={() => setOpen((v) => !v)}
        onKeyDown={handleTriggerKey}
      >
        {/* Globe icon — 16px, stroke 2 (Feather/Lucide style). Explicit
            width/height attributes keep it sized even before CSS arrives. */}
        <svg
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <circle cx="12" cy="12" r="10" />
          <line x1="2" y1="12" x2="22" y2="12" />
          <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
        </svg>
        <span className="code">{locale.toUpperCase()}</span>
        <svg
          className="caret"
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2.5"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <polyline points="6 9 12 15 18 9" />
        </svg>
      </button>

      {open && (
        // <div role="listbox"> rather than <ul role="listbox"> — biome's
        // noNoninteractiveElementToInteractiveRole rejects the ARIA role
        // on a semantic list element. ARIA Combobox pattern works with
        // plain div containers.
        <div className="lang-menu" role="listbox" aria-label="Select language">
          {locales.map((loc, idx) => (
            <button
              key={loc}
              id={`lang-opt-${loc}`}
              type="button"
              role="option"
              aria-selected={loc === locale}
              className={[
                'lang-item',
                loc === locale ? 'active' : '',
                idx === activeIndex ? 'highlight' : '',
              ]
                .filter(Boolean)
                .join(' ')}
              onClick={() => choose(loc)}
              onMouseEnter={() => setActiveIndex(idx)}
            >
              <span className="flag" aria-hidden="true">
                {localeFlags[loc]}
              </span>
              <span className="name">{localeNames[loc]}</span>
              {loc === locale && (
                <svg
                  className="check"
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2.5"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  aria-hidden="true"
                >
                  <polyline points="20 6 9 17 4 12" />
                </svg>
              )}
            </button>
          ))}
        </div>
      )}

      <style jsx>{`
        .lang-wrap {
          position: relative;
          display: inline-flex;
          align-items: center;
        }
        .lang-trigger {
          display: inline-flex;
          align-items: center;
          gap: 6px;
          padding: 7px 10px;
          background: transparent;
          border: 1px solid var(--ppt-border-default);
          border-radius: var(--ppt-radius-md);
          color: var(--ppt-fg-secondary);
          font-family: var(--ppt-font-family);
          font-size: var(--ppt-font-size-sm);
          font-weight: var(--ppt-font-weight-medium);
          cursor: pointer;
          transition: background var(--ppt-transition-fast),
            border-color var(--ppt-transition-fast);
        }
        .lang-trigger:hover {
          background: var(--ppt-bg-subtle);
          border-color: var(--ppt-border-strong);
        }
        .lang-trigger:focus-visible {
          outline: none;
          box-shadow: var(--ppt-focus-ring-shadow);
        }
        .lang-trigger[aria-expanded='true'] {
          background: var(--ppt-bg-subtle);
          border-color: var(--ppt-border-strong);
        }
        .code {
          font-variant-numeric: tabular-nums;
          letter-spacing: 0.02em;
        }
        .caret {
          opacity: 0.6;
          transition: transform var(--ppt-transition-fast);
        }
        .lang-trigger[aria-expanded='true'] .caret {
          transform: rotate(180deg);
        }
        .lang-menu {
          position: absolute;
          top: calc(100% + 6px);
          right: 0;
          min-width: 200px;
          padding: 4px;
          margin: 0;
          list-style: none;
          background: var(--ppt-bg-elevated);
          border: 1px solid var(--ppt-border-default);
          border-radius: var(--ppt-radius-lg);
          box-shadow: var(--ppt-shadow-popover, 0 8px 24px rgba(0, 0, 0, 0.12));
          z-index: var(--ppt-z-dropdown, 1000);
        }
        .lang-item {
          display: flex;
          align-items: center;
          gap: 10px;
          width: 100%;
          padding: 8px 10px;
          background: transparent;
          border: none;
          border-radius: var(--ppt-radius-sm);
          cursor: pointer;
          font-family: var(--ppt-font-family);
          font-size: var(--ppt-font-size-sm);
          color: var(--ppt-fg-secondary);
          text-align: left;
          transition: background var(--ppt-transition-fast);
        }
        .lang-item:hover,
        .lang-item.highlight {
          background: var(--ppt-bg-subtle);
        }
        .lang-item.active {
          color: var(--ppt-color-primary);
          font-weight: var(--ppt-font-weight-semibold);
        }
        .lang-item:focus-visible {
          outline: none;
          box-shadow: var(--ppt-focus-ring-shadow);
        }
        .flag {
          font-size: 16px;
          line-height: 1;
        }
        .name {
          flex: 1;
        }
        .check {
          color: var(--ppt-color-primary);
          flex-shrink: 0;
        }
      `}</style>
    </div>
  );
}
