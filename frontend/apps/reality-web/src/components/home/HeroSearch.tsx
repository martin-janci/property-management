/**
 * HeroSearch Component
 *
 * Hero section with search form for homepage (Epic 44, Story 44.1).
 */

'use client';

import { useTranslations } from 'next-intl';
import { useRouter } from '../../../i18n/routing';
import { useState } from 'react';

export function HeroSearch() {
  const router = useRouter();
  const tHome = useTranslations('home');
  const tSearch = useTranslations('search');
  const [query, setQuery] = useState('');
  const [transactionType, setTransactionType] = useState<'sale' | 'rent'>('sale');

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    const params = new URLSearchParams();
    params.set('transactionType', transactionType);
    if (query.trim()) {
      params.set('q', query.trim());
    }
    router.push(`/listings?${params.toString()}`);
  };

  return (
    <section className="hero">
      <div className="hero-content">
        <h1 className="hero-title">{tHome('heroTitle')}</h1>
        <p className="hero-subtitle">{tHome('heroSubtitle')}</p>

        <form className="search-form" onSubmit={handleSearch}>
          {/* Transaction Type Toggle */}
          <div className="toggle-container">
            <button
              type="button"
              className={`toggle-button ${transactionType === 'sale' ? 'active' : ''}`}
              onClick={() => setTransactionType('sale')}
            >
              {tSearch('buy')}
            </button>
            <button
              type="button"
              className={`toggle-button ${transactionType === 'rent' ? 'active' : ''}`}
              onClick={() => setTransactionType('rent')}
            >
              {tSearch('rent')}
            </button>
          </div>

          {/* Search Input */}
          <div className="search-input-container">
            <svg
              className="search-icon"
              width="20"
              height="20"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <circle cx="11" cy="11" r="8" />
              <path d="m21 21-4.35-4.35" />
            </svg>
            <input
              type="text"
              className="search-input"
              placeholder={tSearch('placeholder')}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
            <button type="submit" className="search-button">
              {tSearch('button')}
            </button>
          </div>
        </form>

        {/* Quick Links */}
        <div className="quick-links">
          <span className="quick-links-label">{tSearch('popular')}</span>
          <button
            type="button"
            className="quick-link"
            onClick={() => router.push('/listings?city=Bratislava&transactionType=sale')}
          >
            Bratislava
          </button>
          <button
            type="button"
            className="quick-link"
            onClick={() => router.push('/listings?city=Prague&transactionType=sale')}
          >
            Prague
          </button>
          <button
            type="button"
            className="quick-link"
            onClick={() => router.push('/listings?city=Vienna&transactionType=sale')}
          >
            Vienna
          </button>
        </div>
      </div>

      <style jsx>{`
        .hero {
          /* The one gradient in the system: brand-800 → brand-500 */
          background: linear-gradient(135deg, var(--ppt-brand-800, #1e40af) 0%, var(--ppt-brand-500, #3b82f6) 100%);
          padding: 72px 32px 96px;
          position: relative;
          overflow: hidden;
        }

        .hero::before {
          content: '';
          position: absolute;
          inset: 0;
          background:
            radial-gradient(circle at 15% 20%, rgba(255,255,255,0.08) 0%, transparent 40%),
            radial-gradient(circle at 85% 80%, rgba(255,255,255,0.06) 0%, transparent 45%);
          pointer-events: none;
        }

        .hero-content {
          max-width: 960px;
          margin: 0 auto;
          position: relative;
          z-index: 1;
        }

        .hero-title {
          font-size: clamp(2rem, 4vw, 3rem);
          font-weight: 800;
          color: #fff;
          margin: 0 0 14px;
          letter-spacing: -0.02em;
          line-height: 1.1;
          max-width: 740px;
        }

        .hero-subtitle {
          font-size: 17px;
          color: rgba(255, 255, 255, 0.85);
          margin: 0 0 28px;
          max-width: 560px;
          line-height: 1.55;
        }

        .search-form {
          background: var(--ppt-bg-surface, #fff);
          border-radius: var(--ppt-radius-xl, 16px);
          padding: 8px;
          box-shadow: 0 20px 50px -20px rgba(0,0,0,0.4), 0 8px 20px -8px rgba(0,0,0,0.2);
          max-width: 860px;
        }

        .toggle-container {
          display: inline-flex;
          background: var(--ppt-bg-subtle, #f3f4f6);
          border-radius: var(--ppt-radius-md, 8px);
          padding: 3px;
          margin-bottom: 8px;
        }

        .toggle-button {
          padding: 7px 18px;
          border: none;
          background: transparent;
          border-radius: 6px;
          font-size: var(--ppt-font-size-sm, 14px);
          font-weight: var(--ppt-font-weight-medium, 500);
          color: var(--ppt-fg-muted, #6b7280);
          cursor: pointer;
          font-family: var(--ppt-font-family);
          transition: all var(--ppt-transition-fast);
        }

        .toggle-button.active {
          background: var(--ppt-bg-surface, #fff);
          color: var(--ppt-color-primary, #2563eb);
          font-weight: var(--ppt-font-weight-semibold, 600);
          box-shadow: 0 1px 2px rgba(0,0,0,0.08);
        }

        .search-input-container {
          display: flex;
          align-items: center;
          gap: 0;
          background: var(--ppt-bg-surface, #fff);
          border: 1px solid var(--ppt-border-default, #e5e7eb);
          border-radius: var(--ppt-radius-lg, 12px);
          padding: 4px 6px 4px 0;
          overflow: hidden;
        }

        .search-icon {
          color: var(--ppt-fg-muted, #9ca3af);
          flex-shrink: 0;
          padding: 0 12px;
        }

        .search-input {
          flex: 1;
          border: none;
          background: transparent;
          font-size: var(--ppt-font-size-sm, 14px);
          font-weight: var(--ppt-font-weight-medium, 500);
          padding: 12px 0;
          outline: none;
          color: var(--ppt-fg-primary, #111827);
          font-family: var(--ppt-font-family);
        }

        .search-input::placeholder {
          color: var(--ppt-fg-muted, #9ca3af);
          font-weight: var(--ppt-font-weight-normal, 400);
        }

        .search-button {
          padding: 11px 24px;
          background: var(--ppt-color-primary, #2563eb);
          color: #fff;
          border: none;
          border-radius: var(--ppt-radius-md, 10px);
          font-size: var(--ppt-font-size-sm, 14px);
          font-weight: var(--ppt-font-weight-semibold, 600);
          cursor: pointer;
          font-family: var(--ppt-font-family);
          transition: background var(--ppt-transition-fast);
          white-space: nowrap;
        }

        .search-button:hover {
          background: var(--ppt-color-primary-hover, #1d4ed8);
        }

        .search-button:focus-visible {
          outline: none;
          box-shadow: var(--ppt-focus-ring-shadow);
        }

        .quick-links {
          display: flex;
          align-items: center;
          gap: 10px;
          margin-top: 24px;
          flex-wrap: wrap;
        }

        .quick-links-label {
          color: rgba(255, 255, 255, 0.75);
          font-size: var(--ppt-font-size-sm, 14px);
          font-weight: var(--ppt-font-weight-medium, 500);
        }

        .quick-link {
          padding: 7px 16px;
          background: rgba(255, 255, 255, 0.12);
          border: 1px solid rgba(255, 255, 255, 0.2);
          border-radius: var(--ppt-radius-full, 9999px);
          color: #fff;
          font-size: var(--ppt-font-size-sm, 14px);
          font-weight: var(--ppt-font-weight-medium, 500);
          cursor: pointer;
          font-family: var(--ppt-font-family);
          transition: background var(--ppt-transition-fast);
        }

        .quick-link:hover {
          background: rgba(255, 255, 255, 0.22);
        }

        .quick-link:focus-visible {
          outline: none;
          box-shadow: 0 0 0 3px rgba(255,255,255,0.4);
        }
      `}</style>
    </section>
  );
}
