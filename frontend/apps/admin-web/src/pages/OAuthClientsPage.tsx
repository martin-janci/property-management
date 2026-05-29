/**
 * OAuthClientsPage — `/identity/oauth-clients`
 *
 * Admin console for managing OAuth 2.0 clients registered in the PPT
 * Authorization Server (Epic 10A-2).
 *
 * Wired to:
 *   GET    /api/v1/admin/oauth/clients           — list all clients
 *   POST   /api/v1/admin/oauth/clients           — register new client
 *   GET    /api/v1/admin/oauth/clients/{id}      — get single client
 *   PATCH  /api/v1/admin/oauth/clients/{id}      — update client
 *   DELETE /api/v1/admin/oauth/clients/{id}      — revoke client
 *   POST   /api/v1/admin/oauth/clients/{id}/regenerate-secret
 *
 * All endpoints are gated by the oauth_client_write capability.
 */

import {
  KNOWN_OAUTH_SCOPES,
  OAUTH_SCOPE_DESCRIPTIONS,
  type OAuthClientSummary,
  type RegisterOAuthClientRequest,
  type UpdateOAuthClientRequest,
  useOAuthClients,
  useRegenerateOAuthClientSecret,
  useRegisterOAuthClient,
  useRevokeOAuthClient,
  useUpdateOAuthClient,
} from '@ppt/api-client';
import {
  type ChangeEvent,
  type FormEvent,
  type KeyboardEvent,
  useCallback,
  useEffect,
  useId,
  useRef,
  useState,
} from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import {
  AuditReasonPrompt,
  auditReasonMinLength,
  useAuditReasonValid,
} from '../components/AuditReasonPrompt';
import { DestructiveConfirmDialog } from '../components/DestructiveConfirmDialog';
import { useToast } from '../components/Toast';
import { HelpTooltip } from '../features/help';

// ============================================================
// Utility helpers
// ============================================================

function formatDate(iso: string): string {
  try {
    return new Intl.DateTimeFormat('en-GB', { dateStyle: 'medium' }).format(new Date(iso));
  } catch {
    return iso;
  }
}

async function copyToClipboard(text: string): Promise<void> {
  if (navigator.clipboard) {
    return navigator.clipboard.writeText(text);
  }
  const el = document.createElement('textarea');
  el.value = text;
  el.style.position = 'fixed';
  el.style.opacity = '0';
  document.body.appendChild(el);
  el.select();
  document.execCommand('copy');
  document.body.removeChild(el);
}

// ============================================================
// Inline styles (injected once)
// ============================================================

const STYLE_ID = 'ppt-oauth-clients-styles';

