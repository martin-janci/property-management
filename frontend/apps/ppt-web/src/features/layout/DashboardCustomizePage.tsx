/**
 * DashboardCustomizePage — Task 4 (layout tenant-editor plan).
 *
 * Org-admin / super-admin only. Allows customising the ppt/dashboard layout
 * via the tenant-override API (Task 3 envelope / Task 2 hooks).
 *
 * Route: /dashboard/customize (lazy, ProtectedRoute requiredRoles org_admin|super_admin)
 */
import {
  type LayoutRails,
  TenantLayoutError,
  type TenantOverride,
  useSaveTenantLayoutOverride,
  useTenantLayout,
} from '@ppt/api-client';
import { useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Link } from 'react-router-dom';
import { useToast } from '../../components';
import { TenantSectionEditor } from './TenantSectionEditor';

const SCREEN = 'ppt/dashboard';

export function DashboardCustomizePage() {
  const { t } = useTranslation();
  const { showToast } = useToast();

  // Fetch the tenant layout envelope
  const { data: envelope, isLoading, isError, refetch } = useTenantLayout(SCREEN);

  // Local editor state — seeded once per load
  const [localOverride, setLocalOverride] = useState<TenantOverride | null>(null);
  const [isDirty, setIsDirty] = useState(false);

  // Validation errors from a 422 response — cleared on next change
  const [validationErrors, setValidationErrors] = useState<string[]>([]);

  // Mirror the latest committed override in a ref so an async save-success
  // callback can read the CURRENT state (not the stale value captured by the
  // closure that started the save). Updated every render, so an edit made
  // while a save is in flight is visible here by the time the save resolves.
  const localOverrideRef = useRef<TenantOverride | null>(localOverride);
  localOverrideRef.current = localOverride;

  const mutation = useSaveTenantLayoutOverride(SCREEN);

  // Seed the local override once per load (when data arrives)
  const seedRef = useRef(false);
  if (envelope && !seedRef.current) {
    seedRef.current = true;
    setLocalOverride(envelope.override?.override_config ?? {});
  }

  const handleChange = (next: TenantOverride) => {
    setLocalOverride(next);
    setIsDirty(true);
    setValidationErrors([]);
  };

  const handleSave = async () => {
    if (localOverride === null) return;
    // Snapshot the payload we're about to send. Comparing the CURRENT state
    // against this snapshot on success tells us whether the user edited while
    // the save was in flight.
    const sentJson = JSON.stringify(localOverride);
    try {
      await mutation.mutateAsync(localOverride);
      showToast({
        type: 'success',
        title: t('layout.customize.saveSuccess'),
      });
      // Compare the CURRENT override (via the ref) against what we sent. If the
      // user edited during the in-flight save the two diverge, so we keep the
      // form dirty instead of silently discarding the newer edit.
      if (JSON.stringify(localOverrideRef.current) === sentJson) {
        setIsDirty(false);
      } else {
        setIsDirty(true);
      }
    } catch (err) {
      // A 422 with an empty `errors` array would render an empty alert list and
      // no other feedback — fall through to the generic toast instead.
      if (err instanceof TenantLayoutError && err.status === 422 && err.errors.length > 0) {
        setValidationErrors(err.errors);
      } else {
        showToast({
          type: 'error',
          title: t('layout.customize.saveError'),
        });
      }
    }
  };

  const handleReset = () => {
    if (!window.confirm(t('layout.customize.resetConfirm'))) return;
    setLocalOverride({});
    setIsDirty(true);
    setValidationErrors([]);
  };

  // Loading state
  if (isLoading) {
    return (
      <div className="dashboard-customize-page">
        <p role="status" aria-live="polite">
          {t('common.loading')}
        </p>
      </div>
    );
  }

  // Error state
  if (isError || !envelope) {
    return (
      <div className="dashboard-customize-page">
        <p role="alert">{t('layout.customize.loadError')}</p>
        <button type="button" onClick={() => refetch()}>
          {t('common.retry')}
        </button>
      </div>
    );
  }

  const published = envelope.published;
  const rails: LayoutRails = {
    hideable: [],
    mode_editable: [],
    reorderable: false,
    prop_whitelist: {},
    ...(envelope.rails as Partial<LayoutRails>),
  };

  return (
    <div className="dashboard-customize-page">
      <header className="dashboard-customize-page__header">
        <Link to="/dashboard/manager" className="dashboard-customize-page__back">
          {t('layout.customize.back')}
        </Link>
        <h1>{t('layout.customize.title')}</h1>
      </header>

      {/* Not-published info panel */}
      {published === null ? (
        <p className="dashboard-customize-page__info">{t('layout.customize.notPublished')}</p>
      ) : (
        <>
          {/* 422 validation errors */}
          {validationErrors.length > 0 && (
            <ul role="alert" className="dashboard-customize-page__errors">
              {validationErrors.map((e) => (
                <li key={e}>{e}</li>
              ))}
            </ul>
          )}

          {localOverride !== null && (
            <TenantSectionEditor
              baseSections={published.sections}
              rails={rails}
              manifest={envelope.manifest}
              override={localOverride}
              onChange={handleChange}
            />
          )}

          <div className="dashboard-customize-page__actions">
            <button
              type="button"
              onClick={handleSave}
              disabled={!isDirty || mutation.isPending}
              aria-label={t('layout.customize.save')}
            >
              {t('layout.customize.save')}
            </button>
            <button type="button" onClick={handleReset}>
              {t('layout.customize.reset')}
            </button>
          </div>
        </>
      )}
    </div>
  );
}
