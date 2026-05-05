/**
 * Header Component
 *
 * Main navigation header for Reality Portal (Epic 44).
 * Includes language switcher for i18n support (Epic 111).
 */

'use client';

import { useAuth } from '@/lib/auth-context';
import { useTranslations } from 'next-intl';
import { useEffect, useRef, useState } from 'react';
import { Link, usePathname } from '../../i18n/routing';
import { LanguageSwitcher } from './LanguageSwitcher';

export function Header() {
  const { user, isLoading, isAuthenticated, login, logout } = useAuth();
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

  const isActive = (href: string) => pathname === href || pathname.startsWith(href + '?');

  return (
    <header className="header">
      <div className="header-inner">
        {/* Logo */}
        <Link href="/" className="logo">
          Reality Portal
        </Link>

        {/* Desktop Navigation */}
        <nav className="nav-desktop">
          <Link
            href="/listings?transactionType=sale"
            className={`nav-link ${isActive('/listings') ? 'nav-link-active' : ''}`}
          >
            {t('search.sale')}
          </Link>
          <Link
            href="/listings?transactionType=rent"
            className={`nav-link ${isActive('/listings') ? 'nav-link-active' : ''}`}
          >
            {t('search.rent')}
          </Link>
          <Link
            href="/listings"
            className={`nav-link ${isActive('/listings') ? 'nav-link-active' : ''}`}
          >
            {t('nav.allListings')}
          </Link>
        </nav>

        {/* Auth Section */}
        <div className="auth-section">
          <LanguageSwitcher />

          {isLoading ? (
            <div className="skeleton" />
          ) : isAuthenticated ? (
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
                    <Link href="/favorites" className="menu-item" onClick={() => setShowDropdown(false)}>
                      {t('common.favorites')}
                    </Link>
                    <Link href="/saved-searches" className="menu-item" onClick={() => setShowDropdown(false)}>
                      {t('nav.savedSearches')}
                    </Link>
                    <Link href="/inquiries" className="menu-item" onClick={() => setShowDropdown(false)}>
                      {t('nav.myInquiries')}
                    </Link>
                    <Link href="/account/profile" className="menu-item" onClick={() => setShowDropdown(false)}>
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
            </>
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

        .logo {
          font-size: 18px;
          font-weight: 800;
          color: var(--ppt-color-primary);
          text-decoration: none;
          letter-spacing: -0.02em;
          flex-shrink: 0;
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

        .nav-link {
          padding: 8px 14px;
          border-radius: var(--ppt-radius-md);
          font-size: 13.5px;
          font-weight: var(--ppt-font-weight-medium);
          color: var(--ppt-fg-secondary);
          text-decoration: none;
          transition: background var(--ppt-transition-fast),
                      color var(--ppt-transition-fast);
        }

        .nav-link:hover {
          color: var(--ppt-color-primary);
          background: var(--ppt-color-primary-soft-bg);
        }

        .nav-link-active {
          color: var(--ppt-color-primary);
          background: var(--ppt-color-primary-soft-bg);
        }

        .auth-section {
          display: flex;
          align-items: center;
          gap: 8px;
          margin-left: auto;
        }

        .skeleton {
          height: 36px;
          width: 96px;
          background-color: var(--ppt-border-default);
          border-radius: var(--ppt-radius-md);
          animation: skeleton-pulse 1.5s ease infinite;
        }

        @keyframes skeleton-pulse {
          0%, 100% { opacity: 1; }
          50% { opacity: 0.5; }
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

        .menu-item {
          display: block;
          width: 100%;
          padding: 8px 10px;
          font-size: var(--ppt-font-size-sm);
          color: var(--ppt-fg-secondary);
          text-decoration: none;
          border-radius: var(--ppt-radius-sm);
          transition: background var(--ppt-transition-fast);
        }

        .menu-item:hover {
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

        .nav-link-mobile {
          padding: 12px 4px;
          color: var(--ppt-fg-secondary);
          text-decoration: none;
          font-size: var(--ppt-font-size-sm);
          font-weight: var(--ppt-font-weight-medium);
          border-bottom: 1px solid var(--ppt-border-subtle);
        }

        .nav-link-mobile:hover {
          color: var(--ppt-color-primary);
        }

        .nav-link-mobile:last-child {
          border-bottom: none;
        }
      `}</style>
    </header>
  );
}