function ensureStyles() {
  if (typeof document === 'undefined') return;
  if (document.getElementById(STYLE_ID)) return;
  const el = document.createElement('style');
  el.id = STYLE_ID;
  el.textContent = `
    .ppt-oc-page { max-width: 900px; margin: 0 auto; padding: 24px 20px; }
    .ppt-oc-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 24px; }
    .ppt-oc-title { font-size: 22px; font-weight: 700; color: var(--ppt-fg-primary,#111827); margin: 0; }
    .ppt-oc-empty {
      display: flex; flex-direction: column; align-items: center; justify-content: center;
      border: 2px dashed var(--ppt-border-default,#e5e7eb); border-radius: 12px;
      padding: 64px 24px; gap: 16px; color: var(--ppt-fg-secondary,#6b7280);
    }
    .ppt-oc-empty-icon { font-size: 40px; }
    .ppt-oc-empty-text { font-size: 15px; }
    .ppt-oc-status { padding: 32px; text-align: center; color: var(--ppt-fg-secondary,#6b7280); }
    .ppt-oc-retry { margin-top: 12px; }
    .ppt-oc-list { display: flex; flex-direction: column; gap: 14px; }
    .ppt-oc-card {
      border: 1px solid var(--ppt-border-default,#e5e7eb);
      border-radius: 10px; padding: 18px 20px;
      background: var(--ppt-bg-surface,#fff);
      display: flex; flex-direction: column; gap: 10px;
    }
    .ppt-oc-card--revoked { opacity: 0.5; }
    .ppt-oc-card-top { display: flex; align-items: flex-start; justify-content: space-between; gap: 12px; }
    .ppt-oc-card-info { flex: 1; min-width: 0; }
    .ppt-oc-card-name { font-size: 15px; font-weight: 600; color: var(--ppt-fg-primary,#111827); margin: 0 0 2px; }
    .ppt-oc-card-desc { font-size: 13px; color: var(--ppt-fg-secondary,#6b7280); margin: 0 0 6px; }
    .ppt-oc-card-meta { display: flex; flex-wrap: wrap; gap: 12px; font-size: 12px; color: var(--ppt-fg-muted,#9ca3af); }
    .ppt-oc-client-id { font-family: var(--ppt-font-mono,monospace); font-size: 12px; }
    .ppt-oc-badge {
      display: inline-block; padding: 2px 8px; border-radius: 999px;
      font-size: 11px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.04em;
    }
    .ppt-oc-badge--active { background: #d1fae5; color: #065f46; }
    .ppt-oc-badge--revoked { background: #fee2e2; color: #991b1b; }
    .ppt-oc-scopes { display: flex; flex-wrap: wrap; gap: 6px; }
    .ppt-oc-scope-chip {
      background: var(--ppt-bg-subtle,#f3f4f6); color: var(--ppt-fg-secondary,#374151);
      border-radius: 4px; padding: 2px 7px; font-size: 12px;
      font-family: var(--ppt-font-mono,monospace);
    }
    .ppt-oc-actions { display: flex; gap: 8px; flex-shrink: 0; }
    .ppt-oc-btn {
      display: inline-flex; align-items: center; gap: 6px;
      padding: 6px 12px; border-radius: 6px; font-size: 13px; font-weight: 500;
      cursor: pointer; border: 1px solid transparent; transition: background 120ms ease;
    }
    .ppt-oc-btn:disabled { opacity: 0.4; cursor: not-allowed; }
    .ppt-oc-btn--primary { background: var(--ppt-brand-500,#3b82f6); color: #fff; border-color: var(--ppt-brand-500,#3b82f6); }
    .ppt-oc-btn--primary:hover:not(:disabled) { background: var(--ppt-brand-600,#2563eb); }
    .ppt-oc-btn--secondary { background: transparent; color: var(--ppt-fg-secondary,#374151); border-color: var(--ppt-border-default,#e5e7eb); }
    .ppt-oc-btn--secondary:hover:not(:disabled) { background: var(--ppt-bg-hover,#f3f4f6); }
    .ppt-oc-btn--danger { background: transparent; color: var(--ppt-danger-600,#dc2626); border-color: var(--ppt-danger-300,#fca5a5); }
    .ppt-oc-btn--danger:hover:not(:disabled) { background: #fee2e2; }
    .ppt-oc-modal-backdrop {
      position: fixed; inset: 0; z-index: 9000;
      background: rgba(0,0,0,0.45);
      display: flex; align-items: center; justify-content: center; padding: 16px;
    }
    .ppt-oc-modal-panel {
      max-width: 540px; width: 100%;
      background: var(--ppt-bg-surface,#fff);
      border-radius: 12px; padding: 28px 28px 24px;
      box-shadow: 0 12px 48px rgba(0,0,0,0.18);
      display: flex; flex-direction: column; gap: 18px;
      max-height: calc(100vh - 64px); overflow-y: auto;
    }
    .ppt-oc-modal-title { font-size: 17px; font-weight: 700; margin: 0; color: var(--ppt-fg-primary,#111827); }
    .ppt-oc-modal-footer { display: flex; justify-content: flex-end; gap: 10px; margin-top: 4px; }
    .ppt-oc-field { display: flex; flex-direction: column; gap: 5px; }
    .ppt-oc-label { font-size: 13px; font-weight: 500; color: var(--ppt-fg-secondary,#374151); }
    .ppt-oc-input, .ppt-oc-textarea {
      width: 100%; box-sizing: border-box;
      padding: 8px 10px; font-size: 14px;
      border: 1px solid var(--ppt-border-default,#e5e7eb); border-radius: 7px;
      background: var(--ppt-bg-input,#fff); color: var(--ppt-fg-primary,#111827);
      outline: none; transition: border-color 120ms;
    }
    .ppt-oc-input:focus, .ppt-oc-textarea:focus { border-color: var(--ppt-brand-500,#3b82f6); box-shadow: 0 0 0 2px rgba(59,130,246,0.2); }
    .ppt-oc-textarea { resize: vertical; min-height: 72px; font-family: inherit; }
    .ppt-oc-scope-grid { display: flex; flex-wrap: wrap; gap: 10px; }
    .ppt-oc-scope-label { display: flex; align-items: center; gap: 7px; cursor: pointer; font-size: 13px; color: var(--ppt-fg-secondary,#374151); }
    .ppt-oc-scope-info {
      position: relative; display: inline-flex; align-items: center;
    }
    .ppt-oc-scope-info-btn {
      display: inline-flex; align-items: center; justify-content: center;
      width: 14px; height: 14px; border-radius: 50%;
      border: 1px solid var(--ppt-border-default,#e5e7eb);
      background: var(--ppt-bg-subtle,#f3f4f6); color: var(--ppt-fg-muted,#6b7280);
      font-size: 9px; font-weight: 700; cursor: default; line-height: 1; padding: 0;
      flex-shrink: 0;
    }
    .ppt-oc-scope-info-btn:hover, .ppt-oc-scope-info-btn:focus {
      background: var(--ppt-brand-100,#dbeafe); border-color: var(--ppt-brand-300,#93c5fd);
      color: var(--ppt-brand-700,#1d4ed8); outline: none;
    }
    .ppt-oc-scope-tooltip {
      position: absolute; bottom: calc(100% + 5px); left: 50%;
      transform: translateX(-50%);
      background: var(--ppt-fg-primary,#111827); color: #fff;
      font-size: 11px; line-height: 1.5; padding: 5px 8px;
      border-radius: 5px; width: 190px; white-space: normal;
      pointer-events: none; z-index: 400;
      box-shadow: 0 4px 12px rgba(0,0,0,0.2);
    }
    .ppt-oc-scope-tooltip::after {
      content: ''; position: absolute; top: 100%; left: 50%;
      transform: translateX(-50%);
      border: 4px solid transparent;
      border-top-color: var(--ppt-fg-primary,#111827);
    }
    .ppt-oc-audit-notice {
      font-size: 12px; color: var(--ppt-warning-700,#b45309);
      background: #fef3c7; border-radius: 5px; padding: 7px 10px;
      border: 1px solid #fde68a;
    }
    .ppt-oc-uri-list { display: flex; flex-direction: column; gap: 6px; }
    .ppt-oc-uri-row { display: flex; gap: 6px; align-items: center; }
    .ppt-oc-uri-input { flex: 1; }
    .ppt-oc-uri-remove { padding: 6px 10px; font-size: 16px; line-height: 1; }
    .ppt-oc-uri-add { align-self: flex-start; margin-top: 2px; }
    .ppt-oc-secret-box {
      background: var(--ppt-bg-subtle,#f3f4f6); border-radius: 7px; padding: 14px;
      display: flex; flex-direction: column; gap: 8px;
    }
    .ppt-oc-secret-label { font-size: 12px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; color: var(--ppt-fg-muted,#6b7280); }
    .ppt-oc-secret-value { font-family: var(--ppt-font-mono,monospace); font-size: 13px; word-break: break-all; color: var(--ppt-fg-primary,#111827); }
    .ppt-oc-secret-warning { font-size: 12px; color: var(--ppt-warning-700,#b45309); background: #fef3c7; border-radius: 5px; padding: 8px 10px; }
    .ppt-oc-copy-btn { align-self: flex-start; font-size: 12px; padding: 4px 10px; }
    .ppt-oc-toggle-row { display: flex; align-items: center; gap: 10px; font-size: 13px; color: var(--ppt-fg-secondary,#374151); cursor: pointer; }
    .ppt-oc-spinner {
      width: 14px; height: 14px; border: 2px solid rgba(255,255,255,0.4);
      border-top-color: #fff; border-radius: 50%;
      animation: ppt-oc-spin 0.7s linear infinite; flex-shrink: 0; display: inline-block;
    }
    .ppt-oc-spinner--dark { border-color: rgba(55,65,81,0.3); border-top-color: #374151; }
    @keyframes ppt-oc-spin { to { transform: rotate(360deg); } }
  `;
  document.head.appendChild(el);
}

