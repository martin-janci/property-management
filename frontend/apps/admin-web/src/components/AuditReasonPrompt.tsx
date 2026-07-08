/**
 * AuditReasonPrompt — inline textarea for audit reason capture.
 *
 * Appears before destructive/sensitive admin operations to capture
 * a human-readable audit trail reason. Never blocks typing — shows hints
 * after the user has started typing.
 *
 * Validation:
 *   - Minimum character length (per-action defaults)
 *   - Single repeated character pattern rejected
 *
 * Exposes a hidden input data-valid attribute so parent forms can read
 * validity without coupling to component internals.
 */

import { useMemo } from 'react';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type AuditReasonAction =
  | 'tenant_purge'
  | 'tenant_restore'
  | 'principal_kind_escalate'
  | 'grant_principal_kind_escalate'
  | 'grant_tenant_purge'
  | 'force_update_floor_bump'
  | 'maintenance_mode_on'
  | 'feature_flag_toggle'
  | 'oauth_scope_grant'
  | 'agency_suspend'
  | 'agency_reactivate';

export interface AuditReasonPromptProps {
  action: AuditReasonAction;
  minLength?: number;
  value: string;
  onChange: (v: string) => void;
  required?: boolean;
}

// ---------------------------------------------------------------------------
// Action metadata
// ---------------------------------------------------------------------------

interface ActionMeta {
  defaultMinLength: number;
  helper: string;
  risk: 'red' | 'amber';
}

const ACTION_META: Record<AuditReasonAction, ActionMeta> = {
  tenant_purge: {
    defaultMinLength: 30,
    helper:
      'Explain why this tenant is being permanently purged. This reason will be stored in the audit log and cannot be edited.',
    risk: 'red',
  },
  tenant_restore: {
    defaultMinLength: 30,
    helper:
      'Explain why a tenant archive is being restored into a new slug. Include the source tenant and any data migration notes.',
    risk: 'red',
  },
  principal_kind_escalate: {
    defaultMinLength: 30,
    helper:
      'Explain why this user account is being escalated to a higher principal kind. Include the business justification.',
    risk: 'red',
  },
  grant_principal_kind_escalate: {
    defaultMinLength: 30,
    helper:
      "Explain why you're granting the principal_kind_escalate capability. Include who approved this and the intended use.",
    risk: 'red',
  },
  grant_tenant_purge: {
    defaultMinLength: 30,
    helper:
      "Explain why you're granting the tenant_purge capability. Include who approved this and the intended scope.",
    risk: 'red',
  },
  force_update_floor_bump: {
    defaultMinLength: 20,
    helper:
      'Explain why a forced floor-version bump is necessary. Include the affected mobile versions.',
    risk: 'amber',
  },
  maintenance_mode_on: {
    defaultMinLength: 20,
    helper: 'Describe the maintenance being performed and expected duration.',
    risk: 'amber',
  },
  feature_flag_toggle: {
    defaultMinLength: 20,
    helper:
      'Explain why this feature flag is being toggled and which tenants or cohorts are affected.',
    risk: 'amber',
  },
  oauth_scope_grant: {
    defaultMinLength: 20,
    helper:
      'Explain why the OAuth scope list for this client is being changed. Include the business justification and which scopes are being added or removed.',
    risk: 'amber',
  },
  agency_suspend: {
    defaultMinLength: 20,
    helper:
      'Explain why this organization is being suspended. All member sessions are revoked immediately; this reason is stored in the audit log and shown on the organization record.',
    risk: 'red',
  },
  agency_reactivate: {
    defaultMinLength: 20,
    helper:
      'Optionally note why this organization is being reactivated (e.g. invoices settled, dispute resolved). Stored in the audit log.',
    risk: 'amber',
  },
};

/**
 * Default minimum reason length for an action — the single source of truth
 * shared with callers that need it (e.g. to drive `useAuditReasonValid`
 * before the component is mounted). Avoids re-hardcoding per-action lengths
 * at call sites where they would silently drift from `ACTION_META`.
 */
export function auditReasonMinLength(action: AuditReasonAction): number {
  return ACTION_META[action].defaultMinLength;
}

// ---------------------------------------------------------------------------
// Validation hook
// ---------------------------------------------------------------------------

const SINGLE_REPEATED_CHAR_RE = /^(.)\1+$/;

/** Returns true when the reason is valid per the given constraints. */
export function useAuditReasonValid(value: string, minLength: number): boolean {
  return useMemo(() => {
    if (value.length < minLength) return false;
    if (SINGLE_REPEATED_CHAR_RE.test(value.trim())) return false;
    return true;
  }, [value, minLength]);
}

// ---------------------------------------------------------------------------
// Styles
// ---------------------------------------------------------------------------

const STYLE_ID = 'ppt-audit-reason-prompt-styles';

