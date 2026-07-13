/**
 * Price Alerts Page
 *
 * Dedicated price-alert subscription surface — lists price-change and
 * back-on-market alerts for the user's favorited listings (Story 84.3).
 */

'use client';

import Link from 'next/link';
import { useTranslations } from 'next-intl';
import { ProtectedRoute } from '@/components/auth';
import { PriceAlerts } from '@/components/favorites';
import { Footer, Header } from '@/components/ui';

export default function PriceAlertsPage() {
  const t = useTranslations('priceAlerts');
  return (
    <div className="page-container">
      <Header />
      <main className="main">
        <div className="container">
          <nav className="breadcrumb">
            <Link href="/favorites" className="crumb-link">
              {t('backToFavorites')}
            </Link>
          </nav>
          <h1 className="page-title">{t('h1')}</h1>
          <p className="page-subtitle">{t('subtitle')}</p>
          <ProtectedRoute>
            <PriceAlerts />
          </ProtectedRoute>
        </div>
      </main>
      <Footer />

      <style jsx>{`
        .page-container {
          min-height: 100vh;
          display: flex;
          flex-direction: column;
          background: var(--ppt-bg-app);
        }
        .main { flex: 1; padding: 32px 0; }
        .container {
          max-width: 960px;
          margin: 0 auto;
          padding: 0 16px;
        }
        .breadcrumb { margin: 0 0 16px; }
        .crumb-link {
          font-size: 14px;
          color: var(--ppt-fg-muted);
          text-decoration: none;
        }
        .crumb-link:hover { text-decoration: underline; }
        .page-title {
          font-size: 2rem;
          font-weight: bold;
          color: var(--ppt-fg-primary);
          margin: 0 0 8px;
        }
        .page-subtitle {
          margin: 0 0 32px;
          color: var(--ppt-fg-muted);
        }
      `}</style>
    </div>
  );
}