// ============================================================
// ScopeInfoTip — small (?) button with hover/focus tooltip
// ============================================================

interface ScopeInfoTipProps {
  description: string;
}

function ScopeInfoTip({ description }: ScopeInfoTipProps) {
  const [visible, setVisible] = useState(false);
  // Stable id so the trigger can reference the tooltip via aria-describedby.
  const tooltipId = useId();
  return (
    <span className="ppt-oc-scope-info">
      <button
        type="button"
        className="ppt-oc-scope-info-btn"
        aria-label="Scope description"
        aria-describedby={visible ? tooltipId : undefined}
        aria-expanded={visible}
        onMouseEnter={() => setVisible(true)}
        onMouseLeave={() => setVisible(false)}
        onFocus={() => setVisible(true)}
        onBlur={() => setVisible(false)}
        // Touch devices have no hover — toggle on click/tap too.
        onClick={() => setVisible((v) => !v)}
        tabIndex={0}
      >
        ?
      </button>
      {/* Conditional render keeps the hidden tooltip out of the a11y tree
          (visibility:hidden alone can still be announced by some AT). */}
      {visible && (
        <span id={tooltipId} className="ppt-oc-scope-tooltip" role="tooltip">
          {description}
        </span>
      )}
    </span>
  );
}

// ============================================================
// ScopeSelector
// ============================================================

interface ScopeSelectorProps {
  selected: string[];
  onChange: (scopes: string[]) => void;
}

function ScopeSelector({ selected, onChange }: ScopeSelectorProps) {
  function toggle(scope: string) {
    onChange(selected.includes(scope) ? selected.filter((s) => s !== scope) : [...selected, scope]);
  }
  return (
    <div className="ppt-oc-scope-grid">
      {KNOWN_OAUTH_SCOPES.map((scope) => (
        <label key={scope} className="ppt-oc-scope-label">
          <input
            type="checkbox"
            checked={selected.includes(scope)}
            onChange={() => toggle(scope)}
          />
          <code>{scope}</code>
          <ScopeInfoTip
            description={
              OAUTH_SCOPE_DESCRIPTIONS[scope as keyof typeof OAUTH_SCOPE_DESCRIPTIONS] ?? scope
            }
          />
        </label>
      ))}
    </div>
  );
}

