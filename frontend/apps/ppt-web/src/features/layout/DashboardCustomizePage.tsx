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

  // Track the JSON we last sent so we can conditionally clear dirty
  const sentJsonRef = useRef<string | null>(null);

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
    const json = JSON.stringify(localOverride);
    sentJsonRef.current = json;
    try {
      await mutation.mutateAsync(localOverride);
      showToast({
        type: 'success',
        title: t('layout.customize.saveSuccess'),
      });
      // Only clear dirty if the override hasn't changed since we sent
      if (JSON.stringify(localOverride) === json) {
        setIsDirty(false);
      }
    } catch (err) {
      if (err instanceof TenantLayoutError && err.status === 422) {
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
