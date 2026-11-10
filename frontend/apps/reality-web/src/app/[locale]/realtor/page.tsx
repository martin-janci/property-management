'use client';

/**
 * Realtor dashboard hub (UC-51).
 *
 * Entry point for individual realtors: links to profile, listing
 * management and analytics.
 */

import { useMyListings } from '@ppt/reality-api-client';
import Link from 'next/link';
import { useTranslations } from 'next-intl';
import { ProtectedRoute } from '@/components/auth';
import { Footer, Header } from '@/components/ui';
import { useAuth } from '@/lib/auth-context';

const SECTIONS: ReadonlyArray<{
  href: string;
  titleKey: 'profileTitle' | 'listingsTitle' | 'createTitle' | 'analyticsTitle';
  descKey: 'profileDesc' | 'listingsDesc' | 'createDesc' | 'analyticsDesc';
}> = [
  { href: '/realtor/profile', titleKey: 'profileTitle', descKey: 'profileDesc' },
  { href: '/realtor/listings', titleKey: 'listingsTitle', descKey: 'listingsDesc' },
  { href: '/realtor/listings/new', titleKey: 'createTitle', descKey: 'createDesc' },
  { href: '/realtor/analytics', titleKey: 'analyticsTitle', descKey: 'analyticsDesc' },
];

function RealtorContent() {
  const t = useTranslations('pages.realtorHub');
  const { user } = useAuth();
  const { data: listings } = useMyListings({ limit: 1 });

  return (
    <div className="container">
      <header className="hero">
        <h1 className="title">{t('title')}</h1>
        <p className="subtitle">
          {user ? t('greetingNamed', { name: user.name }) : t('greeting')}{' '}
          {t('listingsSummary', { count: listings?.total ?? 0 })}
        </p>
      </header>

      <ul className="grid">
        {SECTIONS.map((section) => (
          <li key={section.href}>
            <Link href={section.href} className="card">
              <h2 className="card-title">{t(section.titleKey)}</h2>
              <p className="card-desc">{t(section.descKey)}</p>
            </Link>
          </li>
        ))}
      </ul>

      <style jsx>{`
        .container { max-width: 960px; margin: 0 auto; padding: 32px 16px; }
        .hero { margin-bottom: 24px; }
        .title { font-size: 2rem; font-weight: 700; color: var(--ppt-fg-primary); margin: 0 0 4px; }
        .subtitle { color: var(--ppt-fg-muted); margin: 0; }
        .grid {
          display: grid; gap: 16px; padding: 0; margin: 0; list-style: none;
          grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
        }
        .card {
          display: block; background: var(--ppt-bg-surface); padding: 20px; border-radius: 12px;
          box-shadow: 0 1px 3px rgba(0,0,0,.1); text-decoration: none; color: inherit;
        }
        .card:hover { box-shadow: 0 4px 10px rgba(0,0,0,.08); }
        .card-title { font-size: 1.125rem; font-weight: 600; margin: 0 0 4px; color: var(--ppt-fg-primary); }
        .card-desc { font-size: 14px; color: var(--ppt-fg-muted); margin: 0; }
      `}</style>
    </div>
  );
}

export default function RealtorPage() {
  return (
    <div className="page">
      <Header />
      <main>
        <ProtectedRoute>
          <RealtorContent />
        </ProtectedRoute>
      </main>
      <Footer />
      <style jsx>{`
        .page { min-height: 100vh; display: flex; flex-direction: column; background: var(--ppt-bg-app); }
        main { flex: 1; }
      `}</style>
    </div>
  );
}