// ============================================================
// RedirectUriEditor
// ============================================================

interface RedirectUriEditorProps {
  uris: string[];
  onChange: (uris: string[]) => void;
}

function RedirectUriEditor({ uris, onChange }: RedirectUriEditorProps) {
  return (
    <div className="ppt-oc-uri-list">
      {uris.map((uri, i) => (
        <div key={i} className="ppt-oc-uri-row">
          <input
            type="url"
            className="ppt-oc-input ppt-oc-uri-input"
            value={uri}
            placeholder="https://example.com/callback"
            onChange={(e: ChangeEvent<HTMLInputElement>) => {
              const next = [...uris];
              next[i] = e.target.value;
              onChange(next);
            }}
          />
          <button
            type="button"
            className="ppt-oc-btn ppt-oc-btn--secondary ppt-oc-uri-remove"
            onClick={() => onChange(uris.filter((_, j) => j !== i))}
            aria-label="Remove URI"
          >
            &#x2715;
          </button>
        </div>
      ))}
      <button
        type="button"
        className="ppt-oc-btn ppt-oc-btn--secondary ppt-oc-uri-add"
        onClick={() => onChange([...uris, ''])}
      >
        + Add URI
      </button>
    </div>
  );
}

// ============================================================
// Modal shell
// ============================================================

const FOCUSABLE_SEL =
  'a[href],button:not([disabled]),input:not([disabled]),textarea:not([disabled]),select:not([disabled]),[tabindex]:not([tabindex="-1"])';

interface ModalProps {
  open: boolean;
  title: string;
  onClose: () => void;
  children: React.ReactNode;
  footer: React.ReactNode;
  labelledById: string;
}

function Modal({ open, title, onClose, children, footer, labelledById }: ModalProps) {
  const panelRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open || !panelRef.current) return;
    const nodes = panelRef.current.querySelectorAll<HTMLElement>(FOCUSABLE_SEL);
    if (nodes.length > 0) setTimeout(() => nodes[0].focus(), 0);
  }, [open]);

  function handleKeyDown(e: KeyboardEvent<HTMLDivElement>) {
    if (e.key === 'Escape') {
      onClose();
      return;
    }
    if (e.key !== 'Tab' || !panelRef.current) return;
    const focusable = Array.from(panelRef.current.querySelectorAll<HTMLElement>(FOCUSABLE_SEL));
    if (focusable.length === 0) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (e.shiftKey) {
      if (document.activeElement === first) {
        e.preventDefault();
        last.focus();
      }
    } else {
      if (document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    }
  }

  if (!open) return null;

  return createPortal(
    <div
      className="ppt-oc-modal-backdrop"
      role="dialog"
      aria-modal="true"
      aria-labelledby={labelledById}
      onKeyDown={handleKeyDown}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div ref={panelRef} className="ppt-oc-modal-panel" onClick={(e) => e.stopPropagation()}>
        <h2 id={labelledById} className="ppt-oc-modal-title">
          {title}
        </h2>
        {children}
        <div className="ppt-oc-modal-footer">{footer}</div>
      </div>
    </div>,
    document.body
  );
}

// ============================================================
// RegisterDialog
// ============================================================

interface RegisterDialogProps {
  open: boolean;
  onClose: () => void;
}

