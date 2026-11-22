/**
 * TenantLifecyclePage — Tenant Lifecycle management.
 *
 * Implements Section E of the admin design brief:
 *   - Export card  (gate: tenant_export)
 *   - Purge card   (gate: tenant_purge) with live cooldown countdown + inline purge wizard
 *   - Restore card (gate: tenant_restore) with multipart file upload
 *
 * Route: /tenants/lifecycle
 * Page-level gating: any of tenant_export | tenant_purge | tenant_restore.
 * Inner capability checks hide individual action buttons/controls.
 */

import { useCapability } from '@ppt/admin-ui';
import { FileUpload, Stepper } from '@ppt/ui-kit';
import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { useAdminAuth } from '../auth/AdminAuthContext';
import { AuditReasonPrompt, useAuditReasonValid } from '../components/AuditReasonPrompt';
import { useToast } from '../components/Toast';

// ---------------------------------------------------------------------------
// Demo tenant (hard-coded until :id param iteration)
// ---------------------------------------------------------------------------

export interface TenantInfo {
  id: string;
  slug: string;
  name: string;
  softDeletedAt: Date | null;
  lastExportAt: Date | null;
  lastExportBy: string | null;
  lastExportSizeGb: number | null;
}

/** No tenant loaded yet: every tenant-scoped action stays disabled. */
const NO_TENANT: TenantInfo = {
  id: '',
  slug: '',
  name: '—',
  softDeletedAt: null,
  lastExportAt: null,
  lastExportBy: null,
  lastExportSizeGb: null,
};

/**
 * Resolve a tenant slug to its organization via
 * `GET /api/v1/platform-admin/organizations?search=` (exact slug match).
 * The server exposes no soft-delete / last-export metadata yet, so those
 * stay unknown (null) and purge stays gated behind "not soft-deleted".
 */
export async function lookupTenantBySlug(
  slug: string,
  token: string | null
): Promise<TenantInfo | null> {
  const resp = await fetch(
    `/api/v1/platform-admin/organizations?search=${encodeURIComponent(slug)}&page_size=100`,
    { headers: token ? { Authorization: `Bearer ${token}` } : {}, credentials: 'include' }
  );
  if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
  const data = (await resp.json()) as {
    organizations?: { id: string; slug: string; name: string }[];
  };
  const org = (data.organizations ?? []).find((o) => o.slug === slug);
  return org ? { ...NO_TENANT, id: org.id, slug: org.slug, name: org.name } : null;
}

// 24-hour cooldown in ms
const PURGE_COOLDOWN_MS = 24 * 60 * 60 * 1000;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatDate(d: Date): string {
  return d.toISOString().slice(0, 10);
}

/** Format remaining cooldown time as "Xh YYm" or "Mm SSs". */
function formatCooldown(remainingMs: number): string {
  const totalSecs = Math.max(0, Math.ceil(remainingMs / 1000));
  const hours = Math.floor(totalSecs / 3600);
  const mins = Math.floor((totalSecs % 3600) / 60);
  const secs = totalSecs % 60;
  if (hours > 0) {
    return `${hours}h ${String(mins).padStart(2, '0')}m`;
  }
  return `${mins}m ${String(secs).padStart(2, '0')}s`;
}

// ---------------------------------------------------------------------------
// Inline styles injector (scoped, runs once)
// ---------------------------------------------------------------------------

const STYLE_ID = 'ppt-lifecycle-page-styles';

