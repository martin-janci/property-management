'use client';

/**
 * Realtor profile editor (UC-51.1).
 */

import { ProtectedRoute } from '@/components/auth';
import { Footer, Header } from '@/components/ui';
import {
  RealtorApiError,
  type RealtorProfile,
  getMyRealtorProfile,
  updateMyRealtorProfile,
} from '@/lib/realtor-api';
import { type FormEvent, useEffect, useState } from 'react';

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
          background: #fff; border-radius: 12px; box-shadow: 0 1px 3px rgba(0,0,0,.1);
          display: flex; flex-direction: column; gap: 16px;
        }
        .title { font-size: 1.5rem; font-weight: 700; color: #111827; margin: 0 0 4px; }
        .subtitle { font-size: 14px; color: #6b7280; margin: 0 0 8px; }
        .alert { padding: 12px 16px; background: #fef2f2; color: #b91c1c; border: 1px solid #fecaca; border-radius: 8px; font-size: 14px; }
        .success { padding: 12px 16px; background: #ecfdf5; color: #047857; border: 1px solid #a7f3d0; border-radius: 8px; font-size: 14px; }
        .field { display: flex; flex-direction: column; gap: 6px; }
        .label { font-size: 14px; font-weight: 500; color: #374151; }
        .input {
          padding: 10px 12px; font-size: 15px; border: 1px solid #d1d5db;
          border-radius: 8px; background: #fff; color: #111827; font-family: inherit;
        }
        .input:focus { outline: none; border-color: #2563eb; box-shadow: 0 0 0 3px rgba(37,99,235,.1); }
        .input:disabled { background: #f9fafb; cursor: not-allowed; }
        .hint { color: #6b7280; font-size: 12px; }
        .submit {
          margin-top: 8px; padding: 12px 16px; background: #2563eb; color: #fff;
          border: none; border-radius: 8px; font-size: 16px; font-weight: 600;
          cursor: pointer; align-self: flex-start;
        }
        .submit:hover:not(:disabled) { background: #1d4ed8; }
        .submit:disabled { background: #93c5fd; cursor: not-allowed; }
        .state { padding: 32px; text-align: center; color: #6b7280; }
        .error { color: #dc2626; }
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
        .page { min-height: 100vh; display: flex; flex-direction: column; background: #f9fafb; }
        main { flex: 1; }
      `}</style>
    </div>
  );
}