function RegisterDialog({ open, onClose }: RegisterDialogProps) {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const registerMutation = useRegisterOAuthClient();
  const titleId = useId();

  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [redirectUris, setRedirectUris] = useState<string[]>(['']);
  const [scopes, setScopes] = useState<string[]>(['profile']);
  const [isConfidential, setIsConfidential] = useState(true);
  const [rotateRefreshTokens, setRotateRefreshTokens] = useState(false);
  const [showSecret, setShowSecret] = useState<{ clientId: string; clientSecret: string } | null>(
    null
  );
  const [copiedField, setCopiedField] = useState<'id' | 'secret' | null>(null);

  function reset() {
    setName('');
    setDescription('');
    setRedirectUris(['']);
    setScopes(['profile']);
    setIsConfidential(true);
    setRotateRefreshTokens(false);
    setShowSecret(null);
    setCopiedField(null);
  }

  function handleClose() {
    reset();
    onClose();
  }

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    const req: RegisterOAuthClientRequest = {
      name: name.trim(),
      description: description.trim() || undefined,
      redirectUris: redirectUris.filter((u) => u.trim().length > 0),
      scopes,
      isConfidential,
      rotateRefreshTokens,
    };
    try {
      const res = await registerMutation.mutateAsync(req);
      setShowSecret({ clientId: res.clientId, clientSecret: res.clientSecret });
      showToast({
        type: 'success',
        title: t('admin.oauthClients.toast.registerSuccess'),
        duration: 4000,
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('admin.oauthClients.toast.registerError'),
        message: err instanceof Error ? err.message : String(err),
      });
    }
  }

  async function handleCopy(field: 'id' | 'secret', value: string) {
    await copyToClipboard(value);
    setCopiedField(field);
    setTimeout(() => setCopiedField(null), 2000);
  }

  if (showSecret) {
    return (
      <Modal
        open={open}
        title={t('admin.oauthClients.actions.register')}
        onClose={handleClose}
        labelledById={titleId}
        footer={
          <button type="button" className="ppt-oc-btn ppt-oc-btn--primary" onClick={handleClose}>
            Done
          </button>
        }
      >
        <p style={{ fontSize: 13, color: 'var(--ppt-fg-secondary,#374151)', margin: 0 }}>
          {t('admin.oauthClients.toast.registerSuccess')}
        </p>
        <div className="ppt-oc-secret-warning">
          Copy the client secret now — it will not be shown again.
        </div>
        <div className="ppt-oc-secret-box">
          <span className="ppt-oc-secret-label">Client ID</span>
          <span className="ppt-oc-secret-value">{showSecret.clientId}</span>
          <button
            type="button"
            className="ppt-oc-btn ppt-oc-btn--secondary ppt-oc-copy-btn"
            onClick={() => handleCopy('id', showSecret.clientId)}
          >
            {copiedField === 'id' ? 'Copied' : 'Copy'}
          </button>
        </div>
        <div className="ppt-oc-secret-box">
          <span className="ppt-oc-secret-label">Client Secret</span>
          <span className="ppt-oc-secret-value">{showSecret.clientSecret}</span>
          <button
            type="button"
            className="ppt-oc-btn ppt-oc-btn--secondary ppt-oc-copy-btn"
            onClick={() => handleCopy('secret', showSecret.clientSecret)}
          >
            {copiedField === 'secret' ? 'Copied' : 'Copy'}
          </button>
        </div>
      </Modal>
    );
  }

  return (
    <Modal
      open={open}
      title={t('admin.oauthClients.actions.register')}
      onClose={handleClose}
      labelledById={titleId}
      footer={
        <>
          <button
            type="button"
            className="ppt-oc-btn ppt-oc-btn--secondary"
            onClick={handleClose}
            disabled={registerMutation.isPending}
          >
            Cancel
          </button>
          <button
            type="submit"
            form="register-oauth-form"
            className="ppt-oc-btn ppt-oc-btn--primary"
            disabled={registerMutation.isPending || !name.trim()}
          >
            {registerMutation.isPending && <span className="ppt-oc-spinner" aria-hidden="true" />}
            Register
          </button>
        </>
      }
    >
      <form
        id="register-oauth-form"
        onSubmit={handleSubmit}
        style={{ display: 'flex', flexDirection: 'column', gap: 14 }}
      >
        <div className="ppt-oc-field">
          <label className="ppt-oc-label" htmlFor="oc-name">
            Name *
          </label>
          <input
            id="oc-name"
            className="ppt-oc-input"
            value={name}
            onChange={(e: ChangeEvent<HTMLInputElement>) => setName(e.target.value)}
            required
            autoComplete="off"
            placeholder="My Integration"
          />
        </div>
        <div className="ppt-oc-field">
          <label className="ppt-oc-label" htmlFor="oc-desc">
            Description
          </label>
          <textarea
            id="oc-desc"
            className="ppt-oc-textarea"
            value={description}
            onChange={(e: ChangeEvent<HTMLTextAreaElement>) => setDescription(e.target.value)}
            placeholder="Optional description"
          />
        </div>
        <div className="ppt-oc-field">
          <span className="ppt-oc-label">Redirect URIs</span>
          <RedirectUriEditor uris={redirectUris} onChange={setRedirectUris} />
        </div>
        <div className="ppt-oc-field">
          <span className="ppt-oc-label">Scopes</span>
          <ScopeSelector selected={scopes} onChange={setScopes} />
        </div>
        <label className="ppt-oc-toggle-row">
          <input
            type="checkbox"
            checked={isConfidential}
            onChange={(e: ChangeEvent<HTMLInputElement>) => setIsConfidential(e.target.checked)}
          />
          Confidential client (server-side app with client secret)
        </label>
        <label className="ppt-oc-toggle-row">
          <input
            type="checkbox"
            checked={rotateRefreshTokens}
            onChange={(e: ChangeEvent<HTMLInputElement>) =>
              setRotateRefreshTokens(e.target.checked)
            }
          />
          Rotate refresh tokens on use
        </label>
      </form>
    </Modal>
  );
}

// ============================================================
// EditDialog
// ============================================================

