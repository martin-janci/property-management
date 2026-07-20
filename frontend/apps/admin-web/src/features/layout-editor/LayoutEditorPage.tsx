/**
 * LayoutEditorPage — wiring page (Task 4, Layout Editor MVP)
 *
 * Composes SectionTreeEditor + RailsEditor with TanStack Query mutations.
 * Auth token from useAdminAuth(); toasts via useToast(); i18n via t().
 */

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import type React from 'react';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { useAdminAuth } from '../../auth/AdminAuthContext';
import { useToast } from '../../components/Toast';
import {
  getConfig,
  kill,
  LayoutApiError,
  listManifests,
  listScreens,
  type Manifest,
  publish,
  putDraft,
  putRails,
  type Rails,
  rollback,
  type ScreenConfig,
  unkill,
} from './api';
import {
  ALERT_STYLE,
  BTN_BASE,
  BTN_DANGER,
  BTN_PRIMARY,
  BTN_SECONDARY,
  CARD_STYLE,
  CARD_TITLE_STYLE,
  INPUT_STYLE,
  PAGE_STYLE,
  ROW_STYLE,
  SELECT_STYLE,
  TABLE_STYLE,
  TD_STYLE,
  TH_STYLE,
  TOGGLE_STYLE,
} from './LayoutEditorPage.styles';
import { PreviewPanel } from './PreviewPanel';
import { RailsEditor } from './RailsEditor';
import { SectionTreeEditor } from './SectionTreeEditor';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const SCREEN_PATTERN = /^[a-z0-9-]+\/[a-z0-9-]+$/;

const EMPTY_RAILS: Rails = {
  hideable: [],
  mode_editable: [],
  reorderable: false,
  prop_whitelist: {},
};

// ---------------------------------------------------------------------------
// Deep clone helper
// ---------------------------------------------------------------------------

