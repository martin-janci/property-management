'use client';

/**
 * Account dashboard (UC-47.6).
 *
 * Lists account-related actions for an authenticated user.
 */

import Link from 'next/link';
import { useTranslations } from 'next-intl';
import { ProtectedRoute } from '@/components/auth';
import { Footer, Header } from '@/components/ui';
import { useAuth } from '@/lib/auth-context';

const SECTIONS: ReadonlyArray<{
  href: string;
  key: 'profile' | 'password' | 'favorites' | 'savedSearches' | 'inquiries';
}> = [
  { href: '/account/profile', key: 'profile' },
  { href: '/account/password', key: 'password' },
  { href: '/favorites', key: 'favorites' },
  { href: '/saved-searches', key: 'savedSearches' },
  { href: '/inquiries', key: 'inquiries' },
];

function AccountContent() {
  const { user, logout } = useAuth();
  const t = useTranslations('pages.account');

  return (
    <div className="container">
      <header className="hero">
        <h1 className="title">{t('title')}</h1>
        <p className="subtitle">
          {t.rich('signedInAs', {
            email: user?.email ?? '',
            strong: (chunks) => <strong>{chunks}</strong>,
          })}
        </p>
      </header>

      <ul className="grid">
        {SECTIONS.map((section) => (
          <li key={section.href}>
            <Link href={section.href} className="card">
              <h2 className="card-title">{t(`sections.${section.key}.title`)}</h2>
              <p className="card-desc">{t(`sections.${section.key}.description`)}</p>
            </Link>
          </li>
        ))}
      </ul>

      <button type="button" className="logout" onClick={() => void logout()}>
        {t('signOut')}
      </button>

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
        .logout {
          margin-top: 32px; padding: 10px 20px; background: transparent;
          color: var(--ppt-color-danger-dark); border: 1px solid var(--ppt-color-danger-light); border-radius: 8px;
          font-weight: 500; cursor: pointer;
        }
        .logout:hover { background: var(--ppt-color-danger-light); }
      `}</style>
    </div>
  );
}

export default function AccountPage() {
  return (
    <div className="page">
      <Header />
      <main>
        <ProtectedRoute>
          <AccountContent />
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