interface EditDialogProps {
  open: boolean;
  client: OAuthClientSummary | null;
  onClose: () => void;
}

/**
 * Returns true when two scope lists differ as sets (order-independent).
 * O(n) — builds a Set from `a` and checks membership/length against `b`,
 * avoiding the allocate-and-sort churn on every render. Scope lists are
 * de-duplicated server-side, so set semantics are sufficient here.
 */
function scopesChanged(a: string[], b: string[]): boolean {
  if (a.length !== b.length) return true;
  const setA = new Set(a);
  return b.some((v) => !setA.has(v));
}

// Resolved from AuditReasonPrompt's per-action metadata so the two never drift.
const SCOPE_AUDIT_MIN_LENGTH = auditReasonMinLength('oauth_scope_grant');

function EditDialog({ open, client, onClose }: EditDialogProps) {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const updateMutation = useUpdateOAuthClient(client?.id ?? '');
  const titleId = useId();

  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [scopes, setScopes] = useState<string[]>([]);
  const [originalScopes, setOriginalScopes] = useState<string[]>([]);
  const [isActive, setIsActive] = useState(true);
  const [auditReason, setAuditReason] = useState('');

  useEffect(() => {
    if (client) {
      setName(client.name);
      setDescription(client.description ?? '');
      setScopes(client.scopes);
      setOriginalScopes(client.scopes);
      setIsActive(client.isActive);
      setAuditReason('');
    }
  }, [client]);

  const hasScopeChange = scopesChanged(scopes, originalScopes);
  const auditReasonValid = useAuditReasonValid(auditReason, SCOPE_AUDIT_MIN_LENGTH);
  // Submit is blocked when scopes changed and no valid reason has been provided.
  const submitDisabled =
    updateMutation.isPending || !name.trim() || (hasScopeChange && !auditReasonValid);

  async function handleSubmit(e: FormEvent) {
    e.preventDefault();
    if (hasScopeChange && !auditReasonValid) return;
    const req: UpdateOAuthClientRequest = {
      name: name.trim(),
      description: description.trim() || undefined,
      scopes,
      isActive,
      auditReason: hasScopeChange && auditReason.trim() ? auditReason.trim() : undefined,
    };
    try {
      await updateMutation.mutateAsync(req);
      showToast({
        type: 'success',
        title: t('admin.oauthClients.toast.updateSuccess'),
        duration: 3000,
      });
      onClose();
    } catch (err) {
      showToast({
        type: 'error',
        title: t('admin.oauthClients.toast.updateError'),
        message: err instanceof Error ? err.message : String(err),
      });
    }
  }

  return (
    <Modal
      open={open}
      title={t('admin.oauthClients.actions.edit')}
      onClose={onClose}
      labelledById={titleId}
      footer={
        <>
          <button
            type="button"
            className="ppt-oc-btn ppt-oc-btn--secondary"
            onClick={onClose}
            disabled={updateMutation.isPending}
          >
            Cancel
          </button>
          <button
            type="submit"
            form="edit-oauth-form"
            className="ppt-oc-btn ppt-oc-btn--primary"
            disabled={submitDisabled}
            title={
              hasScopeChange && !auditReasonValid
                ? 'Provide an audit reason before saving scope changes'
                : undefined
            }
          >
            {updateMutation.isPending && <span className="ppt-oc-spinner" aria-hidden="true" />}
            Save
          </button>
        </>
      }
    >
      <form
        id="edit-oauth-form"
        onSubmit={handleSubmit}
        style={{ display: 'flex', flexDirection: 'column', gap: 14 }}
      >
        <div className="ppt-oc-field">
          <label className="ppt-oc-label" htmlFor="oc-edit-name">
            Name *
          </label>
          <input
            id="oc-edit-name"
            className="ppt-oc-input"
            value={name}
            onChange={(e: ChangeEvent<HTMLInputElement>) => setName(e.target.value)}
            required
            autoComplete="off"
          />
        </div>
        <div className="ppt-oc-field">
          <label className="ppt-oc-label" htmlFor="oc-edit-desc">
            Description
          </label>
          <textarea
            id="oc-edit-desc"
            className="ppt-oc-textarea"
            value={description}
            onChange={(e: ChangeEvent<HTMLTextAreaElement>) => setDescription(e.target.value)}
          />
        </div>
        <div className="ppt-oc-field">
          <span className="ppt-oc-label">Scopes</span>
          <ScopeSelector selected={scopes} onChange={setScopes} />
        </div>
        <label className="ppt-oc-toggle-row">
          <input
            type="checkbox"
            checked={isActive}
            onChange={(e: ChangeEvent<HTMLInputElement>) => setIsActive(e.target.checked)}
          />
          Active
        </label>

        {/* Scope-grant audit trail — shown only when the scope list has changed */}
        {hasScopeChange && (
          <div className="ppt-oc-field">
            <div className="ppt-oc-audit-notice" role="note">
              Scope grants changed — an audit reason is required before saving.
            </div>
            <div style={{ marginTop: 10 }}>
              <AuditReasonPrompt
                action="oauth_scope_grant"
                value={auditReason}
                onChange={setAuditReason}
                required
              />
            </div>
          </div>
        )}
      </form>
    </Modal>
  );
}