function ensureStyles() {
  if (typeof document === 'undefined') return;
  if (document.getElementById(STYLE_ID)) return;
  const el = document.createElement('style');
  el.id = STYLE_ID;
  el.textContent = `
    .lc-page { padding: 24px 32px; background: var(--ppt-bg-app, #f9fafb); min-height: 100%; }

    /* Header */
    .lc-header { margin-bottom: 4px; }
    .lc-title { font-size: 1.375rem; font-weight: 700; color: var(--ppt-fg-primary, #111827); margin: 0 0 6px; }
    .lc-subline { font-size: 0.8125rem; color: var(--ppt-fg-muted, #6b7280); }
    .lc-subline span { margin: 0 6px; color: var(--ppt-border-default, #d1d5db); }

    /* Tenant picker */
    .lc-picker { display: flex; align-items: center; gap: 8px; margin-bottom: 20px; }
    .lc-picker label { font-size: 13px; font-weight: 500; color: var(--ppt-fg-secondary, #374151); }
    .lc-picker input {
      font-size: 13px;
      padding: 6px 10px;
      border: 1px solid var(--ppt-border-default, #e5e7eb);
      border-radius: var(--ppt-radius-md, 8px);
      background: var(--ppt-bg-input, #fff);
      color: var(--ppt-fg-primary, #111827);
      outline: none;
      width: 220px;
    }
    .lc-picker input:focus {
      border-color: var(--ppt-brand-500, #3b82f6);
      box-shadow: 0 0 0 2px rgba(59,130,246,0.25);
    }

    /* Card grid */
    .lc-grid {
      display: grid;
      grid-template-columns: repeat(3, 1fr);
      gap: 12px;
      margin-top: 16px;
    }

    /* Base card */
    .lc-card {
      padding: 20px;
      background: var(--ppt-bg-surface, #fff);
      border: 1px solid var(--ppt-border-default, #e5e7eb);
      border-radius: var(--ppt-radius-lg, 12px);
      display: flex;
      flex-direction: column;
      gap: 10px;
    }

    /* Danger card */
    .lc-card.danger {
      background: var(--ppt-danger-50, #fef2f2);
      border-color: var(--ppt-danger-500, #ef4444);
    }

    .lc-card-title { font-size: 15px; font-weight: 600; margin: 0; color: var(--ppt-fg-primary, #111827); }
    .lc-card-title.danger { color: var(--ppt-danger-800, #991b1b); }
    .lc-card-body { font-size: 13px; color: var(--ppt-fg-secondary, #374151); line-height: 1.55; margin: 0; }
    .lc-card-body.danger { color: var(--ppt-danger-800, #991b1b); }
    .lc-card-subtext { font-size: 12px; color: var(--ppt-fg-muted, #6b7280); margin: 0; }
    .lc-card-subtext a { color: var(--ppt-fg-link, #2563eb); text-decoration: underline; cursor: pointer; }

    /* Buttons */
    .lc-btn {
      display: inline-flex;
      align-items: center;
      gap: 6px;
      padding: 8px 14px;
      border-radius: var(--ppt-radius-md, 8px);
      font-size: 13px;
      font-weight: 500;
      cursor: pointer;
      border: none;
      transition: background 120ms ease, opacity 120ms ease;
      width: fit-content;
    }
    .lc-btn--primary {
      background: var(--ppt-brand-600, #2563eb);
      color: #fff;
    }
    .lc-btn--primary:hover:not(:disabled) { background: var(--ppt-brand-700, #1d4ed8); }
    .lc-btn--danger {
      background: var(--ppt-danger-600, #dc2626);
      color: #fff;
      border: none;
    }
    .lc-btn--danger:hover:not(:disabled) { background: var(--ppt-danger-700, #b91c1c); }
    .lc-btn--ghost {
      background: transparent;
      color: var(--ppt-fg-secondary, #374151);
      border: 1px solid var(--ppt-border-default, #e5e7eb);
    }
    .lc-btn--ghost:hover:not(:disabled) { background: var(--ppt-bg-hover, #f3f4f6); }
    .lc-btn:disabled { opacity: 0.45; cursor: not-allowed; }

    /* Countdown hint */
    .lc-btn-hint { font-size: 12px; color: var(--ppt-fg-muted, #6b7280); margin: 0; }

    /* File restore — dropzone provided by @ppt/ui-kit <FileUpload>; only the submit button margin remains */
    .lc-restore-actions { margin-top: 12px; }

    /* ──── Purge wizard ──── */
    .lc-wizard {
      border: 1px solid var(--ppt-danger-500, #ef4444);
      background: var(--ppt-bg-surface, #fff);
      padding: 24px;
      border-radius: var(--ppt-radius-lg, 12px);
      margin-top: 16px;
    }
    .lc-wizard-title { font-size: 14px; font-weight: 600; color: var(--ppt-danger-800, #991b1b); margin: 0 0 16px; }

    /* Step indicator — rendered by @ppt/ui-kit <Stepper>; only spacing remains here */
    .lc-stepper { margin-bottom: 24px; }

    /* Step body */
    .lc-step-body { display: flex; flex-direction: column; gap: 14px; }
    .lc-step-label { font-size: 13px; font-weight: 500; color: var(--ppt-fg-secondary, #374151); margin: 0 0 6px; }
    .lc-slug-input {
      font-family: var(--ppt-font-mono, monospace);
      font-size: 13px;
      padding: 8px 10px;
      border: 1px solid var(--ppt-border-default, #e5e7eb);
      border-radius: var(--ppt-radius-md, 8px);
      background: var(--ppt-bg-input, #fff);
      color: var(--ppt-fg-primary, #111827);
      outline: none;
      width: 100%;
      box-sizing: border-box;
      transition: border-color 120ms ease, box-shadow 120ms ease;
    }
    .lc-slug-input:focus {
      border-color: var(--ppt-brand-500, #3b82f6);
      box-shadow: 0 0 0 2px rgba(59,130,246,0.25);
    }
    .lc-mismatch { font-size: 12px; color: var(--ppt-warning-800, #92400e); margin: 0; }

    /* TOTP inputs */
    .lc-totp-row { display: flex; gap: 6px; }
    .lc-totp-digit {
      width: 40px;
      height: 48px;
      text-align: center;
      font-size: 18px;
      font-weight: 600;
      border: 1px solid var(--ppt-border-default, #e5e7eb);
      border-radius: var(--ppt-radius-md, 8px);
      background: var(--ppt-bg-input, #fff);
      color: var(--ppt-fg-primary, #111827);
      outline: none;
      transition: border-color 120ms ease;
    }
    .lc-totp-digit:focus { border-color: var(--ppt-brand-500, #3b82f6); box-shadow: 0 0 0 2px rgba(59,130,246,0.25); }
    .lc-totp-label { font-size: 13px; font-weight: 500; color: var(--ppt-fg-secondary, #374151); margin-bottom: 6px; }

    /* Wizard actions row */
    .lc-wizard-actions { display: flex; gap: 10px; margin-top: 8px; }
    .lc-spinner {
      display: inline-block;
      width: 14px; height: 14px;
      border: 2px solid rgba(255,255,255,0.4);
      border-top-color: #fff;
      border-radius: 50%;
      animation: lc-spin 0.7s linear infinite;
      flex-shrink: 0;
    }
    @keyframes lc-spin { to { transform: rotate(360deg); } }
  `;
  document.head.appendChild(el);
}

