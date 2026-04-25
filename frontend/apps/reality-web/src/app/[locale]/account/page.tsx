'use client';

/**
 * Account dashboard (UC-47.6).
 *
 * Lists account-related actions for an authenticated user.
 */

import { ProtectedRoute } from '@/components/auth';
import { Footer, Header } from '@/components/ui';
import { useAuth } from '@/lib/auth-context';
import Link from 'next/link';

const SECTIONS: ReadonlyArray<{
  href: string;
  title: string;
  description: string;
}> = [
  {
    href: '/account/profile',
    title: 'Profile',
    description: 'Update your name, contact details and avatar.',
  },
  {
    href: '/account/password',
    title: 'Password',
    description: 'Change the password used to sign in.',
  },
  {
    href: '/favorites',
    title: 'Favorites',
    description: 'Properties you have saved for later.',
  },
  {
    href: '/saved-searches',
    title: 'Saved searches',
    description: 'Manage your saved searches and alert preferences.',
  },
  {
    href: '/inquiries',
    title: 'Inquiries',
    description: 'View messages and viewing requests you have sent.',
  },
];

function AccountContent() {
  const { user, logout } = useAuth();

  return (
    <div className="container">
      <header className="hero">
        <h1 className="title">Account</h1>
        <p className="subtitle">
          Signed in as <strong>{user?.email}</strong>
        </p>
      </header>

      <ul className="grid">
        {SECTIONS.map((section) => (
          <li key={section.href}>
            <Link href={section.href} className="card">
              <h2 className="card-title">{section.title}</h2>
              <p className="card-desc">{section.description}</p>
            </Link>
          </li>
        ))}
      </ul>

      <button type="button" className="logout" onClick={() => void logout()}>
        Sign out
      </button>

      <style jsx>{`
        .container { max-width: 960px; margin: 0 auto; padding: 32px 16px; }
        .hero { margin-bottom: 24px; }
        .title { font-size: 2rem; font-weight: 700; color: #111827; margin: 0 0 4px; }
        .subtitle { color: #6b7280; margin: 0; }
        .grid {
          display: grid; gap: 16px; padding: 0; margin: 0; list-style: none;
          grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
        }
        .card {
          display: block; background: #fff; padding: 20px; border-radius: 12px;
          box-shadow: 0 1px 3px rgba(0,0,0,.1); text-decoration: none; color: inherit;
        }
        .card:hover { box-shadow: 0 4px 10px rgba(0,0,0,.08); }
        .card-title { font-size: 1.125rem; font-weight: 600; margin: 0 0 4px; color: #111827; }
        .card-desc { font-size: 14px; color: #6b7280; margin: 0; }
        .logout {
          margin-top: 32px; padding: 10px 20px; background: transparent;
          color: #b91c1c; border: 1px solid #fecaca; border-radius: 8px;
          font-weight: 500; cursor: pointer;
        }
        .logout:hover { background: #fef2f2; }
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
        .page { min-height: 100vh; display: flex; flex-direction: column; background: #f9fafb; }
        main { flex: 1; }
      `}</style>
    </div>
  );
}
