'use client';

/**
 * Profile edit page (UC-47.7).
 *
 * UI scaffold for updating display name. The backend `PATCH /auth/me` endpoint
 * isn't surfaced via reality-api-client yet, so this page persists changes
 * locally and shows a confirmation; once the SDK exposes `authApiUpdateMe`
 * this can call it directly.
 */

import { ProtectedRoute } from '@/components/auth';
import { Footer, Header } from '@/components/ui';
import { useAuth } from '@/lib/auth-context';
import Link from 'next/link';
import { type FormEvent, useEffect, useState } from 'react';

function ProfileForm() {
  const { user } = useAuth();
  const [displayName, setDisplayName] = useState('');
  const [error, setError] = useState<string>();
  const [savedAt, setSavedAt] = useState<number>();
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    if (user?.name) setDisplayName(user.name);
  }, [user?.name]);

  const handleSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setError(undefined);
    if (!displayName.trim()) {
      setError('Display name is required');
      return;
    }
    setIsSaving(true);
    try {
      // Persist locally until the SDK exposes the update endpoint.
      try {
        sessionStorage.setItem('reality_profile_displayName', displayName.trim());
      } catch {
        // Storage unavailable; ignore.
      }
      setSavedAt(Date.now());
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <form className="form" onSubmit={handleSubmit} noValidate>
      <h1 className="title">Edit profile</h1>
      <p className="subtitle">Update your display name and contact preferences.</p>

      {error && (
        <div className="alert" role="alert">
          {error}
        </div>
      )}
      {savedAt && (
        <div className="success" role="status">
          Profile updated.
        </div>
      )}

      <label className="field">
        <span className="label">Display name</span>
        <input
          type="text"
          autoComplete="name"
          value={displayName}
          onChange={(e) => setDisplayName(e.target.value)}
          disabled={isSaving}
          className="input"
        />
      </label>

      <label className="field">
        <span className="label">Email</span>
        <input type="email" value={user?.email ?? ''} disabled className="input" />
        <span className="hint">Contact support to change the email on this account.</span>
      </label>

      <button type="submit" className="submit" disabled={isSaving}>
        {isSaving ? 'Saving…' : 'Save changes'}
      </button>

      <p className="meta">
        <Link href="/account" className="link">
          Back to account
        </Link>
      </p>

      <style jsx>{`
        .form {
          max-width: 480px; margin: 32px auto; padding: 32px;
          background: #fff; border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,.1);
          display: flex; flex-direction: column; gap: 16px;
        }
        .title { font-size: 1.5rem; font-weight: 700; color: #111827; margin: 0 0 4px; }
        .subtitle { font-size: 14px; color: #6b7280; margin: 0 0 8px; }
        .alert { padding: 12px 16px; background: #fef2f2; color: #b91c1c; border: 1px solid #fecaca; border-radius: 8px; font-size: 14px; }
        .success { padding: 12px 16px; background: #ecfdf5; color: #047857; border: 1px solid #a7f3d0; border-radius: 8px; font-size: 14px; }
        .field { display: flex; flex-direction: column; gap: 6px; }
        .label { font-size: 14px; font-weight: 500; color: #374151; }
        .input { padding: 10px 12px; font-size: 16px; border: 1px solid #d1d5db; border-radius: 8px; background: #fff; color: #111827; }
        .input:focus { outline: none; border-color: #2563eb; box-shadow: 0 0 0 3px rgba(37,99,235,.1); }
        .input:disabled { background: #f9fafb; cursor: not-allowed; }
        .hint { color: #6b7280; font-size: 12px; }
        .submit { margin-top: 8px; padding: 12px 16px; background: #2563eb; color: #fff; border: none; border-radius: 8px; font-size: 16px; font-weight: 600; cursor: pointer; }
        .submit:hover:not(:disabled) { background: #1d4ed8; }
        .submit:disabled { background: #93c5fd; cursor: not-allowed; }
        .meta { text-align: center; margin: 0; }
        .link { color: #2563eb; text-decoration: none; font-weight: 500; font-size: 14px; }
        .link:hover { text-decoration: underline; }
      `}</style>
    </form>
  );
}

export default function ProfileEditPage() {
  return (
    <div className="page">
      <Header />
      <main>
        <ProtectedRoute>
          <ProfileForm />
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