// ============================================================
// RegenerateSecretDialog
// ============================================================

interface RegenerateSecretDialogProps {
  open: boolean;
  client: OAuthClientSummary | null;
  onClose: () => void;
}

function RegenerateSecretDialog({ open, client, onClose }: RegenerateSecretDialogProps) {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const regenMutation = useRegenerateOAuthClientSecret();
  const titleId = useId();

  const [newSecret, setNewSecret] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!open) {
      setNewSecret(null);
      setCopied(false);
    }
  }, [open]);

  async function handleRegenerate() {
    if (!client) return;
    try {
      const res = await regenMutation.mutateAsync(client.id);
      setNewSecret(res.clientSecret);
      showToast({
        type: 'success',
        title: t('admin.oauthClients.toast.regenerateSuccess'),
        duration: 4000,
      });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('admin.oauthClients.toast.regenerateError'),
        message: err instanceof Error ? err.message : String(err),
      });
    }
  }

  async function handleCopy() {
    if (!newSecret) return;
    await copyToClipboard(newSecret);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  return (
    <Modal
      open={open}
      title={t('admin.oauthClients.actions.regenerateSecret')}
      onClose={onClose}
      labelledById={titleId}
      footer={
        newSecret ? (
          <button type="button" className="ppt-oc-btn ppt-oc-btn--primary" onClick={onClose}>
            Done
          </button>
        ) : (
          <>
            <button
              type="button"
              className="ppt-oc-btn ppt-oc-btn--secondary"
              onClick={onClose}
              disabled={regenMutation.isPending}
            >
              Cancel
            </button>
            <button
              type="button"
              className="ppt-oc-btn ppt-oc-btn--danger"
              onClick={handleRegenerate}
              disabled={regenMutation.isPending}
            >
              {regenMutation.isPending && (
                <span className="ppt-oc-spinner ppt-oc-spinner--dark" aria-hidden="true" />
              )}
              Regenerate Secret
            </button>
          </>
        )
      }
    >
      {newSecret ? (
        <>
          <div className="ppt-oc-secret-warning">
            Copy the new secret now — it will not be shown again.
          </div>
          <div className="ppt-oc-secret-box">
            <span className="ppt-oc-secret-label">New Client Secret</span>
            <span className="ppt-oc-secret-value">{newSecret}</span>
            <button
              type="button"
              className="ppt-oc-btn ppt-oc-btn--secondary ppt-oc-copy-btn"
              onClick={handleCopy}
            >
              {copied ? 'Copied' : 'Copy'}
            </button>
          </div>
        </>
      ) : (
        <p style={{ fontSize: 14, color: 'var(--ppt-fg-secondary,#374151)', margin: 0 }}>
          This will invalidate the current secret for <strong>{client?.name}</strong>. Any
          integrations using the old secret will stop working immediately.
        </p>
      )}
    </Modal>
  );
}

// ============================================================
// ClientCard
// ============================================================

interface ClientCardProps {
  client: OAuthClientSummary;
  onEdit: (c: OAuthClientSummary) => void;
  onRevoke: (c: OAuthClientSummary) => void;
  onRegenerate: (c: OAuthClientSummary) => void;
}

function ClientCard({ client, onEdit, onRevoke, onRegenerate }: ClientCardProps) {
  const { t } = useTranslation();
  const isRevoked = !client.isActive;
  return (
    <div className={`ppt-oc-card${isRevoked ? ' ppt-oc-card--revoked' : ''}`}>
      <div className="ppt-oc-card-top">
        <div className="ppt-oc-card-info">
          <p className="ppt-oc-card-name">{client.name}</p>
          {client.description && <p className="ppt-oc-card-desc">{client.description}</p>}
          <div className="ppt-oc-card-meta">
            <span className="ppt-oc-client-id">ID: {client.clientId}</span>
            <span>Registered: {formatDate(client.createdAt)}</span>
          </div>
        </div>
        <div style={{ display: 'flex', flexDirection: 'column', alignItems: 'flex-end', gap: 8 }}>
          <span
            className={`ppt-oc-badge ${isRevoked ? 'ppt-oc-badge--revoked' : 'ppt-oc-badge--active'}`}
          >
            {isRevoked ? 'REVOKED' : 'ACTIVE'}
          </span>
          <div className="ppt-oc-actions">
            <button
              type="button"
              className="ppt-oc-btn ppt-oc-btn--secondary"
              onClick={() => onEdit(client)}
              disabled={isRevoked}
              title={t('admin.oauthClients.actions.edit')}
            >
              Edit
            </button>
            <button
              type="button"
              className="ppt-oc-btn ppt-oc-btn--secondary"
              onClick={() => onRegenerate(client)}
              disabled={isRevoked}
              title={t('admin.oauthClients.actions.regenerateSecret')}
            >
              Regen Secret
            </button>
            <button
              type="button"
              className="ppt-oc-btn ppt-oc-btn--danger"
              onClick={() => onRevoke(client)}
              disabled={isRevoked}
              title={t('admin.oauthClients.actions.revoke')}
            >
              Revoke
            </button>
          </div>
        </div>
      </div>
      {client.scopes.length > 0 && (
        <div className="ppt-oc-scopes">
          {client.scopes.map((scope) => (
            <span key={scope} className="ppt-oc-scope-chip">
              {scope}
            </span>
          ))}
        </div>
      )}
    </div>
  );
}