function ensureStyles() {
  if (typeof document === 'undefined') return;
  if (document.getElementById(STYLE_ID)) return;
  const el = document.createElement('style');
  el.id = STYLE_ID;
  el.textContent = `
    .ppt-arp {
      display: flex;
      flex-direction: column;
      gap: 6px;
    }
    .ppt-arp__header {
      display: flex;
      align-items: center;
      gap: 8px;
    }
    .ppt-arp__label {
      font-size: 13px;
      font-weight: 600;
      color: var(--ppt-fg-primary, #111827);
    }
    .ppt-arp__badge {
      display: inline-flex;
      align-items: center;
      padding: 2px 8px;
      border-radius: 999px;
      font-size: 11px;
      font-weight: 600;
      letter-spacing: 0.04em;
      text-transform: uppercase;
    }
    .ppt-arp__badge--red {
      background: var(--ppt-danger-100, #fee2e2);
      color: var(--ppt-danger-800, #991b1b);
    }
    .ppt-arp__badge--amber {
      background: var(--ppt-warning-100, #fef3c7);
      color: var(--ppt-warning-800, #92400e);
    }
    .ppt-arp__helper {
      font-size: 12px;
      color: var(--ppt-fg-muted, #6b7280);
      line-height: 1.5;
    }
    .ppt-arp__textarea {
      width: 100%;
      box-sizing: border-box;
      font-family: var(--ppt-font-mono, 'JetBrains Mono', 'SF Mono', 'Menlo', 'Consolas', monospace);
      font-size: 13px;
      line-height: 1.6;
      padding: 8px 10px;
      border: 1px solid var(--ppt-border-default, #e5e7eb);
      border-radius: var(--ppt-radius-md, 8px);
      background: var(--ppt-bg-input, #ffffff);
      color: var(--ppt-fg-primary, #111827);
      resize: none;
      outline: none;
      transition: border-color 120ms ease, box-shadow 120ms ease;
    }
    .ppt-arp__textarea:focus {
      border-color: var(--ppt-brand-500, #3b82f6);
      box-shadow: 0 0 0 2px var(--ppt-brand-500, #3b82f6);
    }
    .ppt-arp__footer {
      display: flex;
      align-items: baseline;
      gap: 8px;
    }
    .ppt-arp__counter {
      font-size: 12px;
      color: var(--ppt-fg-muted, #6b7280);
      font-variant-numeric: tabular-nums;
    }
    .ppt-arp__counter--invalid {
      color: var(--ppt-danger-600, #dc2626);
    }
    .ppt-arp__hint {
      font-size: 12px;
      color: var(--ppt-danger-600, #dc2626);
    }
  `;
  document.head.appendChild(el);
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function AuditReasonPrompt({
  action,
  minLength,
  value,
  onChange,
  required = true,
}: AuditReasonPromptProps) {
  ensureStyles();

  const meta = ACTION_META[action];
  const resolvedMin = minLength ?? meta.defaultMinLength;
  const isValid = useAuditReasonValid(value, resolvedMin);

  const hint = useMemo<string>(() => {
    if (value.length === 0) return '';
    if (SINGLE_REPEATED_CHAR_RE.test(value.trim())) {
      return "Reason can't be a single repeated character";
    }
    const missing = resolvedMin - value.length;
    if (missing > 0) {
      return `Needs ${missing} more character${missing === 1 ? '' : 's'}`;
    }
    return '';
  }, [value, resolvedMin]);

  const counterBelowMin = value.length < resolvedMin;

  return (
    <div className="ppt-arp">
      <div className="ppt-arp__header">
        <span className="ppt-arp__label">Audit reason</span>
        <span className={`ppt-arp__badge ppt-arp__badge--${meta.risk}`}>
          {meta.risk === 'red' ? 'High risk' : 'Medium risk'}
        </span>
      </div>

      <p className="ppt-arp__helper">{meta.helper}</p>

      <textarea
        className="ppt-arp__textarea"
        rows={4}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        required={required}
        aria-label="Audit reason"
        aria-describedby="ppt-arp-hint"
        autoComplete="off"
        spellCheck
      />

      <div className="ppt-arp__footer">
        <span
          className={`ppt-arp__counter${counterBelowMin ? ' ppt-arp__counter--invalid' : ''}`}
          aria-live="polite"
          aria-label={`${value.length} of ${resolvedMin} minimum characters`}
        >
          {value.length} / {resolvedMin}
        </span>
        {hint && (
          <span id="ppt-arp-hint" className="ppt-arp__hint" role="alert">
            {hint}
          </span>
        )}
      </div>

      {/* Hidden input for form-level validity checks */}
      <input type="hidden" data-valid={isValid} />
    </div>
  );
}

AuditReasonPrompt.displayName = 'AuditReasonPrompt';
