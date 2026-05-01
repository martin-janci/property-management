/**
 * Footer Component
 *
 * Site footer for Reality Portal (Epic 44).
 */

'use client';

import { useTranslations } from 'next-intl';
import Link from 'next/link';

export function Footer() {
  const t = useTranslations('footer');
  const currentYear = new Date().getFullYear();

  return (
    <footer className="footer">
      <div className="footer-inner">
        <div className="footer-grid">
          {/* Brand */}
          <div className="footer-section">
            <Link href="/" className="footer-logo">
              {t('brandName')}
            </Link>
            <p className="footer-description">{t('description')}</p>
          </div>

          {/* Quick Links */}
          <div className="footer-section">
            <h3 className="footer-title">{t('quickLinks')}</h3>
            <nav className="footer-nav">
              <Link href="/listings?transactionType=sale">{t('propertiesForSale')}</Link>
              <Link href="/listings?transactionType=rent">{t('propertiesForRent')}</Link>
              <Link href="/listings">{t('allListings')}</Link>
            </nav>
          </div>

          {/* Property Types */}
          <div className="footer-section">
            <h3 className="footer-title">{t('propertyTypes')}</h3>
            <nav className="footer-nav">
              <Link href="/listings?propertyType=apartment">{t('apartments')}</Link>
              <Link href="/listings?propertyType=house">{t('houses')}</Link>
              <Link href="/listings?propertyType=land">{t('land')}</Link>
              <Link href="/listings?propertyType=commercial">{t('commercial')}</Link>
            </nav>
          </div>

          {/* Company */}
          <div className="footer-section">
            <h3 className="footer-title">{t('company')}</h3>
            <nav className="footer-nav">
              <Link href="/about">{t('aboutUs')}</Link>
              <Link href="/contact">{t('contact')}</Link>
              <Link href="/privacy">{t('privacy')}</Link>
              <Link href="/terms">{t('terms')}</Link>
            </nav>
          </div>
        </div>

        {/* Bottom */}
        <div className="footer-bottom">
          <p>{t('copyright', { year: currentYear })}</p>
        </div>
      </div>

      <style jsx>{`
        .footer {
          background-color: var(--ppt-neutral-800);
          color: var(--ppt-border-default);
          margin-top: auto;
        }

        .footer-inner {
          max-width: 1280px;
          margin: 0 auto;
          padding: 48px 16px 24px;
        }

        .footer-grid {
          display: grid;
          grid-template-columns: 1fr;
          gap: 32px;
        }

        @media (min-width: 640px) {
          .footer-grid {
            grid-template-columns: repeat(2, 1fr);
          }
        }

        @media (min-width: 1024px) {
          .footer-grid {
            grid-template-columns: 2fr repeat(3, 1fr);
          }
        }

        .footer-section {
          display: flex;
          flex-direction: column;
          gap: 12px;
        }

        .footer-logo {
          font-size: 1.25rem;
          font-weight: bold;
          color: var(--ppt-fg-on-accent);
          text-decoration: none;
        }

        .footer-description {
          color: var(--ppt-fg-subtle);
          font-size: 14px;
          line-height: 1.6;
          margin: 0;
        }

        .footer-title {
          font-size: 14px;
          font-weight: 600;
          color: var(--ppt-fg-on-accent);
          text-transform: uppercase;
          letter-spacing: 0.05em;
          margin: 0 0 4px;
        }

        .footer-nav {
          display: flex;
          flex-direction: column;
          gap: 8px;
        }

        .footer-nav a {
          color: var(--ppt-fg-subtle);
          text-decoration: none;
          font-size: 14px;
          transition: color 0.2s;
        }

        .footer-nav a:hover {
          color: var(--ppt-fg-on-accent);
        }

        .footer-bottom {
          margin-top: 48px;
          padding-top: 24px;
          border-top: 1px solid var(--ppt-neutral-700);
          text-align: center;
        }

        .footer-bottom p {
          margin: 0;
          color: var(--ppt-fg-muted);
          font-size: 14px;
        }
      `}</style>
    </footer>
  );
}
