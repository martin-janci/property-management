'use client';

/**
 * Profile edit page (UC-47.7).
 *
 * Calls reality-server `PUT /api/v1/users/me` to update the display name.
 */

import { ProtectedRoute } from '@/components/auth';
import { Footer, Header } from '@/components/ui';
import { AuthApiError, updateProfile } from '@/lib/auth-api';
import { useAuth } from '@/lib/auth-context';
import Link from 'next/link';
import { type FormEvent, useEffect, useState } from 'react';

function ProfileForm() {
  const { user, refreshSession } = useAuth();
  const [displayName, setDisplayName] = useState('');
  const [error, setError] = useState<string>();
  const [savedAt, setSavedAt] = useState<number>();
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    if (user?.name) setDisplayName(user.name);
  }, [user?.name]);

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setError(undefined);
    if (!displayName.trim()) {
      setError('Display name is required');
      return;
    }
    setIsSaving(true);
    try {
      await updateProfile({ name: displayName.trim() });
      await refreshSession();
      setSavedAt(Date.now());
    } catch (err) {
      setError(err instanceof AuthApiError ? err.message : 'Could not save profile.');
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
          background: var(--ppt-bg-surface); border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,.1);
          display: flex; flex-direction: column; gap: 16px;
        }
        .title { font-size: 1.5rem; font-weight: 700; color: var(--ppt-fg-primary); margin: 0 0 4px; }
        .subtitle { font-size: 14px; color: var(--ppt-fg-muted); margin: 0 0 8px; }
        .alert { padding: 12px 16px; background: var(--ppt-color-danger-light); color: var(--ppt-color-danger-dark); border: 1px solid var(--ppt-color-danger-light); border-radius: 8px; font-size: 14px; }
        .success { padding: 12px 16px; background: var(--ppt-color-success-light); color: var(--ppt-color-success-hover); border: 1px solid var(--ppt-color-success-light); border-radius: 8px; font-size: 14px; }
        .field { display: flex; flex-direction: column; gap: 6px; }
        .label { font-size: 14px; font-weight: 500; color: var(--ppt-fg-secondary); }
        .input { padding: 10px 12px; font-size: 16px; border: 1px solid var(--ppt-border-strong); border-radius: 8px; background: var(--ppt-bg-surface); color: var(--ppt-fg-primary); }
        .input:focus { outline: none; border-color: var(--ppt-color-primary); box-shadow: 0 0 0 3px rgba(37,99,235,.1); }
        .input:disabled { background: var(--ppt-bg-app); cursor: not-allowed; }
        .hint { color: var(--ppt-fg-muted); font-size: 12px; }
        .submit { margin-top: 8px; padding: 12px 16px; background: var(--ppt-color-primary); color: var(--ppt-fg-on-accent); border: none; border-radius: 8px; font-size: 16px; font-weight: 600; cursor: pointer; }
        .submit:hover:not(:disabled) { background: var(--ppt-color-primary-hover); }
        .submit:disabled { background: var(--ppt-brand-500); cursor: not-allowed; }
        .meta { text-align: center; margin: 0; }
        .link { color: var(--ppt-color-primary); text-decoration: none; font-weight: 500; font-size: 14px; }
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
        .page { min-height: 100vh; display: flex; flex-direction: column; background: var(--ppt-bg-app); }
        main { flex: 1; }
      `}</style>
    </div>
  );
}
