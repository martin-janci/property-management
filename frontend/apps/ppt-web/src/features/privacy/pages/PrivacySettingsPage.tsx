/**
 * Privacy Settings Page - GDPR-compliant privacy management.
 * Epic 63: GDPR Screening & Consent Management
 */

import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
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
  const { t, i18n } = useTranslation();
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
        throw new Error(`${t('gdprPrivacy.errors.loadSettings')} (HTTP ${settingsRes.status})`);
      }
      if (!historyRes.ok) {
        throw new Error(`${t('gdprPrivacy.errors.loadHistory')} (HTTP ${historyRes.status})`);
      }

      setSettings(await settingsRes.json());
      setExportRequests(await historyRes.json());
    } catch (err) {
      setError(err instanceof Error ? err.message : t('gdprPrivacy.errors.loadSettings'));
    } finally {
      setLoading(false);
    }
  }, [t]);

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
        throw new Error(t('gdprPrivacy.preferences.saveFailed'));
      }

      setSuccess(t('gdprPrivacy.preferences.saved'));
    } catch (err) {
      setError(err instanceof Error ? err.message : t('gdprPrivacy.preferences.saveFailed'));
    } finally {
      setSaving(false);
    }
  }, [settings, t]);

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
        throw new Error(errorData || t('gdprPrivacy.export.requestFailed'));
      }

      setSuccess(t('gdprPrivacy.export.requested'));
      loadSettings();
    } catch (err) {
      setError(err instanceof Error ? err.message : t('gdprPrivacy.export.requestFailed'));
    }
  }, [loadSettings, t]);

  const handleRequestDeletion = useCallback(async () => {
    // The backend (RequestDataDeletion) requires a `confirmation` string that
    // must match the account email — an empty body always 400s. Ask the user
    // to retype their email as the GDPR-grade confirmation step.
    const confirmation = window.prompt(t('gdprPrivacy.deletion.confirmPrompt'));
    if (!confirmation) {
      return;
    }
    if (user?.email && confirmation.trim().toLowerCase() !== user.email.toLowerCase()) {
      setError(t('gdprPrivacy.deletion.emailMismatch'));
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
        throw new Error(errorData || t('gdprPrivacy.deletion.requestFailed'));
      }

      setSuccess(t('gdprPrivacy.deletion.requested'));
    } catch (err) {
      setError(err instanceof Error ? err.message : t('gdprPrivacy.deletion.requestFailed'));
    }
  }, [user, t]);

  if (loading) {
    return <div className="privacy-loading">{t('gdprPrivacy.loading')}</div>;
  }

  return (
    <div className="privacy-settings-page">
      <header className="privacy-header">
        <h1>{t('gdprPrivacy.title')}</h1>
        <p>{t('gdprPrivacy.subtitle')}</p>
      </header>

      {error && (
        <div className="privacy-error" role="alert">
          {error}
          <button type="button" onClick={() => setError(null)}>
            {t('gdprPrivacy.dismiss')}
          </button>
        </div>
      )}

      {success && (
        <output className="privacy-success">
          {success}
          <button type="button" onClick={() => setSuccess(null)}>
            {t('gdprPrivacy.dismiss')}
          </button>
        </output>
      )}

      {/* Privacy Settings Section */}
      <section className="privacy-section">
        <h2>{t('gdprPrivacy.preferences.title')}</h2>

        {settings && (
          <form
            onSubmit={(e) => {
              e.preventDefault();
              handleSaveSettings();
            }}
          >
            <div className="form-field">
              <label htmlFor="profile-visibility">{t('gdprPrivacy.preferences.visibility')}</label>
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
                <option value="visible">{t('gdprPrivacy.preferences.visible')}</option>
                <option value="contacts_only">{t('gdprPrivacy.preferences.contactsOnly')}</option>
                <option value="hidden">{t('gdprPrivacy.preferences.hidden')}</option>
              </select>
              <p className="field-description">{t('gdprPrivacy.preferences.visibilityHint')}</p>
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
                {t('gdprPrivacy.preferences.showContact')}
              </label>
              <p className="field-description">{t('gdprPrivacy.preferences.showContactHint')}</p>
            </div>

            <button type="submit" className="btn btn-primary" disabled={saving}>
              {saving ? t('gdprPrivacy.preferences.saving') : t('gdprPrivacy.preferences.save')}
            </button>
          </form>
        )}
      </section>

      {/* Data Export Section */}
      <section className="privacy-section">
        <h2>{t('gdprPrivacy.export.title')}</h2>
        <p>{t('gdprPrivacy.export.description')}</p>

        <button type="button" onClick={handleRequestExport} className="btn btn-secondary">
          {t('gdprPrivacy.export.request')}
        </button>

        {exportRequests.length > 0 && (
          <div className="export-history">
            <h3>{t('gdprPrivacy.export.history')}</h3>
            <ul>
              {exportRequests.map((request) => (
                <li key={request.id}>
                  <span className="export-date">
                    {new Date(request.created_at).toLocaleDateString(i18n.language)}
                  </span>
                  <span className={`export-status status-${request.status}`}>
                    {t(`gdprPrivacy.export.status.${request.status}`, {
                      defaultValue: request.status,
                    })}
                  </span>
                  {request.download_url && (
                    <a href={request.download_url} className="download-link">
                      {t('gdprPrivacy.export.download')}
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
        <h2>{t('gdprPrivacy.deletion.title')}</h2>
        <p>{t('gdprPrivacy.deletion.description')}</p>
        <p className="warning">
          <strong>{t('gdprPrivacy.deletion.warningLabel')}</strong>{' '}
          {t('gdprPrivacy.deletion.warning')}
        </p>

        <button type="button" onClick={handleRequestDeletion} className="btn btn-danger">
          {t('gdprPrivacy.deletion.request')}
        </button>
      </section>
    </div>
  );
}

export default PrivacySettingsPage;
