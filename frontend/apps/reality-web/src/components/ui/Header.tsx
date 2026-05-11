/**
 * Header Component
 *
 * Main navigation header for Reality Portal (Epic 44).
 * Includes language switcher for i18n support (Epic 111).
 */

'use client';

import { useTranslations } from 'next-intl';
import { useEffect, useRef, useState } from 'react';
import { useAuth } from '@/lib/auth-context';
import { Link, usePathname } from '../../i18n/routing';
import { LanguageSwitcher } from './LanguageSwitcher';

export function Header() {
  const { user, isAuthenticated, login, logout } = useAuth();
  const [showDropdown, setShowDropdown] = useState(false);
  const [showMobileMenu, setShowMobileMenu] = useState(false);
  const t = useTranslations();
  const pathname = usePathname();

  const dropdownRef = useRef<HTMLDivElement>(null);
  const mobileMenuRef = useRef<HTMLElement>(null);

  // Close dropdown on outside click
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setShowDropdown(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  // Close mobile menu on outside click
  useEffect(() => {
    if (!showMobileMenu) return;
    const handleClickOutside = (e: MouseEvent) => {
      if (
        mobileMenuRef.current &&
        !mobileMenuRef.current.contains(e.target as Node) &&
        !(e.target as HTMLElement).closest('.mobile-menu-toggle')
      ) {
        setShowMobileMenu(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [showMobileMenu]);

  const isActive = (href: string) => pathname === href || pathname.startsWith(`${href}?`);

  return (
    <header className="header">
      <div className="header-inner">
        {/* Logo — brand square + 2-tone wordmark. The square uses the same
            brand-600 the wordmark uses; the second word ("Portal") drops to
            fg-primary weight 600 so the brand sits visually first. Matches
            the design's `.logo` rule from colors_and_type.css. */}
        <Link href="/" className="logo" aria-label="Reality Portal">
          <span className="logo-mark" aria-hidden="true" />
          <span className="logo-text">
            Reality<em className="logo-text-em">Portal</em>
          </span>
        </Link>

        {/* Desktop Navigation — five links matching the design's standard
            header (Predaj / Prenájom / Predať / Magazín / Pomoc). The
            "Predať" entry (Sell) is the seller-info entry point; the
            primary "Pridať nehnuteľnosť" CTA on the right is the action
            shortcut. Both currently route to /sell — when a dedicated
            /for-sellers info page lands the nav entry will move there. */}
        <nav className="nav-desktop">
          <Link href="/listings?transactionType=sale" className="nav-link">
            {t('search.buy')}
          </Link>
          <Link href="/listings?transactionType=rent" className="nav-link">
            {t('search.rent')}
          </Link>
          <Link href="/sell" className={`nav-link ${isActive('/sell') ? 'nav-link-active' : ''}`}>
            {t('nav.sell')}
          </Link>
          <Link
            href="/journal"
            className={`nav-link ${isActive('/journal') ? 'nav-link-active' : ''}`}
          >
            {t('nav.journal')}
          </Link>
          <Link href="/help" className={`nav-link ${isActive('/help') ? 'nav-link-active' : ''}`}>
            {t('nav.help')}
          </Link>
        </nav>

        {/* Auth Section */}
        <div className="auth-section">
          {/* Seller CTA — shown on >=768px only (mobile menu has its own
              entry). Primary-color filled so it reads as the main action
              even alongside the auth slot. */}
          <Link href="/sell" className="list-cta">
            {t('nav.listProperty')}
          </Link>

          <LanguageSwitcher />

          {/*
           * Auth slot — has a fixed min-width on the wrapping `.auth-slot`
           * so swapping between "Sign in" button and the user avatar+name
           * after the async `/users/me` verification doesn't reflow the
           * surrounding header items. No skeleton: we either know the user
           * (optimistic from localStorage) or we know they're anonymous —
           * in both cases the final UI can render immediately.
           */}
          <div className="auth-slot">
            {isAuthenticated ? (
              <div className="user-container" ref={dropdownRef}>
                <button
                  type="button"
                  className="user-button"
                  onClick={() => setShowDropdown((v) => !v)}
                  aria-expanded={showDropdown}
                  aria-haspopup="true"
                >
                  <div className="avatar">{user?.name.charAt(0).toUpperCase()}</div>
                  <span className="user-name">{user?.name}</span>
                </button>

                {showDropdown && (
                  <div className="dropdown">
                    <div className="dropdown-header">
                      <p className="dropdown-name">{user?.name}</p>
                      <p className="dropdown-email">{user?.email}</p>
                    </div>
                    <div className="dropdown-menu">
                      <Link
                        href="/favorites"
                        className="menu-item"
                        onClick={() => setShowDropdown(false)}
                      >
                        {t('common.favorites')}
                      </Link>
                      <Link
                        href="/saved-searches"
                        className="menu-item"
                        onClick={() => setShowDropdown(false)}
                      >
                        {t('nav.savedSearches')}
                      </Link>
                      <Link
                        href="/inquiries"
                        className="menu-item"
                        onClick={() => setShowDropdown(false)}
                      >
                        {t('nav.myInquiries')}
                      </Link>
                      <Link
                        href="/account/profile"
                        className="menu-item"
                        onClick={() => setShowDropdown(false)}
                      >
                        {t('nav.profile')}
                      </Link>
                      <button type="button" onClick={logout} className="sign-out-button">
                        {t('common.logout')}
                      </button>
                    </div>
                  </div>
                )}
              </div>
            ) : (
              <button type="button" onClick={() => login()} className="sign-in-button">
                {t('common.login')}
              </button>
            )}
          </div>

          {/* Mobile Menu Toggle */}
          <button
            type="button"
            className="mobile-menu-toggle"
            onClick={() => setShowMobileMenu((v) => !v)}
            aria-label="Toggle menu"
            aria-expanded={showMobileMenu}
          >
            <svg
              width="24"
              height="24"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              {showMobileMenu ? (
                <path d="M6 18L18 6M6 6l12 12" />
              ) : (
                <path d="M4 6h16M4 12h16M4 18h16" />
              )}
            </svg>
          </button>
        </div>
      </div>

      {/* Mobile Navigation */}
      {showMobileMenu && (
        <nav className="nav-mobile" ref={mobileMenuRef}>
          <Link
            href="/listings?transactionType=sale"
            className="nav-link-mobile"
            onClick={() => setShowMobileMenu(false)}
          >
            {t('search.sale')}
          </Link>
          <Link
            href="/listings?transactionType=rent"
            className="nav-link-mobile"
            onClick={() => setShowMobileMenu(false)}
          >
            {t('search.rent')}
          </Link>
          <Link
            href="/listings"
            className="nav-link-mobile"
            onClick={() => setShowMobileMenu(false)}
          >
            {t('nav.allListings')}
          </Link>
          <Link
            href="/journal"
            className="nav-link-mobile"
            onClick={() => setShowMobileMenu(false)}
          >
            {t('nav.journal')}
          </Link>
          <Link href="/help" className="nav-link-mobile" onClick={() => setShowMobileMenu(false)}>
            {t('nav.help')}
          </Link>
          <Link
            href="/sell"
            className="nav-link-mobile sell-cta-mobile"
            onClick={() => setShowMobileMenu(false)}
          >
            {t('nav.listProperty')}
          </Link>
          {isAuthenticated && (
            <>
              <Link
                href="/favorites"
                className="nav-link-mobile"
                onClick={() => setShowMobileMenu(false)}
              >
                {t('common.favorites')}
              </Link>
              <Link
                href="/saved-searches"
                className="nav-link-mobile"
                onClick={() => setShowMobileMenu(false)}
              >
                {t('nav.savedSearches')}
              </Link>
              <Link
                href="/inquiries"
                className="nav-link-mobile"
                onClick={() => setShowMobileMenu(false)}
              >
                {t('nav.myInquiries')}
              </Link>
              <button
                type="button"
                className="nav-link-mobile auth-cta-mobile"
                onClick={() => {
                  setShowMobileMenu(false);
                  logout();
                }}
              >
                {t('common.logout')}
              </button>
            </>
          )}
          {/* Sign-in entry on mobile — replaces the auth-slot button that's
              hidden by media query below 768 px. */}
          {!isAuthenticated && (
            <button
              type="button"
              className="nav-link-mobile auth-cta-mobile"
              onClick={() => {
                setShowMobileMenu(false);
                login();
              }}
            >
              {t('common.login')}
            </button>
          )}
        </nav>
      )}

      <style jsx>{`
        .header {
          background-color: var(--ppt-bg-surface);
          border-bottom: 1px solid var(--ppt-border-default);
          position: sticky;
          top: 0;
          z-index: var(--ppt-z-sticky);
        }

        .header-inner {
          max-width: var(--ppt-content-max, 1280px);
          margin: 0 auto;
          padding: 0 32px;
          height: 64px;
          display: flex;
          align-items: center;
          gap: 24px;
        }

        /*
         * :global() — Link from next-intl/navigation renders its own <a>
         * outside styled-jsx's hashed-class scoping. The wrapping .header-inner
         * still carries the hash, keeping the rules tightly bound to this
         * component while the inner Link-rendered <a> still picks them up.
         */
        .header-inner :global(.logo) {
          display: inline-flex;
          align-items: center;
          gap: 8px;
          text-decoration: none;
          flex-shrink: 0;
        }
        .header-inner :global(.logo):focus-visible {
          outline: none;
          border-radius: var(--ppt-radius-md);
          box-shadow: var(--ppt-focus-ring-shadow);
        }
        .logo-mark {
          width: 28px;
          height: 28px;
          border-radius: 8px;
          background: var(--ppt-color-primary);
          flex-shrink: 0;
          display: inline-block;
        }
        .logo-text {
          font-size: 18px;
          font-weight: 800;
          color: var(--ppt-color-primary);
          letter-spacing: -0.02em;
          line-height: 1;
        }
        .logo-text-em {
          font-style: normal;
          color: var(--ppt-fg-primary);
          font-weight: 600;
          margin-left: 2px;
        }

        .nav-desktop {
          display: none;
          gap: 2px;
          margin-left: 16px;
        }

        @media (min-width: 768px) {
          .nav-desktop {
            display: flex;
          }
        }

        .nav-desktop :global(.nav-link) {
          padding: 8px 14px;
          border-radius: var(--ppt-radius-md);
          font-size: 13.5px;
          font-weight: var(--ppt-font-weight-medium);
          color: var(--ppt-fg-secondary);
          text-decoration: none;
          transition: background var(--ppt-transition-fast),
                      color var(--ppt-transition-fast);
        }

        .nav-desktop :global(.nav-link):hover {
          color: var(--ppt-color-primary);
          background: var(--ppt-color-primary-soft-bg);
        }

        .nav-desktop :global(.nav-link-active) {
          color: var(--ppt-color-primary);
          background: var(--ppt-color-primary-soft-bg);
        }

        .auth-section {
          display: flex;
          align-items: center;
          gap: 8px;
          margin-left: auto;
        }

        /* Primary "List your property" CTA in the header. Same blue as the
           brand square and the primary button used elsewhere; hidden on
           narrow viewports (mobile menu has its own /sell entry). */
        .auth-section :global(.list-cta) {
          display: none;
          padding: 8px 14px;
          background: var(--ppt-color-primary);
          color: var(--ppt-fg-on-accent, #fff);
          border-radius: var(--ppt-radius-md);
          font-size: var(--ppt-font-size-sm);
          font-weight: var(--ppt-font-weight-semibold);
          text-decoration: none;
          transition: background var(--ppt-transition-fast);
          white-space: nowrap;
        }
        .auth-section :global(.list-cta):hover {
          background: var(--ppt-color-primary-hover);
        }
        .auth-section :global(.list-cta):focus-visible {
          outline: none;
          box-shadow: var(--ppt-focus-ring-shadow);
        }
        @media (min-width: 1024px) {
          .auth-section :global(.list-cta) {
            display: inline-flex;
            align-items: center;
          }
        }

        /* Reserve a constant width so the swap between the sign-in button
           and the user avatar/name doesn't shift the surrounding items.
           34px avatar + 8px gap + ~80px username area + 16px padding ≈ 138px.
           Mobile (<768px) hides the username, so 60px is enough. */
        .auth-slot {
          min-width: 138px;
          min-height: 36px;
          display: inline-flex;
          align-items: center;
          justify-content: flex-end;
        }

        @media (max-width: 767px) {
          .auth-slot {
            /* Hide the entire auth slot on mobile — the hamburger menu has
               its own Sign in / Logout entry, so duplicating the affordance
               here costs ~140 px of header width and forces overflow. */
            display: none;
          }
        }

        .sign-in-button {
          padding: 9px 14px;
          background-color: transparent;
          color: var(--ppt-fg-secondary);
          border-radius: var(--ppt-radius-md);
          border: 1px solid var(--ppt-border-default);
          cursor: pointer;
          font-size: var(--ppt-font-size-sm);
          font-weight: var(--ppt-font-weight-medium);
          font-family: var(--ppt-font-family);
          transition: background var(--ppt-transition-fast),
                      border-color var(--ppt-transition-fast);
        }

        .sign-in-button:hover {
          background-color: var(--ppt-bg-subtle);
          border-color: var(--ppt-border-strong);
        }

        .sign-in-button:focus-visible {
          outline: none;
          box-shadow: var(--ppt-focus-ring-shadow);
        }

        .user-container {
          position: relative;
        }

        .user-button {
          display: flex;
          align-items: center;
          gap: 8px;
          padding: 5px 8px;
          border-radius: var(--ppt-radius-md);
          border: none;
          background-color: transparent;
          cursor: pointer;
          font-family: var(--ppt-font-family);
          transition: background var(--ppt-transition-fast);
        }

        .user-button:hover {
          background-color: var(--ppt-bg-subtle);
        }

        .user-button:focus-visible {
          outline: none;
          box-shadow: var(--ppt-focus-ring-shadow);
        }

        .avatar {
          width: 34px;
          height: 34px;
          border-radius: var(--ppt-radius-full);
          background: linear-gradient(135deg, var(--ppt-brand-500), var(--ppt-color-primary-hover));
          color: var(--ppt-fg-on-accent);
          display: flex;
          align-items: center;
          justify-content: center;
          font-weight: var(--ppt-font-weight-bold);
          font-size: 12px;
          flex-shrink: 0;
        }

        .user-name {
          font-size: var(--ppt-font-size-sm);
          font-weight: var(--ppt-font-weight-medium);
          color: var(--ppt-fg-secondary);
          display: none;
        }

        @media (min-width: 768px) {
          .user-name {
            display: block;
          }
        }

        .dropdown {
          position: absolute;
          right: 0;
          top: 100%;
          margin-top: 8px;
          width: 220px;
          background-color: var(--ppt-bg-elevated);
          border-radius: var(--ppt-radius-lg);
          box-shadow: var(--ppt-shadow-popover);
          border: 1px solid var(--ppt-border-default);
          z-index: var(--ppt-z-dropdown);
        }

        .dropdown-header {
          padding: 12px 14px;
          border-bottom: 1px solid var(--ppt-border-default);
        }

        .dropdown-name {
          font-size: var(--ppt-font-size-sm);
          font-weight: var(--ppt-font-weight-semibold);
          color: var(--ppt-fg-primary);
          margin: 0;
        }

        .dropdown-email {
          font-size: var(--ppt-font-size-xs);
          color: var(--ppt-fg-muted);
          margin: 4px 0 0;
        }

        .dropdown-menu {
          padding: 4px;
        }

        .dropdown-menu :global(.menu-item) {
          display: block;
          width: 100%;
          padding: 8px 10px;
          font-size: var(--ppt-font-size-sm);
          color: var(--ppt-fg-secondary);
          text-decoration: none;
          border-radius: var(--ppt-radius-sm);
          transition: background var(--ppt-transition-fast);
        }

        .dropdown-menu :global(.menu-item):hover {
          background-color: var(--ppt-bg-subtle);
        }

        .sign-out-button {
          display: block;
          width: 100%;
          padding: 8px 10px;
          font-size: var(--ppt-font-size-sm);
          color: var(--ppt-color-danger-hover);
          text-align: left;
          border: none;
          background-color: transparent;
          cursor: pointer;
          border-radius: var(--ppt-radius-sm);
          font-family: var(--ppt-font-family);
          transition: background var(--ppt-transition-fast);
        }

        .sign-out-button:hover {
          background-color: var(--ppt-color-danger-light);
        }

        .mobile-menu-toggle {
          display: flex;
          padding: 8px;
          border: none;
          background: transparent;
          cursor: pointer;
          color: var(--ppt-fg-secondary);
          border-radius: var(--ppt-radius-md);
          transition: background var(--ppt-transition-fast);
        }

        .mobile-menu-toggle:hover {
          background: var(--ppt-bg-subtle);
        }

        @media (min-width: 768px) {
          .mobile-menu-toggle {
            display: none;
          }
        }

        .nav-mobile {
          display: flex;
          flex-direction: column;
          padding: 8px 16px 16px;
          border-top: 1px solid var(--ppt-border-default);
          background: var(--ppt-bg-surface);
        }

        @media (min-width: 768px) {
          .nav-mobile {
            display: none;
          }
        }

        .nav-mobile :global(.nav-link-mobile) {
          padding: 12px 4px;
          color: var(--ppt-fg-secondary);
          text-decoration: none;
          font-size: var(--ppt-font-size-sm);
          font-weight: var(--ppt-font-weight-medium);
          border-bottom: 1px solid var(--ppt-border-subtle);
        }

        .nav-mobile :global(.nav-link-mobile):hover {
          color: var(--ppt-color-primary);
        }

        .nav-mobile :global(.nav-link-mobile):last-child {
          border-bottom: none;
        }

        /* Highlight the seller CTA inside the mobile menu so it doesn't blend
           with the other nav rows. Keeps the same border-row look but tints
           the text and adds the same arrow affordance the design uses. */
        .nav-mobile :global(.nav-link-mobile.sell-cta-mobile) {
          color: var(--ppt-color-primary);
          font-weight: var(--ppt-font-weight-semibold);
        }
        .nav-mobile :global(.nav-link-mobile.sell-cta-mobile)::after {
          content: ' →';
        }

        /* Sign in / Logout entries inside the mobile menu — render as
           buttons (they trigger login()/logout() rather than navigation)
           but keep the same row look as the link entries. */
        .nav-mobile :global(.nav-link-mobile.auth-cta-mobile) {
          background: transparent;
          border: none;
          border-bottom: 1px solid var(--ppt-border-subtle);
          cursor: pointer;
          text-align: left;
          font-family: var(--ppt-font-family);
          width: 100%;
        }
      `}</style>
    </header>
  );
}