// ============================================================
// OAuthClientsPage
// ============================================================

export default function OAuthClientsPage() {
  ensureStyles();
  const { t } = useTranslation();
  const { showToast } = useToast();
  const { data: clients, isLoading, isError, error, refetch } = useOAuthClients();
  const revokeMutation = useRevokeOAuthClient();

  const [showRegister, setShowRegister] = useState(false);
  const [editingClient, setEditingClient] = useState<OAuthClientSummary | null>(null);
  const [revokingClient, setRevokingClient] = useState<OAuthClientSummary | null>(null);
  const [regeneratingClient, setRegeneratingClient] = useState<OAuthClientSummary | null>(null);

  const handleRevoke = useCallback(async () => {
    if (!revokingClient) return;
    try {
      await revokeMutation.mutateAsync(revokingClient.id);
      showToast({
        type: 'success',
        title: t('admin.oauthClients.toast.revokeSuccess'),
        message: revokingClient.name,
        duration: 3000,
      });
      setRevokingClient(null);
    } catch (err) {
      showToast({
        type: 'error',
        title: t('admin.oauthClients.toast.revokeError'),
        message: err instanceof Error ? err.message : String(err),
      });
    }
  }, [revokingClient, revokeMutation, showToast, t]);

  return (
    <div className="ppt-oc-page">
      <div className="ppt-oc-header">
        <h1 className="ppt-oc-title" style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          {t('admin.oauthClients.title')}
          <HelpTooltip text={t('admin.oauthClients.helpTooltip')} />
        </h1>
        <button
          type="button"
          className="ppt-oc-btn ppt-oc-btn--primary"
          onClick={() => setShowRegister(true)}
        >
          {t('admin.oauthClients.actions.register')}
        </button>
      </div>

      {isLoading && <div className="ppt-oc-status">Loading…</div>}

      {isError && (
        <div className="ppt-oc-status">
          <p>
            {t('admin.oauthClients.loadError', {
              message: error instanceof Error ? error.message : String(error),
            })}
          </p>
          <button
            type="button"
            className="ppt-oc-btn ppt-oc-btn--secondary ppt-oc-retry"
            onClick={() => void refetch()}
          >
            Retry
          </button>
        </div>
      )}

      {!isLoading && !isError && clients?.length === 0 && (
        <div className="ppt-oc-empty">
          <p className="ppt-oc-empty-text">{t('admin.oauthClients.empty')}</p>
          <button
            type="button"
            className="ppt-oc-btn ppt-oc-btn--primary"
            onClick={() => setShowRegister(true)}
          >
            {t('admin.oauthClients.actions.registerFirst')}
          </button>
        </div>
      )}

      {!isLoading && !isError && clients && clients.length > 0 && (
        <div className="ppt-oc-list">
          {clients.map((client) => (
            <ClientCard
              key={client.id}
              client={client}
              onEdit={setEditingClient}
              onRevoke={setRevokingClient}
              onRegenerate={setRegeneratingClient}
            />
          ))}
        </div>
      )}

      <RegisterDialog open={showRegister} onClose={() => setShowRegister(false)} />
      <EditDialog
        open={editingClient !== null}
        client={editingClient}
        onClose={() => setEditingClient(null)}
      />
      <RegenerateSecretDialog
        open={regeneratingClient !== null}
        client={regeneratingClient}
        onClose={() => setRegeneratingClient(null)}
      />
      <DestructiveConfirmDialog
        open={revokingClient !== null}
        title={t('admin.oauthClients.revokeDialog.title')}
        body={
          <span>
            {t('admin.oauthClients.revokeDialog.body', { name: revokingClient?.name ?? '' })}
          </span>
        }
        confirmText={revokingClient?.name ?? ''}
        confirmLabel={t('admin.oauthClients.revokeDialog.confirmLabel')}
        delayMs={2000}
        onConfirm={handleRevoke}
        onCancel={() => setRevokingClient(null)}
      />
    </div>
  );
}
