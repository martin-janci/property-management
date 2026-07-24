/**
 * LayoutManifestsPage — list and upload platform manifests (Task 5, Layout Editor MVP)
 *
 * Lists stored manifests (platform, component count, updated_at, expandable JSON preview)
 * and provides an upload form with platform <select> + <textarea>.
 *
 * Client-side gate before PUT:
 *   1. Body must parse as valid JSON.
 *   2. Parsed object must have a `components` object.
 *   3. If the body contains a `platform` field it must match the selected platform.
 * Violations render inline errors; no request is fired.
 *
 * Hint: copy a manifest from the checked-in sources:
 *   - frontend/apps/ppt-web/src/features/layout/manifest.json  (web)
 *   - frontend/apps/reality-web/src/lib/layout-manifest.json   (mobile)
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';

import { useAdminAuth } from '../../auth/AdminAuthContext';
import { useToast } from '../../components/Toast';
import { listManifests, type Manifest, putManifest } from './api';
import {
  ALERT_STYLE,
  BTN_PRIMARY,
  CARD_STYLE,
  CARD_TITLE_STYLE,
  HINT_STYLE,
  PAGE_STYLE,
  PRE_STYLE,
  SELECT_STYLE,
  TABLE_STYLE,
  TD_STYLE,
  TEXTAREA_STYLE,
  TH_STYLE,
} from './LayoutManifestsPage.styles';

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function LayoutManifestsPage() {
  const { t } = useTranslation();
  const { token } = useAdminAuth();
  const { showToast } = useToast();
  const queryClient = useQueryClient();

  // -------------------------------------------------------------------------
  // List query
  // -------------------------------------------------------------------------

  const manifestsQuery = useQuery({
    queryKey: ['admin', 'platform', 'layout', 'manifests'],
    queryFn: () => listManifests(token),
    staleTime: 60_000,
  });

  const rows = manifestsQuery.data ?? [];

  // -------------------------------------------------------------------------
  // Expandable JSON state (keyed by platform)
  // -------------------------------------------------------------------------

  const [expanded, setExpanded] = useState<Record<string, boolean>>({});

  // -------------------------------------------------------------------------
  // Upload form state
  // -------------------------------------------------------------------------

  const [platform, setPlatform] = useState<'web' | 'mobile'>('web');
  const [manifestText, setManifestText] = useState('');
  const [formError, setFormError] = useState('');

  // -------------------------------------------------------------------------
  // Upload mutation
  // -------------------------------------------------------------------------

  const uploadMutation = useMutation({
    mutationFn: ({ parsedManifest }: { parsedManifest: Manifest }) =>
      putManifest(token, platform, parsedManifest),
    onSuccess: () => {
      showToast({
        type: 'success',
        title: t('admin.manifests.uploaded', { defaultValue: 'Manifest uploaded successfully' }),
      });
      setManifestText('');
      setFormError('');
      queryClient.invalidateQueries({
        queryKey: ['admin', 'platform', 'layout', 'manifests'],
      });
    },
    onError: (err) => {
      showToast({
        type: 'error',
        title: t('admin.manifests.uploadError', { defaultValue: 'Upload failed' }),
        message: err instanceof Error ? err.message : String(err),
      });
    },
  });

  // -------------------------------------------------------------------------
  // Client-side validation + submit
  // -------------------------------------------------------------------------

  function handleUpload() {
    setFormError('');

    // 1. Must parse as valid JSON
    let parsed: unknown;
    try {
      parsed = JSON.parse(manifestText);
    } catch {
      setFormError(
        t('admin.manifests.invalidJson', {
          defaultValue: 'Invalid JSON — please check your input.',
        })
      );
      return;
    }

    // 2. Must be a non-array object with a non-array `components` object
    if (
      typeof parsed !== 'object' ||
      parsed === null ||
      Array.isArray(parsed) ||
      typeof (parsed as Record<string, unknown>).components !== 'object' ||
      (parsed as Record<string, unknown>).components === null ||
      Array.isArray((parsed as Record<string, unknown>).components)
    ) {
      setFormError(
        t('admin.manifests.missingComponents', {
          defaultValue: 'Manifest must have a "components" object.',
        })
      );
      return;
    }

    // 3. If body has a `platform` field it must match the selected platform;
    //    if the field is absent, inject it from the selected platform before PUT.
    const bodyPlatform = (parsed as Record<string, unknown>).platform;
    if (bodyPlatform !== undefined && bodyPlatform !== platform) {
      setFormError(
        t('admin.manifests.platformMismatch', {
          defaultValue: `Platform mismatch: body says "${String(bodyPlatform)}" but selected platform is "${platform}".`,
        })
      );
      return;
    }
    if (bodyPlatform === undefined) {
      (parsed as Record<string, unknown>).platform = platform;
    }

    uploadMutation.mutate({ parsedManifest: parsed as Manifest });
  }

  // -------------------------------------------------------------------------
  // Render
  // -------------------------------------------------------------------------

  return (
    <div style={PAGE_STYLE}>
      <h1 style={{ fontSize: 22, fontWeight: 700, margin: 0 }}>
        {t('admin.manifests.title', { defaultValue: 'Layout Manifests' })}
      </h1>

      {/* ------------------------------------------------------------------- */}
      {/* Stored manifests list                                                 */}
      {/* ------------------------------------------------------------------- */}
      <div style={CARD_STYLE}>
        <h2 style={CARD_TITLE_STYLE}>
          {t('admin.manifests.listTitle', { defaultValue: 'Stored Manifests' })}
        </h2>

        {manifestsQuery.isLoading && (
          <span style={{ fontSize: 13, color: 'var(--ppt-fg-secondary, #6b7280)' }}>
            {t('admin.manifests.loading', { defaultValue: 'Loading…' })}
          </span>
        )}

        {rows.length === 0 && !manifestsQuery.isLoading && (
          <span style={{ fontSize: 13, color: 'var(--ppt-fg-secondary, #6b7280)' }}>
            {t('admin.manifests.empty', { defaultValue: 'No manifests stored yet.' })}
          </span>
        )}

        {rows.length > 0 && (
          <table style={TABLE_STYLE}>
            <thead>
              <tr>
                <th style={TH_STYLE}>
                  {t('admin.manifests.colPlatform', { defaultValue: 'Platform' })}
                </th>
                <th style={TH_STYLE}>
                  {t('admin.manifests.colComponents', { defaultValue: 'Components' })}
                </th>
                <th style={TH_STYLE}>
                  {t('admin.manifests.colUpdatedAt', { defaultValue: 'Updated at' })}
                </th>
                <th style={TH_STYLE} />
              </tr>
            </thead>
            <tbody>
              {rows.map((row) => (
                <tr key={row.platform}>
                  <td style={TD_STYLE}>{row.platform}</td>
                  <td style={TD_STYLE}>{Object.keys(row.manifest?.components ?? {}).length}</td>
                  <td style={TD_STYLE}>{new Date(row.updated_at).toLocaleString()}</td>
                  <td style={TD_STYLE}>
                    <button
                      type="button"
                      style={{
                        fontSize: 12,
                        background: 'none',
                        border: 'none',
                        cursor: 'pointer',
                        color: 'var(--ppt-primary-600, #2563eb)',
                        padding: 0,
                      }}
                      onClick={() =>
                        setExpanded((prev) => ({ ...prev, [row.platform]: !prev[row.platform] }))
                      }
                      data-testid={`expand-btn-${row.platform}`}
                    >
                      {expanded[row.platform]
                        ? t('admin.manifests.collapse', { defaultValue: 'Collapse' })
                        : t('admin.manifests.expand', { defaultValue: 'Expand' })}
                    </button>
                    {expanded[row.platform] && (
                      <pre style={PRE_STYLE}>{JSON.stringify(row.manifest, null, 2)}</pre>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {/* ------------------------------------------------------------------- */}
      {/* Upload form                                                           */}
      {/* ------------------------------------------------------------------- */}
      <div style={CARD_STYLE}>
        <h2 style={CARD_TITLE_STYLE}>
          {t('admin.manifests.uploadTitle', { defaultValue: 'Upload Manifest' })}
        </h2>

        {/* Hint line */}
        <p style={HINT_STYLE}>
          {t('admin.manifests.hint', {
            defaultValue:
              'Copy from checked-in manifests: ' +
              'frontend/apps/ppt-web/src/features/layout/manifest.json (web) · ' +
              'frontend/apps/reality-web/src/lib/layout-manifest.json (mobile)',
          })}
        </p>

        {/* Platform select */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          <label htmlFor="manifest-platform-select" style={{ fontSize: 13, fontWeight: 500 }}>
            {t('admin.manifests.platformLabel', { defaultValue: 'Platform' })}
          </label>
          <select
            id="manifest-platform-select"
            aria-label={t('admin.manifests.platformLabel', { defaultValue: 'Platform' })}
            value={platform}
            onChange={(e) => {
              setPlatform(e.target.value as 'web' | 'mobile');
              setFormError('');
            }}
            style={SELECT_STYLE}
          >
            <option value="web">web</option>
            <option value="mobile">mobile</option>
          </select>
        </div>

        {/* Manifest JSON textarea */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          <label htmlFor="manifest-json-textarea" style={{ fontSize: 13, fontWeight: 500 }}>
            {t('admin.manifests.jsonLabel', { defaultValue: 'Manifest JSON' })}
          </label>
          <textarea
            id="manifest-json-textarea"
            aria-label={t('admin.manifests.jsonLabel', { defaultValue: 'Manifest JSON' })}
            value={manifestText}
            onChange={(e) => {
              setManifestText(e.target.value);
              setFormError('');
            }}
            style={TEXTAREA_STYLE}
            placeholder='{"platform": "web", "components": { ... }}'
            data-testid="manifest-json-textarea"
          />
        </div>

        {/* Inline validation error */}
        {formError && (
          <div style={ALERT_STYLE} role="alert" data-testid="manifest-form-error">
            {formError}
          </div>
        )}

        <div>
          <button
            type="button"
            style={{
              ...BTN_PRIMARY,
              opacity: uploadMutation.isPending ? 0.6 : 1,
              cursor: uploadMutation.isPending ? 'not-allowed' : 'pointer',
            }}
            disabled={uploadMutation.isPending || !manifestText.trim()}
            onClick={handleUpload}
            data-testid="manifest-upload-btn"
          >
            {uploadMutation.isPending
              ? t('admin.manifests.uploading', { defaultValue: 'Uploading…' })
              : t('admin.manifests.uploadBtn', { defaultValue: 'Upload' })}
          </button>
        </div>
      </div>
    </div>
  );
}
