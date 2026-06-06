/**
 * Privacy Settings Page - GDPR-compliant privacy management.
 * Epic 63: GDPR Screening & Consent Management
 */

import { useCallback, useEffect, useState } from 'react';
import { useAuth } from '../../../contexts';
import { gdprFetch } from '../gdprClient';

interface PrivacySettings {
  profile_visibility: 'visible' | 'hidden' | 'contacts_only';
  show_contact_info: boolean;
}

interface DataExportRequest {
  id: string;
  status: 'pending' | 'processing' | 'completed' | 'failed';
  created_at: string;
  completed_at?: string;
  download_url?: string;
}

export function PrivacySettingsPage() {
  const { user } = useAuth();
  const [settings, setSettings] = useState<PrivacySettings | null>(null);
  const [exportRequests, setExportRequests] = useState<DataExportRequest[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  const loadSettings = useCallback(async () => {
    setLoading(true);
    setError(null);

    try {
      const [settingsRes, historyRes] = await Promise.all([
        gdprFetch('/api/v1/gdpr/privacy'),
        gdprFetch('/api/v1/gdpr/export/history'),
      ]);

      if (!settingsRes.ok) {
        throw new Error(`Failed to load privacy settings (status ${settingsRes.status})`);
      }
      if (!historyRes.ok) {
        throw new Error(`Failed to load export history (status ${historyRes.status})`);
      }

      setSettings(await settingsRes.json());
      setExportRequests(await historyRes.json());
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load privacy settings');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadSettings();
  }, [loadSettings]);

  const handleSaveSettings = useCallback(async () => {
    if (!settings) return;

    setSaving(true);
    setError(null);
    setSuccess(null);

    try {
      // Backend (UpdatePrivacySettingsRequest) only persists these two fields;
      // sending the full `settings` object silently drops marketing/analytics
      // consent. Send only the supported fields so the request is honest.
      const response = await gdprFetch('/api/v1/gdpr/privacy', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          profile_visibility: settings.profile_visibility,
          show_contact_info: settings.show_contact_info,
        }),
      });

      if (!response.ok) {
        throw new Error('Failed to save settings');
      }

      setSuccess('Privacy settings saved successfully');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save settings');
    } finally {
      setSaving(false);
    }
  }, [settings]);

  const handleRequestExport = useCallback(async () => {
    setError(null);

    try {
      const response = await gdprFetch('/api/v1/gdpr/export/request', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ format: 'json' }),
      });

      if (!response.ok) {
        const errorData = await response.text();
        throw new Error(errorData || 'Failed to request data export');
      }

      setSuccess('Data export requested. You will be notified when it is ready.');
      loadSettings();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to request export');
    }
  }, [loadSettings]);

  const handleRequestDeletion = useCallback(async () => {
    // The backend (RequestDataDeletion) requires a `confirmation` string that
    // must match the account email — an empty body always 400s. Ask the user
    // to retype their email as the GDPR-grade confirmation step.
    const confirmation = window.prompt(
      'This will schedule your account and all associated data for permanent ' +
        'deletion after a 30-day grace period. This cannot be undone after the ' +
        'grace period.\n\nType your account email address to confirm:'
    );
    if (!confirmation) {
      return;
    }
    if (user?.email && confirmation.trim().toLowerCase() !== user.email.toLowerCase()) {
      setError('The email you entered does not match your account email.');
      return;
    }

    setError(null);

    try {
      const response = await gdprFetch('/api/v1/gdpr/deletion/request', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ confirmation: confirmation.trim() }),
      });

      if (!response.ok) {
        const errorData = await response.text();
        throw new Error(errorData || 'Failed to request account deletion');
      }

      setSuccess('Account deletion requested. You have 30 days to cancel this request.');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to request deletion');
    }
  }, [user]);

  if (loading) {
    return <div className="privacy-loading">Loading privacy settings...</div>;
  }

  return (
    <div className="privacy-settings-page">
      <header className="privacy-header">
        <h1>Privacy & Data Settings</h1>
        <p>Manage your privacy preferences and data in accordance with GDPR.</p>
      </header>

      {error && (
        <div className="privacy-error" role="alert">
          {error}
          <button type="button" onClick={() => setError(null)}>
            Dismiss
          </button>
        </div>
      )}

      {success && (
        <output className="privacy-success">
          {success}
          <button type="button" onClick={() => setSuccess(null)}>
            Dismiss
          </button>
        </output>
      )}

      {/* Privacy Settings Section */}
      <section className="privacy-section">
        <h2>Privacy Preferences</h2>

        {settings && (
          <form
            onSubmit={(e) => {
              e.preventDefault();
              handleSaveSettings();
            }}
          >
            <div className="form-field">
              <label htmlFor="profile-visibility">Profile Visibility</label>
              <select
                id="profile-visibility"
                value={settings.profile_visibility}
                onChange={(e) =>
                  setSettings({
                    ...settings,
                    profile_visibility: e.target.value as PrivacySettings['profile_visibility'],
                  })
                }
              >
                <option value="visible">Visible to all residents</option>
                <option value="contacts_only">Visible to contacts only</option>
                <option value="hidden">Hidden from everyone</option>
              </select>
              <p className="field-description">
                Controls who can see your profile in the resident directory.
              </p>
            </div>

            <div className="form-field">
              <label className="checkbox-label">
                <input
                  type="checkbox"
                  checked={settings.show_contact_info}
                  onChange={(e) =>
                    setSettings({
                      ...settings,
                      show_contact_info: e.target.checked,
                    })
                  }
                />
                Show contact information on profile
              </label>
              <p className="field-description">
                When enabled, your email and phone will be visible on your profile.
              </p>
            </div>

            <button type="submit" className="btn btn-primary" disabled={saving}>
              {saving ? 'Saving...' : 'Save Preferences'}
            </button>
          </form>
        )}
      </section>

      {/* Data Export Section */}
      <section className="privacy-section">
        <h2>Your Data</h2>
        <p>
          Under GDPR Article 15, you have the right to access your personal data. Request a copy of
          all data we hold about you.
        </p>

        <button type="button" onClick={handleRequestExport} className="btn btn-secondary">
          Request Data Export
        </button>

        {exportRequests.length > 0 && (
          <div className="export-history">
            <h3>Export History</h3>
            <ul>
              {exportRequests.map((request) => (
                <li key={request.id}>
                  <span className="export-date">
                    {new Date(request.created_at).toLocaleDateString()}
                  </span>
                  <span className={`export-status status-${request.status}`}>{request.status}</span>
                  {request.download_url && (
                    <a href={request.download_url} className="download-link">
                      Download
                    </a>
                  )}
                </li>
              ))}
            </ul>
          </div>
        )}
      </section>

      {/* Account Deletion Section */}
      <section className="privacy-section privacy-danger">
        <h2>Delete Account</h2>
        <p>
          Under GDPR Article 17, you have the right to erasure ("right to be forgotten"). Requesting
          deletion will schedule your account and all associated data for permanent removal after a
          30-day grace period.
        </p>
        <p className="warning">
          <strong>Warning:</strong> This action cannot be undone after the grace period. You will
          lose access to all your data, messages, and account history.
        </p>

        <button type="button" onClick={handleRequestDeletion} className="btn btn-danger">
          Request Account Deletion
        </button>
      </section>
    </div>
  );
}

export default PrivacySettingsPage;