function deepClone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function LayoutEditorPage() {
  const { t } = useTranslation();
  const { token } = useAdminAuth();
  const { showToast } = useToast();
  const queryClient = useQueryClient();

  // -------------------------------------------------------------------------
  // Screen selection state
  // -------------------------------------------------------------------------

  const [selectedScreen, setSelectedScreen] = useState<string>('');
  const [newScreenInput, setNewScreenInput] = useState('');
  const [newScreenError, setNewScreenError] = useState('');

  // Platform toggle: which manifest to feed the tree
  const [platform, setPlatform] = useState<'web' | 'mobile'>('web');

  // Highlighted section — set via PreviewPanel section-click
  const [highlightedSection, setHighlightedSection] = useState<string | null>(null);

  // -------------------------------------------------------------------------
  // Publish error list (persistent, 422 only)
  // -------------------------------------------------------------------------

  const [publishErrors, setPublishErrors] = useState<string[]>([]);

  // -------------------------------------------------------------------------
  // Queries
  // -------------------------------------------------------------------------

  const screensQuery = useQuery({
    queryKey: ['admin', 'platform', 'layout', 'screens'],
    queryFn: () => listScreens(token),
    staleTime: 30_000,
    enabled: Boolean(token),
  });

  const configQuery = useQuery({
    queryKey: ['admin', 'platform', 'layout', 'config', selectedScreen],
    queryFn: async () => {
      try {
        return await getConfig(token, selectedScreen);
      } catch (err) {
        if (err instanceof LayoutApiError && err.status === 404) {
          // New screen — return an empty envelope
          return {
            config: {
              screen: selectedScreen,
              draft: { screen: selectedScreen, version: 0, sections: [] },
              published: null,
              published_version: 0,
              rails: EMPTY_RAILS,
            },
            versions: [],
            kills: [],
          };
        }
        throw err;
      }
    },
    enabled: Boolean(selectedScreen) && Boolean(token),
    staleTime: 0,
  });

  const manifestsQuery = useQuery({
    queryKey: ['admin', 'platform', 'layout', 'manifests'],
    queryFn: () => listManifests(token),
    staleTime: 60_000,
    enabled: Boolean(token),
  });

  // -------------------------------------------------------------------------
  // Active manifest for the current platform
  // -------------------------------------------------------------------------

  const activeManifest: Manifest | null = useMemo(() => {
    const rows = manifestsQuery.data ?? [];
    return rows.find((r) => r.platform === platform)?.manifest ?? null;
  }, [manifestsQuery.data, platform]);

  // -------------------------------------------------------------------------
  // Local draft + rails state (deep-cloned from envelope on explicit reseed)
  // -------------------------------------------------------------------------

  const [localDraft, setLocalDraft] = useState<ScreenConfig | null>(null);
  const [localRails, setLocalRails] = useState<Rails>(EMPTY_RAILS);
  const [draftDirty, setDraftDirty] = useState(false);
  const [railsDirty, setRailsDirty] = useState(false);

  // Reseed epoch — bump this to force a reseed from the server (e.g. after rollback).
  // Invalidation alone (publish, save) does NOT bump epoch; only rollback does.
  const [reseedEpoch, setReseedEpoch] = useState(0);

  // Track what was last seeded so we only reseed on screen change or explicit epoch bump.
  const lastSeededRef = useRef<{ screen: string; epoch: number } | null>(null);

  const envelope = configQuery.data;

  useEffect(() => {
    if (!envelope) return;

    const shouldSeed =
      lastSeededRef.current === null ||
      lastSeededRef.current.screen !== selectedScreen ||
      lastSeededRef.current.epoch !== reseedEpoch;

    if (!shouldSeed) return;

    lastSeededRef.current = { screen: selectedScreen, epoch: reseedEpoch };

    setLocalDraft(deepClone(envelope.config.draft));
    const rails = envelope.config.rails;
    const hasRails =
      rails &&
      (Array.isArray((rails as Rails).hideable) ||
        typeof (rails as Rails).reorderable === 'boolean');
    setLocalRails(deepClone(hasRails ? (rails as Rails) : EMPTY_RAILS));
    setDraftDirty(false);
    setRailsDirty(false);
    setPublishErrors([]);
  }, [envelope, selectedScreen, reseedEpoch]);

  // -------------------------------------------------------------------------
  // Mutations
  // -------------------------------------------------------------------------

  // Track the JSON payload sent in saveDraftMutation so onSuccess can compare
  // against current local state — only clear dirty if no new edits arrived
  // during the round-trip.
  const sentDraftJsonRef = useRef<string | null>(null);

  const saveDraftMutation = useMutation({
    mutationFn: () => {
      if (!localDraft) throw new Error('no draft');
      sentDraftJsonRef.current = JSON.stringify(localDraft);
      return putDraft(token, selectedScreen, localDraft);
    },
    onSuccess: () => {
      showToast({
        type: 'success',
        title: t('admin.layout.draftSaved', { defaultValue: 'Draft saved' }),
      });
      // Only clear dirty if the user hasn't made new edits since mutate() fired.
      if (sentDraftJsonRef.current === JSON.stringify(localDraft)) {
        setDraftDirty(false);
      }
      // Invalidate to sync server version number — but do NOT reseed (local IS
      // what was saved; reseeding would clobber any edits made during round-trip).
      queryClient.invalidateQueries({
        queryKey: ['admin', 'platform', 'layout', 'config', selectedScreen],
      });
    },
    onError: (err) => {
      showToast({
        type: 'error',
        title: t('admin.layout.draftSaveError', { defaultValue: 'Failed to save draft' }),
        message: err instanceof Error ? err.message : String(err),
      });
    },
  });

  const sentRailsJsonRef = useRef<string | null>(null);

  const saveRailsMutation = useMutation({
    mutationFn: () => {
      sentRailsJsonRef.current = JSON.stringify(localRails);
      return putRails(token, selectedScreen, localRails);
    },
    onSuccess: () => {
      showToast({
        type: 'success',
        title: t('admin.layout.railsSaved', { defaultValue: 'Rails saved' }),
      });
      // Only clear dirty if no new edits arrived during the round-trip.
      if (sentRailsJsonRef.current === JSON.stringify(localRails)) {
        setRailsDirty(false);
      }
      // Invalidate only — no reseed (local state IS what was saved).
      queryClient.invalidateQueries({
        queryKey: ['admin', 'platform', 'layout', 'config', selectedScreen],
      });
    },
    onError: (err) => {
      showToast({
        type: 'error',
        title: t('admin.layout.railsSaveError', { defaultValue: 'Failed to save rails' }),
        message: err instanceof Error ? err.message : String(err),
      });
    },
  });

  const publishMutation = useMutation({
    mutationFn: () => publish(token, selectedScreen),
    onSuccess: () => {
      setPublishErrors([]);
      showToast({
        type: 'success',
        title: t('admin.layout.published', { defaultValue: 'Published successfully' }),
      });
      queryClient.invalidateQueries({
        queryKey: ['admin', 'platform', 'layout', 'config', selectedScreen],
      });
    },
    onError: (err) => {
      if (err instanceof LayoutApiError && err.status === 422) {
        // Render verbatim — NOT just a toast
        setPublishErrors(err.errors);
      } else {
        showToast({
          type: 'error',
          title: t('admin.layout.publishError', { defaultValue: 'Publish failed' }),
          message: err instanceof Error ? err.message : String(err),
        });
      }
    },
  });

  const rollbackMutation = useMutation({
    mutationFn: (version: number) => rollback(token, selectedScreen, version),
    onSuccess: () => {
      showToast({
        type: 'success',
        title: t('admin.layout.rolledBack', { defaultValue: 'Rolled back' }),
      });
      // Invalidate AND bump reseedEpoch — server is now the truth, discard
      // any in-flight local edits and reseed from the refetched envelope.
      queryClient.invalidateQueries({
        queryKey: ['admin', 'platform', 'layout', 'config', selectedScreen],
      });
      setReseedEpoch((e) => e + 1);
    },
    onError: (err) => {
      showToast({
        type: 'error',
        title: t('admin.layout.rollbackError', { defaultValue: 'Rollback failed' }),
        message: err instanceof Error ? err.message : String(err),
      });
    },
  });

  const killMutation = useMutation({
    mutationFn: (sectionType: string) => kill(token, selectedScreen, sectionType),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ['admin', 'platform', 'layout', 'config', selectedScreen],
      });
    },
    onError: (err) => {
      showToast({
        type: 'error',
        title: t('admin.layout.killError', { defaultValue: 'Kill failed' }),
        message: err instanceof Error ? err.message : String(err),
      });
    },
  });

  const unkillMutation = useMutation({
    mutationFn: (sectionType: string) => unkill(token, selectedScreen, sectionType),
    onSuccess: () => {
      queryClient.invalidateQueries({
        queryKey: ['admin', 'platform', 'layout', 'config', selectedScreen],
      });
    },
    onError: (err) => {
      showToast({
        type: 'error',
        title: t('admin.layout.unkillError', { defaultValue: 'Unkill failed' }),
        message: err instanceof Error ? err.message : String(err),
      });
    },
  });

  // -------------------------------------------------------------------------
  // Handlers
  // -------------------------------------------------------------------------

  const handleScreenChange = useCallback(
    (e: React.ChangeEvent<HTMLSelectElement>) => {
      if (
        (draftDirty || railsDirty) &&
        !window.confirm(
          t('admin.layout.discardConfirm', { defaultValue: 'Discard unsaved changes?' })
        )
      ) {
        return;
      }
      setSelectedScreen(e.target.value);
      setNewScreenInput('');
      setNewScreenError('');
      setPublishErrors([]);
    },
    [draftDirty, railsDirty, t]
  );

  const handleNewScreenSubmit = useCallback(() => {
    if (
      (draftDirty || railsDirty) &&
      !window.confirm(
        t('admin.layout.discardConfirm', { defaultValue: 'Discard unsaved changes?' })
      )
    ) {
      return;
    }
    const val = newScreenInput.trim();
    if (!SCREEN_PATTERN.test(val)) {
      setNewScreenError(
        t('admin.layout.newScreenInvalid', {
          defaultValue: 'Format must match pattern: namespace/name (lowercase, digits, hyphens)',
        })
      );
      return;
    }
    setNewScreenError('');
    setSelectedScreen(val);
    setNewScreenInput('');
    setLocalDraft({ screen: val, version: 0, sections: [] });
    setLocalRails(EMPTY_RAILS);
    setDraftDirty(false);
    setRailsDirty(false);
    setPublishErrors([]);
  }, [draftDirty, railsDirty, newScreenInput, t]);

  const handleDraftChange = useCallback((next: ScreenConfig['sections'][0][]) => {
    setLocalDraft((prev) => (prev ? { ...prev, sections: next } : prev));
    setDraftDirty(true);
  }, []);

  const handleRailsChange = useCallback((next: Rails) => {
    setLocalRails(next);
    setRailsDirty(true);
  }, []);

  const handleRollback = useCallback(
    (version: number) => {
      if (
        window.confirm(
          t('admin.layout.rollbackConfirm', {
            defaultValue: `Roll back to version ${version}?`,
          })
        )
      ) {
        rollbackMutation.mutate(version);
      }
    },
    [rollbackMutation, t]
  );

  const handleKill = useCallback(
    (sectionType: string) => {
      killMutation.mutate(sectionType);
    },
    [killMutation]
  );

  const handleUnkill = useCallback(
    (sectionType: string) => {
      unkillMutation.mutate(sectionType);
    },
    [unkillMutation]
  );

  // -------------------------------------------------------------------------
  // Derived values
  // -------------------------------------------------------------------------

  const screens = screensQuery.data ?? [];
  const kills = envelope?.kills.map((k) => k.section_type) ?? [];
  const versions = envelope?.versions ?? [];
  const publishedVersion = envelope?.config.published_version ?? 0;
  const hasPublished = Boolean(envelope?.config.published);

  const sectionTypes = useMemo(
    () =>
      activeManifest
        ? Object.keys(activeManifest.components)
        : (localDraft?.sections.map((s) => s.type) ?? []),
    [activeManifest, localDraft]
  );

  // -------------------------------------------------------------------------
  // Render
  // -------------------------------------------------------------------------

  return (
    <div style={PAGE_STYLE}>
      <h1 style={{ fontSize: 22, fontWeight: 700, margin: 0 }}>
        {t('admin.layout.title', { defaultValue: 'Layout Editor' })}
      </h1>

      {/* ----------------------------------------------------------------- */}
      {/* Screen selector card                                                */}
      {/* ----------------------------------------------------------------- */}
      <div style={CARD_STYLE}>
        <h2 style={CARD_TITLE_STYLE}>
          {t('admin.layout.screenSelector', { defaultValue: 'Screen' })}
        </h2>

        <div style={ROW_STYLE}>
          {/* Existing screen select */}
          <label htmlFor="screen-select" style={{ fontSize: 13, fontWeight: 500 }}>
            {t('admin.layout.selectScreen', { defaultValue: 'Screen' })}
          </label>
          <select
            id="screen-select"
            aria-label={t('admin.layout.selectScreen', { defaultValue: 'Screen' })}
            value={selectedScreen}
            onChange={handleScreenChange}
            style={SELECT_STYLE}
          >
            <option value="">
              {t('admin.layout.selectScreenPlaceholder', { defaultValue: '— select a screen —' })}
            </option>
            {screens.map((s) => (
              <option key={s.screen} value={s.screen}>
                {s.screen}
              </option>
            ))}
          </select>

          {screensQuery.isLoading && (
            <span style={{ fontSize: 12, color: 'var(--ppt-fg-secondary, #6b7280)' }}>
              {t('admin.layout.loading', { defaultValue: 'Loading…' })}
            </span>
          )}
        </div>

        {/* New screen input */}
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          <label style={{ fontSize: 13, fontWeight: 500 }}>
            {t('admin.layout.newScreen', { defaultValue: 'Or create a new screen' })}
          </label>
          <div style={ROW_STYLE}>
            <input
              type="text"
              aria-label={t('admin.layout.newScreenInput', { defaultValue: 'New screen name' })}
              placeholder="namespace/screen-name"
              value={newScreenInput}
              onChange={(e) => {
                setNewScreenInput(e.target.value);
                setNewScreenError('');
              }}
              style={INPUT_STYLE}
              data-testid="new-screen-input"
            />
            <button
              type="button"
              style={BTN_SECONDARY}
              onClick={handleNewScreenSubmit}
              disabled={!newScreenInput}
              data-testid="new-screen-btn"
            >
              {t('admin.layout.newScreenBtn', { defaultValue: 'Create' })}
            </button>
          </div>
          {newScreenError && (
            <span
              style={{ fontSize: 12, color: 'var(--ppt-danger-600, #dc2626)' }}
              role="alert"
              data-testid="new-screen-error"
            >
              {newScreenError}
            </span>
          )}
        </div>

        {/* Platform toggle */}
        {selectedScreen && (
          <div style={ROW_STYLE}>
            <span style={{ fontSize: 13, fontWeight: 500 }}>
              {t('admin.layout.platform', { defaultValue: 'Platform' })}
            </span>
            <div style={TOGGLE_STYLE}>
              {(['web', 'mobile'] as const).map((p) => (
                <button
                  key={p}
                  type="button"
                  data-testid={`platform-${p}`}
                  onClick={() => setPlatform(p)}
                  style={{
                    ...BTN_BASE,
                    borderRadius: 0,
                    border: 'none',
                    background:
                      platform === p
                        ? 'var(--ppt-primary-600, #2563eb)'
                        : 'var(--ppt-bg-surface, #fff)',
                    color: platform === p ? '#fff' : 'var(--ppt-fg-primary, #111827)',
                  }}
                >
                  {p}
                </button>
              ))}
            </div>
          </div>
        )}
      </div>

      {/* Only show remaining cards when a screen is selected and config loaded */}
      {selectedScreen &&
        configQuery.isError &&
        !(
          configQuery.error instanceof LayoutApiError &&
          (configQuery.error as LayoutApiError).status === 404
        ) && (
          <div style={ALERT_STYLE} role="alert">
            {t('admin.layout.configLoadError', { defaultValue: 'Failed to load config' })}
          </div>
        )}

      {selectedScreen && localDraft && (
        <>
          {/* Published state summary */}
          <div style={{ fontSize: 13, color: 'var(--ppt-fg-secondary, #6b7280)' }}>
            {t('admin.layout.publishedState', {
              defaultValue: `Published version: ${publishedVersion} | Published: ${hasPublished ? 'yes' : 'no'}`,
            })}
          </div>

          {/* --------------------------------------------------------------- */}
          {/* Sections card                                                     */}
          {/* --------------------------------------------------------------- */}
          <div style={CARD_STYLE}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <h2 style={CARD_TITLE_STYLE}>
                {t('admin.layout.sectionsTitle', { defaultValue: 'Sections' })}
              </h2>
              <button
                type="button"
                style={{
                  ...BTN_PRIMARY,
                  opacity: !draftDirty || saveDraftMutation.isPending ? 0.6 : 1,
                  cursor: !draftDirty || saveDraftMutation.isPending ? 'not-allowed' : 'pointer',
                }}
                disabled={!draftDirty || saveDraftMutation.isPending}
                onClick={() => saveDraftMutation.mutate()}
                data-testid="save-draft-btn"
              >
                {saveDraftMutation.isPending
                  ? t('admin.layout.saving', { defaultValue: 'Saving…' })
                  : t('admin.layout.saveDraft', { defaultValue: 'Save Draft' })}
              </button>
            </div>

            {/* Key on screen + reseedEpoch: a rollback (epoch bump) or screen
                switch remounts the editor, clearing any in-flight editing slot
                so a focused textarea's onBlur can't commit stale text onto the
                reseeded data. */}
            <SectionTreeEditor
              key={`${selectedScreen}:${reseedEpoch}`}
              sections={localDraft.sections}
              manifest={activeManifest}
              kills={kills}
              onChange={handleDraftChange}
              onKill={handleKill}
              onUnkill={handleUnkill}
              highlightType={highlightedSection ?? undefined}
            />
          </div>

          {/* --------------------------------------------------------------- */}
          {/* Rails card                                                        */}
          {/* --------------------------------------------------------------- */}
          <div style={CARD_STYLE}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <h2 style={CARD_TITLE_STYLE}>
                {t('admin.layout.railsTitle', { defaultValue: 'Rails' })}
              </h2>
              <button
                type="button"
                style={{
                  ...BTN_PRIMARY,
                  opacity: !railsDirty || saveRailsMutation.isPending ? 0.6 : 1,
                  cursor: !railsDirty || saveRailsMutation.isPending ? 'not-allowed' : 'pointer',
                }}
                disabled={!railsDirty || saveRailsMutation.isPending}
                onClick={() => saveRailsMutation.mutate()}
                data-testid="save-rails-btn"
              >
                {saveRailsMutation.isPending
                  ? t('admin.layout.saving', { defaultValue: 'Saving…' })
                  : t('admin.layout.saveRails', { defaultValue: 'Save Rails' })}
              </button>
            </div>

            {/* Same reseed-remount discipline as SectionTreeEditor above. */}
            <RailsEditor
              key={`${selectedScreen}:${reseedEpoch}`}
              rails={localRails}
              sectionTypes={sectionTypes}
              onChange={handleRailsChange}
            />
          </div>

          {/* --------------------------------------------------------------- */}
          {/* Publish & History card                                            */}
          {/* --------------------------------------------------------------- */}
          <div style={CARD_STYLE}>
            <h2 style={CARD_TITLE_STYLE}>
              {t('admin.layout.publishTitle', { defaultValue: 'Publish & History' })}
            </h2>

            {/* 422 publish errors — persistent list */}
            {publishErrors.length > 0 && (
              <div style={ALERT_STYLE} role="alert" data-testid="publish-errors">
                <strong>
                  {t('admin.layout.publishValidationErrors', {
                    defaultValue: 'Publish validation errors:',
                  })}
                </strong>
                <ul style={{ margin: '8px 0 0 16px', padding: 0 }}>
                  {publishErrors.map((err) => (
                    <li key={err}>{err}</li>
                  ))}
                </ul>
              </div>
            )}

            <div style={ROW_STYLE}>
              {/* Publish also stays disabled while a draft/rails save is in
                  flight — publishing mid-save would race the server state. */}
              <button
                type="button"
                style={{
                  ...BTN_PRIMARY,
                  opacity:
                    publishMutation.isPending ||
                    saveDraftMutation.isPending ||
                    saveRailsMutation.isPending
                      ? 0.6
                      : 1,
                  cursor:
                    publishMutation.isPending ||
                    saveDraftMutation.isPending ||
                    saveRailsMutation.isPending
                      ? 'not-allowed'
                      : 'pointer',
                }}
                disabled={
                  publishMutation.isPending ||
                  saveDraftMutation.isPending ||
                  saveRailsMutation.isPending
                }
                onClick={() => publishMutation.mutate()}
                data-testid="publish-btn"
              >
                {publishMutation.isPending
                  ? t('admin.layout.publishing', { defaultValue: 'Publishing…' })
                  : t('admin.layout.publish', { defaultValue: 'Publish' })}
              </button>
              {(draftDirty || railsDirty) && (
                <span
                  style={{ fontSize: 12, color: 'var(--ppt-warning-700, #b45309)' }}
                  data-testid="publish-dirty-warning"
                >
                  {t('admin.layout.publishDirtyWarning', {
                    defaultValue: 'You have unsaved changes — Publish uses the last saved draft.',
                  })}
                </span>
              )}
            </div>

            {/* Version history table */}
            {versions.length > 0 && (
              <table style={TABLE_STYLE}>
                <thead>
                  <tr>
                    <th style={TH_STYLE}>
                      {t('admin.layout.versionCol', { defaultValue: 'Version' })}
                    </th>
                    <th style={TH_STYLE}>
                      {t('admin.layout.publishedAtCol', { defaultValue: 'Published at' })}
                    </th>
                    <th style={TH_STYLE} />
                  </tr>
                </thead>
                <tbody>
                  {versions.map((v) => (
                    <tr key={v.version}>
                      <td style={TD_STYLE}>{v.version}</td>
                      <td style={TD_STYLE}>{new Date(v.published_at).toLocaleString()}</td>
                      <td style={TD_STYLE}>
                        <button
                          type="button"
                          style={BTN_DANGER}
                          onClick={() => handleRollback(v.version)}
                          data-testid={`rollback-btn-${v.version}`}
                          disabled={rollbackMutation.isPending}
                        >
                          {t('admin.layout.rollback', { defaultValue: 'Rollback' })}
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>

          {/* --------------------------------------------------------------- */}
          {/* Danger zone — Kills                                               */}
          {/* --------------------------------------------------------------- */}
          {kills.length > 0 && (
            <div style={{ ...CARD_STYLE, borderColor: 'var(--ppt-danger-200, #fecaca)' }}>
              <h2 style={{ ...CARD_TITLE_STYLE, color: 'var(--ppt-danger-700, #b91c1c)' }}>
                {t('admin.layout.dangerZone', { defaultValue: 'Danger zone — killed sections' })}
              </h2>
              <p style={{ fontSize: 13, color: 'var(--ppt-fg-secondary, #6b7280)', margin: 0 }}>
                {t('admin.layout.killsDescription', {
                  defaultValue: 'These section types are killed (globally hidden for all users).',
                })}
              </p>
              <ul style={{ margin: 0, paddingLeft: 20, fontSize: 13 }}>
                {kills.map((k) => (
                  <li key={k} style={{ fontFamily: 'monospace' }}>
                    {k}
                  </li>
                ))}
              </ul>
            </div>
          )}

          {/* --------------------------------------------------------------- */}
          {/* Live Preview card                                                  */}
          {/* --------------------------------------------------------------- */}
          <div style={CARD_STYLE}>
            <h2 style={CARD_TITLE_STYLE}>
              {t('admin.layout.preview.title', { defaultValue: 'Live Preview' })}
            </h2>
            <PreviewPanel
              screen={selectedScreen}
              platform={platform}
              draft={localDraft}
              token={token}
              onSectionSelected={setHighlightedSection}
            />
          </div>
        </>
      )}

      {/* Loading indicator */}
      {selectedScreen && configQuery.isLoading && (
        <div style={{ fontSize: 13, color: 'var(--ppt-fg-secondary, #6b7280)' }}>
          {t('admin.layout.loadingConfig', { defaultValue: 'Loading config…' })}
        </div>
      )}
    </div>
  );
}