// ---------------------------------------------------------------------------
// Purge wizard (inline, below the grid)
// ---------------------------------------------------------------------------

interface PurgeWizardProps {
  tenant: TenantInfo;
  onCancel: () => void;
}

// Exported (in addition to the default `TenantLifecyclePage`) so tests can
// drive the wizard directly without first arranging a soft-deleted tenant
// fixture in the parent page. Also exported is `TenantInfo` for fixture use.
export function PurgeWizard({ tenant, onCancel }: PurgeWizardProps) {
  const { t } = useTranslation();
  // Gate "Start export now" button on tenant_export capability
  const canExportInWizard = useCapability('tenant_export');
  const [step, setStep] = useState<1 | 2 | 3>(1);
  const [slugTyped, setSlugTyped] = useState('');
  const [reason, setReason] = useState('');
  const [totp, setTotp] = useState(['', '', '', '', '', '']);
  const [isPending, setIsPending] = useState(false);
  const totpRefs = useRef<(HTMLInputElement | null)[]>([]);
  const { token } = useAdminAuth();
  const { showToast } = useToast();

  const reasonValid = useAuditReasonValid(reason, 30);
  const slugMatch = slugTyped === tenant.slug;

  // Step 1: auto-pass if last export < 7 days old
  const exportAgeDays =
    tenant.lastExportAt != null
      ? (Date.now() - tenant.lastExportAt.getTime()) / 86_400_000
      : Infinity;
  const exportOk = exportAgeDays < 7;

  // Move to step 2 when step 1 is satisfied
  useEffect(() => {
    if (step === 1 && exportOk) {
      setStep(2);
    }
  }, [step, exportOk]);

  // TOTP digit input handler with auto-advance
  const handleTotpChange = (idx: number, val: string) => {
    const digit = val.replace(/\D/g, '').slice(-1);
    const next = [...totp];
    next[idx] = digit;
    setTotp(next);
    if (digit && idx < 5) {
      totpRefs.current[idx + 1]?.focus();
    }
  };

  const handleTotpKeyDown = (idx: number, e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Backspace' && !totp[idx] && idx > 0) {
      totpRefs.current[idx - 1]?.focus();
    }
  };

  const totpFull = totp.join('').length === 6;

  const handlePurge = useCallback(async () => {
    if (isPending) return;
    setIsPending(true);
    try {
      const resp = await fetch(`/api/v1/admin/tenants/${tenant.id}/purge`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Authorization: `Bearer ${token ?? ''}`,
          'X-MFA-Recent': '1',
        },
        body: JSON.stringify({ confirm: 'DELETE-PERMANENT', reason }),
      });

      if (resp.ok) {
        showToast({
          type: 'success',
          title: t('admin.tenantLifecycle.wizard.purgedTitle', { defaultValue: 'Tenant purged' }),
          message: t('admin.tenantLifecycle.wizard.purgedMessage', {
            name: tenant.name,
            defaultValue: '{{name}} has been permanently purged.',
          }),
        });
        onCancel();
      } else if (resp.status === 401) {
        showToast({
          type: 'error',
          title: t('admin.tenantLifecycle.wizard.mfaRequiredTitle', {
            defaultValue: 'MFA required',
          }),
          message: t('admin.tenantLifecycle.wizard.mfaRequiredMessage', {
            defaultValue: 'Re-verify MFA and try again.',
          }),
        });
      } else {
        const text = await resp.text().catch(() => resp.statusText);
        showToast({
          type: 'error',
          title: t('admin.tenantLifecycle.wizard.purgeFailedTitle', {
            status: resp.status,
            defaultValue: 'Purge failed ({{status}})',
          }),
          message: text,
        });
      }
    } catch {
      showToast({
        type: 'error',
        title: t('admin.tenantLifecycle.wizard.networkErrorTitle', {
          defaultValue: 'Network error',
        }),
        message: t('admin.tenantLifecycle.wizard.networkErrorMessage', {
          defaultValue: 'Unable to reach the server. Check your connection and try again.',
        }),
      });
    } finally {
      setIsPending(false);
    }
  }, [isPending, tenant, reason, token, showToast, onCancel, t]);

  return (
    <div
      className="lc-wizard"
      role="region"
      aria-label={t('admin.tenantLifecycle.wizard.region', { defaultValue: 'Purge wizard' })}
    >
      <p className="lc-wizard-title">
        {t('admin.tenantLifecycle.wizard.title', {
          defaultValue: 'Purge wizard — proceed carefully',
        })}
      </p>

      {/* Step indicator — shared @ppt/ui-kit primitive (step is 1-based, Stepper is 0-based) */}
      <Stepper
        className="lc-stepper"
        steps={[
          t('admin.tenantLifecycle.wizard.stepConfirmExport', { defaultValue: 'Confirm export' }),
          t('admin.tenantLifecycle.wizard.stepTypeSlug', { defaultValue: 'Type slug' }),
          t('admin.tenantLifecycle.wizard.stepMfaReason', { defaultValue: 'MFA + reason' }),
        ]}
        current={step - 1}
      />

      <div className="lc-step-body">
        {/* Step 1 */}
        {step === 1 && !exportOk && (
          <div>
            <p
              style={{ fontSize: 13, color: 'var(--ppt-warning-800, #92400e)', margin: '0 0 12px' }}
            >
              {t('admin.tenantLifecycle.wizard.exportRequired', {
                age:
                  exportAgeDays === Infinity
                    ? t('admin.tenantLifecycle.wizard.exportAgeNever', { defaultValue: 'never' })
                    : t('admin.tenantLifecycle.wizard.exportAgeDays', {
                        days: Math.floor(exportAgeDays),
                        defaultValue: '{{days}} days ago',
                      }),
                defaultValue:
                  'An export less than 7 days old is required before purge. Last export was {{age}}.',
              })}
            </p>
            {canExportInWizard && (
              <button
                type="button"
                className="lc-btn lc-btn--primary"
                onClick={() => {
                  // Trigger export + advance only when the server actually
                  // accepts it. `fetch` only rejects on network errors, so
                  // 4xx/5xx still resolves — guard with response.ok.
                  fetch(`/api/v1/admin/tenants/${tenant.id}/export`, {
                    method: 'POST',
                    headers: { Authorization: `Bearer ${token ?? ''}` },
                  })
                    .then((resp) => {
                      if (!resp.ok) {
                        showToast({
                          type: 'error',
                          title: t('admin.tenantLifecycle.wizard.exportFailedTitle', {
                            defaultValue: 'Export failed',
                          }),
                          message: t('admin.tenantLifecycle.wizard.exportFailedServer', {
                            status: resp.status,
                            defaultValue: 'Server returned {{status}}. Cannot proceed to purge.',
                          }),
                        });
                        return;
                      }
                      setStep(2);
                    })
                    .catch(() => {
                      showToast({
                        type: 'error',
                        title: t('admin.tenantLifecycle.wizard.exportFailedTitle', {
                          defaultValue: 'Export failed',
                        }),
                        message: t('admin.tenantLifecycle.wizard.exportFailedUnable', {
                          defaultValue: 'Unable to start export.',
                        }),
                      });
                    });
                }}
              >
                {t('admin.tenantLifecycle.wizard.startExportNow', {
                  defaultValue: 'Start export now',
                })}
              </button>
            )}
          </div>
        )}

        {/* Step 2 */}
        {step === 2 && (
          <div>
            <p className="lc-step-label">
              {t('admin.tenantLifecycle.wizard.typeSlugExactly', {
                defaultValue: 'Type the tenant slug exactly:',
              })}{' '}
              <code
                style={{
                  fontFamily: 'var(--ppt-font-mono, monospace)',
                  background: 'var(--ppt-bg-subtle, #f3f4f6)',
                  padding: '2px 6px',
                  borderRadius: 4,
                }}
              >
                {tenant.slug}
              </code>
            </p>
            <input
              className="lc-slug-input"
              type="text"
              value={slugTyped}
              onChange={(e) => setSlugTyped(e.target.value)}
              onPaste={(e) => e.preventDefault()}
              autoComplete="off"
              autoCorrect="off"
              autoCapitalize="off"
              spellCheck={false}
              aria-label={t('admin.tenantLifecycle.wizard.confirmSlugAria', {
                slug: tenant.slug,
                defaultValue: 'Type "{{slug}}" to confirm',
              })}
            />
            {slugTyped.length > 0 && !slugMatch && (
              <p className="lc-mismatch" role="alert">
                {t('admin.tenantLifecycle.wizard.slugMismatch', {
                  defaultValue: "⚠ Doesn't match yet — keep typing",
                })}
              </p>
            )}
            <div className="lc-wizard-actions">
              <button
                type="button"
                className="lc-btn lc-btn--primary"
                disabled={!slugMatch}
                onClick={() => setStep(3)}
              >
                {t('admin.tenantLifecycle.wizard.continueToMfa', {
                  defaultValue: 'Continue to MFA',
                })}
              </button>
              <button type="button" className="lc-btn lc-btn--ghost" onClick={onCancel}>
                {t('common.cancel', { defaultValue: 'Cancel' })}
              </button>
            </div>
          </div>
        )}

        {/* Step 3 */}
        {step === 3 && (
          <div style={{ display: 'flex', flexDirection: 'column', gap: 16 }}>
            <AuditReasonPrompt
              action="tenant_purge"
              minLength={30}
              value={reason}
              onChange={setReason}
            />

            <div>
              <p className="lc-totp-label">
                {t('admin.tenantLifecycle.wizard.enterTotp', {
                  defaultValue: 'Enter 6-digit authenticator code',
                })}
              </p>
              <div
                className="lc-totp-row"
                role="group"
                aria-label={t('admin.tenantLifecycle.wizard.totpGroupAria', {
                  defaultValue: 'TOTP code digits',
                })}
              >
                {totp.map((digit, idx) => (
                  <input
                    key={idx}
                    ref={(el) => {
                      totpRefs.current[idx] = el;
                    }}
                    className="lc-totp-digit"
                    type="text"
                    inputMode="numeric"
                    maxLength={1}
                    value={digit}
                    onChange={(e) => handleTotpChange(idx, e.target.value)}
                    onKeyDown={(e) => handleTotpKeyDown(idx, e)}
                    aria-label={t('admin.tenantLifecycle.wizard.totpDigitAria', {
                      index: idx + 1,
                      defaultValue: 'Digit {{index}} of 6',
                    })}
                    autoComplete="one-time-code"
                  />
                ))}
              </div>
            </div>

            <div className="lc-wizard-actions">
              <button
                type="button"
                className="lc-btn lc-btn--danger"
                disabled={!reasonValid || !totpFull || isPending}
                onClick={handlePurge}
              >
                {isPending && <span className="lc-spinner" aria-hidden="true" />}
                {isPending
                  ? t('admin.tenantLifecycle.wizard.purging', { defaultValue: 'Purging…' })
                  : t('admin.tenantLifecycle.wizard.verifyAndPurge', {
                      defaultValue: 'Verify and purge',
                    })}
              </button>
              <button
                type="button"
                className="lc-btn lc-btn--ghost"
                onClick={onCancel}
                disabled={isPending}
              >
                {t('common.cancel', { defaultValue: 'Cancel' })}
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main page component
// ---------------------------------------------------------------------------

export default function TenantLifecyclePage() {
  ensureStyles();
  const { t } = useTranslation();

  // Capability checks for OR-gate (page-level gating mirrors ProtectedRoute
  // which also uses any-of, but we double-check here and show ForbiddenPage
  // when none are held so the inner page logic is always guarded).
  const canExport = useCapability('tenant_export');
  const canPurge = useCapability('tenant_purge');
  const canRestore = useCapability('tenant_restore');

  const [slugInput, setSlugInput] = useState('');
  const [tenant, setTenant] = useState<TenantInfo>(NO_TENANT);
  const [isLoadingTenant, setIsLoadingTenant] = useState(false);

  const [purgeWizardOpen, setPurgeWizardOpen] = useState(false);
  const [cooldownMs, setCooldownMs] = useState<number>(0);
  const cooldownIntervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const [restoreFile, setRestoreFile] = useState<File | null>(null);
  const [isExporting, setIsExporting] = useState(false);
  const [isUploading, setIsUploading] = useState(false);

  const { token } = useAdminAuth();
  const { showToast } = useToast();

  // Capability gating is enforced by `ProtectedRoute` in App.tsx (route is
  // wrapped with requiredCapability=['tenant_export','tenant_purge','tenant_restore']
  // which treats the array as ANY-of). A redundant in-page early return here
  // would violate Rules of Hooks — it would skip the useEffect/useCallback
  // declarations below on initial render and re-run them after a cap flip,
  // throwing "Rendered fewer hooks than expected".
  // Per-button caps still gate individual affordances inside each card.

  // Compute initial cooldown on tenant change
  // biome-ignore lint/correctness/useExhaustiveDependencies: intentional on tenant change
  useEffect(() => {
    if (cooldownIntervalRef.current) {
      clearInterval(cooldownIntervalRef.current);
      cooldownIntervalRef.current = null;
    }

    if (!tenant.softDeletedAt) {
      setCooldownMs(0);
      return;
    }

    const elapsed = Date.now() - tenant.softDeletedAt.getTime();
    const remaining = Math.max(0, PURGE_COOLDOWN_MS - elapsed);
    setCooldownMs(remaining);

    if (remaining > 0) {
      cooldownIntervalRef.current = setInterval(() => {
        setCooldownMs((prev) => {
          const next = prev - 1000;
          if (next <= 0) {
            if (cooldownIntervalRef.current) clearInterval(cooldownIntervalRef.current);
            return 0;
          }
          return next;
        });
      }, 1000);
    }

    return () => {
      if (cooldownIntervalRef.current) clearInterval(cooldownIntervalRef.current);
    };
  }, [tenant.id]);

  // Load tenant from slug input
  const handleLoadTenant = useCallback(async () => {
    const slug = slugInput.trim();
    if (!slug) return;
    setPurgeWizardOpen(false);
    setIsLoadingTenant(true);
    try {
      const found = await lookupTenantBySlug(slug, token);
      setTenant(found ?? NO_TENANT);
      if (!found) {
        showToast({
          type: 'error',
          title: t('admin.tenantLifecycle.picker.notFound', {
            slug,
            defaultValue: 'No tenant with slug "{{slug}}"',
          }),
        });
      }
    } catch (err) {
      setTenant(NO_TENANT);
      showToast({
        type: 'error',
        title: t('admin.tenantLifecycle.picker.loadFailed', {
          defaultValue: 'Could not load tenant',
        }),
        message: err instanceof Error ? err.message : String(err),
      });
    } finally {
      setIsLoadingTenant(false);
    }
  }, [slugInput, token, showToast, t]);

  // Export action
  const handleExport = useCallback(async () => {
    if (isExporting) return;
    setIsExporting(true);
    try {
      const resp = await fetch(`/api/v1/admin/tenants/${tenant.id}/export`, {
        method: 'POST',
        headers: { Authorization: `Bearer ${token ?? ''}` },
      });
      if (resp.ok) {
        showToast({
          type: 'success',
          title: t('admin.tenantLifecycle.export.startedTitle', { defaultValue: 'Export started' }),
          message: t('admin.tenantLifecycle.export.startedMessage', {
            defaultValue: 'A new export is being generated. You will be notified when it is ready.',
          }),
        });
      } else {
        showToast({
          type: 'error',
          title: t('admin.tenantLifecycle.export.failedTitle', {
            status: resp.status,
            defaultValue: 'Export failed ({{status}})',
          }),
          message: await resp.text().catch(() => resp.statusText),
        });
      }
    } catch {
      showToast({
        type: 'error',
        title: t('admin.tenantLifecycle.export.networkErrorTitle', {
          defaultValue: 'Network error',
        }),
        message: t('admin.tenantLifecycle.export.networkErrorMessage', {
          defaultValue: 'Unable to start export. Check your connection and try again.',
        }),
      });
    } finally {
      setIsExporting(false);
    }
  }, [isExporting, tenant.id, token, showToast, t]);

  // Restore upload — multipart POST to the admin tenant-restore endpoint.
  // Defense #16: restore always lands as a NEW org id server-side, so the
  // endpoint carries no tenant :id. The capability extractor enforces
  // tenant_restore + recent MFA; X-MFA-Recent mirrors the purge call.
  const handleUpload = useCallback(async () => {
    if (!restoreFile || isUploading) return;
    setIsUploading(true);
    try {
      const formData = new FormData();
      formData.append('tarball', restoreFile);
      const resp = await fetch('/api/v1/admin/tenants/restore', {
        method: 'POST',
        // Note: do NOT set Content-Type — the browser must add the multipart
        // boundary itself.
        headers: {
          Authorization: `Bearer ${token ?? ''}`,
          'X-MFA-Recent': '1',
        },
        body: formData,
      });

      if (resp.ok) {
        const report = (await resp.json().catch(() => null)) as {
          new_organization_id?: string;
          rows_restored?: number;
        } | null;
        showToast({
          type: 'success',
          title: t('admin.tenantLifecycle.restore.restoredTitle', {
            defaultValue: 'Tenant restored',
          }),
          message: report?.new_organization_id
            ? t('admin.tenantLifecycle.restore.restoredIntoNew', {
                id: report.new_organization_id,
                rows: report.rows_restored ?? 0,
                defaultValue: 'Restored into new tenant {{id}} ({{rows}} rows).',
              })
            : t('admin.tenantLifecycle.restore.restoredGeneric', {
                name: restoreFile.name,
                defaultValue: '{{name}} restored into a new tenant.',
              }),
        });
        setRestoreFile(null);
      } else if (resp.status === 401) {
        showToast({
          type: 'error',
          title: t('admin.tenantLifecycle.restore.mfaRequiredTitle', {
            defaultValue: 'MFA required',
          }),
          message: t('admin.tenantLifecycle.restore.mfaRequiredMessage', {
            defaultValue: 'Re-verify MFA and try again.',
          }),
        });
      } else {
        const text = await resp.text().catch(() => resp.statusText);
        showToast({
          type: 'error',
          title: t('admin.tenantLifecycle.restore.failedTitle', {
            status: resp.status,
            defaultValue: 'Restore failed ({{status}})',
          }),
          message: text,
        });
      }
    } catch {
      showToast({
        type: 'error',
        title: t('admin.tenantLifecycle.restore.networkErrorTitle', {
          defaultValue: 'Network error',
        }),
        message: t('admin.tenantLifecycle.restore.networkErrorMessage', {
          defaultValue: 'Unable to reach the server. Check your connection and try again.',
        }),
      });
    } finally {
      setIsUploading(false);
    }
  }, [restoreFile, isUploading, token, showToast, t]);

  // Purge button state
  const tenantSoftDeleted = tenant.softDeletedAt !== null;
  const cooldownActive = cooldownMs > 0;
  const purgeButtonDisabled = !tenant.id || !tenantSoftDeleted || cooldownActive;

  const purgeButtonLabel = !tenantSoftDeleted
    ? t('admin.tenantLifecycle.purge.buttonNotSoftDeleted', {
        defaultValue: 'Tenant is not soft-deleted yet',
      })
    : cooldownActive
      ? t('admin.tenantLifecycle.purge.buttonCooldown', {
          time: formatCooldown(cooldownMs),
          defaultValue: 'Cooldown · {{time}} left',
        })
      : t('admin.tenantLifecycle.purge.button', { defaultValue: 'Purge tenant' });

  return (
    <div className="lc-page">
      {/* Tenant picker */}
      <div className="lc-picker">
        <label htmlFor="lc-slug-picker">
          {t('admin.tenantLifecycle.picker.label', { defaultValue: 'Tenant slug' })}
        </label>
        <input
          id="lc-slug-picker"
          type="text"
          placeholder="tenant-slug"
          value={slugInput}
          onChange={(e) => setSlugInput(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') handleLoadTenant();
          }}
        />
        <button
          type="button"
          className="lc-btn lc-btn--ghost"
          onClick={handleLoadTenant}
          disabled={isLoadingTenant || !slugInput.trim()}
        >
          {t('admin.tenantLifecycle.picker.load', { defaultValue: 'Load' })}
        </button>
      </div>

      {/* Page header */}
      <div className="lc-header">
        <h1 className="lc-title">
          {t('admin.tenantLifecycle.header.title', {
            name: tenant.name,
            defaultValue: 'Lifecycle · {{name}}',
          })}
        </h1>
        <p className="lc-subline">
          {t('admin.tenantLifecycle.header.slug', { defaultValue: 'Tenant slug:' })}{' '}
          <strong>{tenant.slug}</strong>
          <span aria-hidden="true">·</span>
          {t('admin.tenantLifecycle.header.lastExport', { defaultValue: 'Last export:' })}{' '}
          {tenant.lastExportAt
            ? formatDate(tenant.lastExportAt)
            : t('admin.tenantLifecycle.header.never', { defaultValue: 'Never' })}
          <span aria-hidden="true">·</span>
          {t('admin.tenantLifecycle.header.softDeletedAt', { defaultValue: 'Soft-deleted at:' })}{' '}
          {tenant.softDeletedAt
            ? formatDate(tenant.softDeletedAt)
            : t('admin.tenantLifecycle.header.active', { defaultValue: 'Active' })}
        </p>
      </div>

      {/* Card grid */}
      <div className="lc-grid">
        {/* ── Export card ── */}
        <div className="lc-card">
          <h3 className="lc-card-title">
            {t('admin.tenantLifecycle.export.title', { defaultValue: 'Export' })}
          </h3>
          <p className="lc-card-body">
            {t('admin.tenantLifecycle.export.body', {
              defaultValue:
                'Full tenant dump as .tar.gz. Includes listings, memberships, audit rows, blobs.',
            })}
          </p>
          {canExport && (
            <button
              type="button"
              className="lc-btn lc-btn--primary"
              onClick={handleExport}
              disabled={isExporting || !tenant.id}
            >
              {isExporting && <span className="lc-spinner" aria-hidden="true" />}
              {isExporting
                ? t('admin.tenantLifecycle.export.starting', { defaultValue: 'Starting…' })
                : t('admin.tenantLifecycle.export.start', { defaultValue: 'Start export' })}
            </button>
          )}
          {tenant.lastExportAt ? (
            <p className="lc-card-subtext">
              {t('admin.tenantLifecycle.export.lastExportBy', {
                date: formatDate(tenant.lastExportAt),
                by:
                  tenant.lastExportBy ??
                  t('admin.tenantLifecycle.export.unknownUser', { defaultValue: 'unknown' }),
                defaultValue: 'Last export: {{date}} by {{by}} — ',
              })}
              {/* biome-ignore lint/a11y/useValidAnchor: placeholder download link */}
              <a onClick={(e) => e.preventDefault()} href="#">
                {t('admin.tenantLifecycle.export.download', {
                  size: tenant.lastExportSizeGb?.toFixed(1) ?? '?',
                  defaultValue: 'download ({{size}} GB)',
                })}
              </a>
            </p>
          ) : (
            <p className="lc-card-subtext">
              {t('admin.tenantLifecycle.export.none', { defaultValue: 'No exports yet.' })}
            </p>
          )}
        </div>

        {/* ── Purge card ── */}
        <div className="lc-card danger">
          <h3 className="lc-card-title danger">
            {t('admin.tenantLifecycle.purge.title', { defaultValue: 'Purge · irreversible' })}
          </h3>
          <p className="lc-card-body danger">
            {t('admin.tenantLifecycle.purge.body', {
              defaultValue:
                'Deletes all tenant data including audit log. After 24-hour cooldown from soft-delete. Cannot be undone.',
            })}
          </p>
          {!tenantSoftDeleted && (
            <p className="lc-btn-hint">
              {t('admin.tenantLifecycle.purge.needsSoftDelete', {
                defaultValue: 'Tenant must be soft-deleted before purge can be initiated.',
              })}
            </p>
          )}
          {canPurge && (
            <>
              <button
                type="button"
                className="lc-btn lc-btn--danger"
                disabled={purgeButtonDisabled}
                onClick={() => setPurgeWizardOpen(true)}
                aria-label={purgeButtonLabel}
              >
                {purgeButtonLabel}
              </button>
              {cooldownActive && (
                <p className="lc-btn-hint" aria-live="polite">
                  {t('admin.tenantLifecycle.purge.cooldownRemaining', {
                    time: formatCooldown(cooldownMs),
                    defaultValue: 'Cooldown: {{time}} remaining',
                  })}
                </p>
              )}
            </>
          )}
        </div>

        {/* ── Restore card ── */}
        <div className="lc-card">
          <h3 className="lc-card-title">
            {t('admin.tenantLifecycle.restore.title', { defaultValue: 'Restore' })}
          </h3>
          <p className="lc-card-body">
            {t('admin.tenantLifecycle.restore.body', {
              defaultValue:
                'Upload a previously exported tarball into a new tenant slug. Multipart — resumes on disconnect.',
            })}
          </p>
          {canRestore && (
            <>
              <FileUpload
                accept=".tar.gz"
                multiple={false}
                onFiles={(files) => setRestoreFile(files[0] ?? null)}
                onRemove={() => setRestoreFile(null)}
                files={
                  restoreFile
                    ? [
                        {
                          id: restoreFile.name,
                          file: restoreFile,
                          progress: isUploading ? 50 : 0,
                          status: isUploading ? 'uploading' : 'idle',
                        },
                      ]
                    : []
                }
              />
              {restoreFile && (
                <div className="lc-restore-actions">
                  <button
                    type="button"
                    className="lc-btn lc-btn--primary"
                    onClick={handleUpload}
                    disabled={isUploading}
                  >
                    {isUploading && <span className="lc-spinner" aria-hidden="true" />}
                    {isUploading
                      ? t('admin.tenantLifecycle.restore.uploading', { defaultValue: 'Uploading…' })
                      : t('admin.tenantLifecycle.restore.upload', { defaultValue: 'Upload' })}
                  </button>
                </div>
              )}
            </>
          )}
        </div>
      </div>

      {/* Purge wizard (inline, below grid) */}
      {purgeWizardOpen && canPurge && (
        <PurgeWizard tenant={tenant} onCancel={() => setPurgeWizardOpen(false)} />
      )}
    </div>
  );
}

TenantLifecyclePage.displayName = 'TenantLifecyclePage';
