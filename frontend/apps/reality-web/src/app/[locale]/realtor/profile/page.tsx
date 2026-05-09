'use client';

/**
 * Realtor profile editor (UC-51.1).
 */

import { type FormEvent, useEffect, useState } from 'react';
import { ProtectedRoute } from '@/components/auth';
import { Footer, Header } from '@/components/ui';
import {
  getMyRealtorProfile,
  RealtorApiError,
  type RealtorProfile,
  updateMyRealtorProfile,
} from '@/lib/realtor-api';

function ProfileFormContent() {
  const [profile, setProfile] = useState<RealtorProfile | null>(null);
  const [name, setName] = useState('');
  const [phone, setPhone] = useState('');
  const [title, setTitle] = useState('');
  const [bio, setBio] = useState('');
  const [licenseNumber, setLicenseNumber] = useState('');
  const [specializationsText, setSpecializationsText] = useState('');

  const [isLoading, setIsLoading] = useState(true);
  const [loadError, setLoadError] = useState<string>();
  const [isSaving, setIsSaving] = useState(false);
  const [savedAt, setSavedAt] = useState<number>();
  const [saveError, setSaveError] = useState<string>();

  useEffect(() => {
    let cancelled = false;
    setIsLoading(true);
    getMyRealtorProfile()
      .then((value) => {
        if (cancelled) return;
        setProfile(value);
        setName(value.name);
        setPhone(value.phone ?? '');
        setTitle(value.title ?? '');
        setBio(value.bio ?? '');
        setLicenseNumber(value.licenseNumber ?? '');
        setSpecializationsText(value.specializations.join(', '));
      })
      .catch((err) => {
        if (cancelled) return;
        setLoadError(err instanceof RealtorApiError ? err.message : 'Could not load profile.');
      })
      .finally(() => {
        if (!cancelled) setIsLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const handleSubmit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setSaveError(undefined);
    setSavedAt(undefined);
    if (!name.trim()) {
      setSaveError('Name is required');
      return;
    }
    setIsSaving(true);
    try {
      const updated = await updateMyRealtorProfile({
        name: name.trim(),
        phone: phone.trim() || undefined,
        title: title.trim() || undefined,
        bio: bio.trim() || undefined,
        licenseNumber: licenseNumber.trim() || undefined,
        specializations: specializationsText
          .split(',')
          .map((s) => s.trim())
          .filter(Boolean),
      });
      setProfile(updated);
      setSavedAt(Date.now());
    } catch (err) {
      setSaveError(err instanceof RealtorApiError ? err.message : 'Could not save profile.');
    } finally {
      setIsSaving(false);
    }
  };

  if (isLoading) return <p className="state">Loading profile…</p>;
  if (loadError) return <p className="state error">{loadError}</p>;

  return (
    <form className="form" onSubmit={handleSubmit} noValidate>
      <h1 className="title">Realtor profile</h1>
      <p className="subtitle">Buyers and renters see this information on your listings.</p>

      {saveError && (
        <div className="alert" role="alert">
          {saveError}
        </div>
      )}
      {savedAt && (
        <div className="success" role="status">
          Profile updated.
        </div>
      )}

      <label className="field">
        <span className="label">Name</span>
        <input
          type="text"
          autoComplete="name"
          value={name}
          onChange={(e) => setName(e.target.value)}
          disabled={isSaving}
          className="input"
        />
      </label>

      <label className="field">
        <span className="label">Email</span>
        <input type="email" value={profile?.email ?? ''} disabled className="input" />
        <span className="hint">Contact support to change the email on this account.</span>
      </label>

      <label className="field">
        <span className="label">Phone</span>
        <input
          type="tel"
          autoComplete="tel"
          value={phone}
          onChange={(e) => setPhone(e.target.value)}
          disabled={isSaving}
          className="input"
        />
      </label>

      <label className="field">
        <span className="label">Title</span>
        <input
          type="text"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          disabled={isSaving}
          className="input"
        />
      </label>

      <label className="field">
        <span className="label">Bio</span>
        <textarea
          value={bio}
          onChange={(e) => setBio(e.target.value)}
          disabled={isSaving}
          rows={5}
          className="input"
        />
      </label>

      <label className="field">
        <span className="label">License number</span>
        <input
          type="text"
          value={licenseNumber}
          onChange={(e) => setLicenseNumber(e.target.value)}
          disabled={isSaving}
          className="input"
        />
      </label>

      <label className="field">
        <span className="label">Specializations</span>
        <input
          type="text"
          value={specializationsText}
          onChange={(e) => setSpecializationsText(e.target.value)}
          disabled={isSaving}
          className="input"
        />
        <span className="hint">Comma-separated list, e.g. Apartments, Commercial.</span>
      </label>

      <button type="submit" className="submit" disabled={isSaving}>
        {isSaving ? 'Saving…' : 'Save changes'}
      </button>

      <style jsx>{`
        .form {
          max-width: 640px; margin: 32px auto; padding: 32px;
          background: var(--ppt-bg-surface); border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,.1);
          display: flex; flex-direction: column; gap: 16px;
        }
        .title { font-size: 1.5rem; font-weight: 700; color: var(--ppt-fg-primary); margin: 0 0 4px; }
        .subtitle { font-size: 14px; color: var(--ppt-fg-muted); margin: 0 0 8px; }
        .alert { padding: 12px 16px; background: var(--ppt-color-danger-light); color: var(--ppt-color-danger-dark); border: 1px solid var(--ppt-color-danger); border-radius: 8px; font-size: 14px; }
        .success { padding: 12px 16px; background: var(--ppt-color-success-light); color: var(--ppt-color-success-dark); border: 1px solid var(--ppt-color-success); border-radius: 8px; font-size: 14px; }
        .field { display: flex; flex-direction: column; gap: 6px; }
        .label { font-size: 14px; font-weight: 500; color: var(--ppt-fg-secondary); }
        .input {
          padding: 10px 12px; font-size: 15px; border: 1px solid var(--ppt-border-strong);
          border-radius: 8px; background: var(--ppt-bg-surface); color: var(--ppt-fg-primary); font-family: inherit;
        }
        .input:focus { outline: none; border-color: var(--ppt-color-primary); box-shadow: 0 0 0 3px rgba(37,99,235,.1); }
        .input:disabled { background: var(--ppt-bg-app); cursor: not-allowed; }
        .hint { color: var(--ppt-fg-muted); font-size: 12px; }
        .submit {
          margin-top: 8px; padding: 12px 16px; background: var(--ppt-color-primary); color: var(--ppt-fg-on-accent);
          border: none; border-radius: 8px; font-size: 16px; font-weight: 600;
          cursor: pointer; align-self: flex-start;
        }
        .submit:hover:not(:disabled) { background: var(--ppt-color-primary-hover); }
        .submit:disabled { background: var(--ppt-brand-500); cursor: not-allowed; }
        .state { padding: 32px; text-align: center; color: var(--ppt-fg-muted); }
        .error { color: var(--ppt-color-danger-hover); }
      `}</style>
    </form>
  );
}

export default function RealtorProfilePage() {
  return (
    <div className="page">
      <Header />
      <main>
        <ProtectedRoute>
          <ProfileFormContent />
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
