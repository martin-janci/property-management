/**
 * Epic 10A-2 — `/admin/identity/oauth-clients` page.
 *
 * OAuth Client Management console for platform admins. Provides:
 *   - List all registered OAuth clients (GET /api/v1/admin/oauth/clients)
 *   - Register new client with name, description, redirect URIs, scopes,
 *     confidential flag (POST /api/v1/admin/oauth/clients)
 *   - Edit client name / description / redirect URIs / scopes / active flag
 *     (PATCH /api/v1/admin/oauth/clients/{id})
 *   - Revoke (deactivate) a client with destructive-confirm dialog
 *     (DELETE /api/v1/admin/oauth/clients/{id})
 *   - Regenerate client secret — shown plaintext exactly once
 *     (POST /api/v1/admin/oauth/clients/{id}/regenerate-secret)
 *
 * Capability required: `oauth_client_write`
 */

import {
  KNOWN_OAUTH_SCOPES,
  type KnownOAuthScope,
  type OAuthClientSummary,
  type RegisterOAuthClientRequest,
  type UpdateOAuthClientRequest,
  useOAuthClients,
  useRegenerateOAuthClientSecret,
  useRegisterOAuthClient,
  useRevokeOAuthClient,
  useUpdateOAuthClient,
} from '@ppt/api-client';
import type React from 'react';
import { useCallback, useId, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { DestructiveConfirmDialog } from '../components/DestructiveConfirmDialog';
import { useToast } from '../components/Toast';
import { useFocusTrap } from '../components/useFocusTrap';

// ---------------------------------------------------------------------------
// Scope selector helper
// ---------------------------------------------------------------------------

const SCOPE_DESCRIPTIONS: Record<KnownOAuthScope, string> = {
  profile: 'Basic profile (name, avatar)',
  email: 'Email address',
  'org:read': 'Read-only organization data',
  full: 'Full account access',
};

interface ScopeSelectorProps {
  value: string[];
  onChange: (scopes: string[]) => void;
  disabled?: boolean;
}

function ScopeSelector({ value, onChange, disabled }: ScopeSelectorProps) {
  const toggle = (scope: string) => {
    if (disabled) return;
    if (value.includes(scope)) {
      onChange(value.filter((s) => s !== scope));
    } else {
      onChange([...value, scope]);
    }
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      {KNOWN_OAUTH_SCOPES.map((scope) => {
        const checked = value.includes(scope);
        return (
          <label
            key={scope}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 10,
              cursor: disabled ? 'not-allowed' : 'pointer',
              opacity: disabled ? 0.6 : 1,
              padding: '6px 10px',
              borderRadius: 6,
              border: '1px solid var(--ppt-border-default, #e5e7eb)',
              background: checked
                ? 'var(--ppt-accent-soft-bg, #eff6ff)'
                : 'var(--ppt-bg-surface, #fff)',
              transition: 'background 120ms ease',
            }}
          >
            <input
              type="checkbox"
              checked={checked}
              onChange={() => toggle(scope)}
              disabled={disabled}
              style={{ width: 16, height: 16, flexShrink: 0 }}
            />
            <span style={{ flex: 1 }}>
              <span
                style={{
                  fontFamily: 'var(--ppt-font-mono, monospace)',
                  fontSize: 12,
                  fontWeight: 600,
                  color: 'var(--ppt-brand-700, #1d4ed8)',
                }}
              >
                {scope}
              </span>
              <span
                style={{
                  fontSize: 12,
                  color: 'var(--ppt-fg-muted, #6b7280)',
                  marginLeft: 8,
                }}
              >
                — {SCOPE_DESCRIPTIONS[scope]}
              </span>
            </span>
          </label>
        );
      })}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Redirect URI editor
// ---------------------------------------------------------------------------

interface RedirectUriEditorProps {
  value: string[];
  onChange: (uris: string[]) => void;
  disabled?: boolean;
}

function RedirectUriEditor({ value, onChange, disabled }: RedirectUriEditorProps) {
  const [newUri, setNewUri] = useState('');

  const addUri = () => {
    const trimmed = newUri.trim();
    if (!trimmed || value.includes(trimmed)) return;
    onChange([...value, trimmed]);
    setNewUri('');
  };

  const removeUri = (uri: string) => {
    onChange(value.filter((u) => u !== uri));
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
      {value.map((uri) => (
        <div
          key={uri}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            padding: '4px 8px',
            borderRadius: 6,
            background: 'var(--ppt-bg-subtle, #f3f4f6)',
            fontSize: 12,
            fontFamily: 'var(--ppt-font-mono, monospace)',
          }}
        >
          <span style={{ flex: 1, wordBreak: 'break-all', color: 'var(--ppt-fg-primary, #111827)' }}>
            {uri}
          </span>
          {!disabled && (
            <button
              type="button"
              onClick={() => removeUri(uri)}
              style={{
                background: 'none',
                border: 'none',
                cursor: 'pointer',
                color: 'var(--ppt-danger-600, #dc2626)',
                fontSize: 14,
                padding: '0 2px',
                lineHeight: 1,
              }}
              aria-label={`Remove ${uri}`}
            >
              ×
            </button>
          )}
        </div>
      ))}
      {!disabled && (
        <div style={{ display: 'flex', gap: 6 }}>
          <input
            type="url"
            value={newUri}
            onChange={(e) => setNewUri(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                addUri();
              }
            }}
            placeholder="https://example.com/callback"
            style={{
              flex: 1,
              fontSize: 12,
              padding: '6px 8px',
              border: '1px solid var(--ppt-border-default, #e5e7eb)',
              borderRadius: 6,
              background: 'var(--ppt-bg-input, #fff)',
              color: 'var(--ppt-fg-primary, #111827)',
              outline: 'none',
              fontFamily: 'var(--ppt-font-mono, monospace)',
            }}
          />
          <button
            type="button"
            onClick={addUri}
            style={{
              padding: '6px 12px',
              borderRadius: 6,
              border: '1px solid var(--ppt-border-default, #e5e7eb)',
              background: 'transparent',
              cursor: 'pointer',
              fontSize: 12,
              fontWeight: 500,
              color: 'var(--ppt-fg-secondary, #374151)',
            }}
          >
            Add
          </button>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Form field component
// ---------------------------------------------------------------------------

interface FormFieldProps {
  label: string;
  required?: boolean;
  error?: string;
  children: React.ReactNode;
  htmlFor?: string;
}

function FormField({ label, required, error, children, htmlFor }: FormFieldProps) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
      <label
        htmlFor={htmlFor}
        style={{
          fontSize: 13,
          fontWeight: 500,
          color: 'var(--ppt-fg-secondary, #374151)',
        }}
      >
        {label}
        {required && (
          <span
            style={{ color: 'var(--ppt-danger-600, #dc2626)', marginLeft: 3 }}
            aria-hidden="true"
          >
            *
          </span>
        )}
      </label>
      {children}
      {error && (
        <span style={{ fontSize: 11, color: 'var(--ppt-danger-600, #dc2626)' }}>{error}</span>
      )}
    </div>
  );
}

const INPUT_STYLE: React.CSSProperties = {
  fontSize: 13,
  padding: '7px 10px',
  border: '1px solid var(--ppt-border-default, #e5e7eb)',
  borderRadius: 8,
  background: 'var(--ppt-bg-input, #fff)',
  color: 'var(--ppt-fg-primary, #111827)',
  outline: 'none',
  width: '100%',
  boxSizing: 'border-box',
};

// ---------------------------------------------------------------------------
// Register client dialog
// ---------------------------------------------------------------------------

interface RegisterDialogState {
  name: string;
  description: string;
  redirectUris: string[];
  scopes: string[];
  isConfidential: boolean;
  rotateRefreshTokens: boolean;
}

const DEFAULT_REGISTER_STATE: RegisterDialogState = {
  name: '',
  description: '',
  redirectUris: [],
  scopes: ['profile'],
  isConfidential: true,
  rotateRefreshTokens: false,
};

interface RegisterDialogProps {
  onClose: () => void;
  onSuccess: (clientId: string, clientSecret: string) => void;
}

function RegisterDialog({ onClose, onSuccess }: RegisterDialogProps) {
  const dialogRef = useRef<HTMLDivElement | null>(null);
  const titleId = useId();
  const [form, setForm] = useState<RegisterDialogState>(DEFAULT_REGISTER_STATE);
  const [errors, setErrors] = useState<Partial<Record<keyof RegisterDialogState, string>>>({});
  const { mutateAsync, isPending } = useRegisterOAuthClient();

  useFocusTrap(dialogRef, () => {
    if (!isPending) onClose();
  });

  const validate = (): boolean => {
    const e: Partial<Record<keyof RegisterDialogState, string>> = {};
    if (!form.name.trim()) e.name = 'Name is required';
    if (form.redirectUris.length === 0) e.redirectUris = 'At least one redirect URI is required';
    if (form.scopes.length === 0) e.scopes = 'At least one scope is required';
    setErrors(e);
    return Object.keys(e).length === 0;
  };

  const handleSubmit = async () => {
    if (!validate()) return;
    const req: RegisterOAuthClientRequest = {
      name: form.name.trim(),
      description: form.description.trim() || undefined,
      redirectUris: form.redirectUris,
      scopes: form.scopes,
      isConfidential: form.isConfidential,
      rotateRefreshTokens: form.rotateRefreshTokens,
    };
    try {
      const res = await mutateAsync(req);
      onSuccess(res.clientId, res.clientSecret);
    } catch {
      // errors shown by caller toast
    }
  };

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 900,
        background: 'rgba(0,0,0,0.45)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: 16,
        overflow: 'auto',
      }}
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        style={{
          maxWidth: 560,
          width: '100%',
          background: 'var(--ppt-bg-surface, #fff)',
          borderRadius: 'var(--ppt-radius-lg, 12px)',
          padding: 24,
          display: 'flex',
          flexDirection: 'column',
          gap: 16,
          boxShadow: 'var(--ppt-shadow-modal, 0 10px 40px rgba(0,0,0,0.15))',
          maxHeight: '90vh',
          overflowY: 'auto',
        }}
      >
        <h2
          id={titleId}
          style={{ margin: 0, fontSize: 16, fontWeight: 600, color: 'var(--ppt-fg-primary, #111827)' }}
        >
          Register OAuth Client
        </h2>

        <FormField label="Name" required htmlFor="reg-name" error={errors.name}>
          <input
            id="reg-name"
            type="text"
            value={form.name}
            onChange={(e) => setForm((f) => ({ ...f, name: e.target.value }))}
            disabled={isPending}
            style={INPUT_STYLE}
            placeholder="My Integration App"
          />
        </FormField>

        <FormField label="Description" htmlFor="reg-desc">
          <textarea
            id="reg-desc"
            value={form.description}
            onChange={(e) => setForm((f) => ({ ...f, description: e.target.value }))}
            disabled={isPending}
            rows={2}
            style={{ ...INPUT_STYLE, resize: 'vertical', fontFamily: 'inherit' }}
            placeholder="Optional description of this OAuth client"
          />
        </FormField>

        <FormField label="Redirect URIs" required error={errors.redirectUris}>
          <RedirectUriEditor
            value={form.redirectUris}
            onChange={(uris) => setForm((f) => ({ ...f, redirectUris: uris }))}
            disabled={isPending}
          />
        </FormField>

        <FormField label="Scopes" required error={errors.scopes}>
          <ScopeSelector
            value={form.scopes}
            onChange={(scopes) => setForm((f) => ({ ...f, scopes }))}
            disabled={isPending}
          />
        </FormField>

        <FormField label="Client type">
          <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
            <label
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 8,
                cursor: isPending ? 'not-allowed' : 'pointer',
                fontSize: 13,
                color: 'var(--ppt-fg-secondary, #374151)',
              }}
            >
              <input
                type="checkbox"
                checked={form.isConfidential}
                onChange={(e) => setForm((f) => ({ ...f, isConfidential: e.target.checked }))}
                disabled={isPending}
              />
              Confidential client (server-side app, can keep a secret)
            </label>
            <label
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 8,
                cursor: isPending ? 'not-allowed' : 'pointer',
                fontSize: 13,
                color: 'var(--ppt-fg-secondary, #374151)',
              }}
            >
              <input
                type="checkbox"
                checked={form.rotateRefreshTokens}
                onChange={(e) => setForm((f) => ({ ...f, rotateRefreshTokens: e.target.checked }))}
                disabled={isPending}
              />
              Rotate refresh tokens on use
            </label>
          </div>
        </FormField>

        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 10, paddingTop: 4 }}>
          <button
            type="button"
            onClick={onClose}
            disabled={isPending}
            style={{
              padding: '7px 14px',
              borderRadius: 8,
              border: '1px solid var(--ppt-border-default, #e5e7eb)',
              background: 'transparent',
              cursor: 'pointer',
              fontSize: 13,
              fontWeight: 500,
            }}
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={() => {
              void handleSubmit();
            }}
            disabled={isPending}
            style={{
              padding: '7px 14px',
              borderRadius: 8,
              border: 'none',
              background: 'var(--ppt-brand-600, #2563eb)',
              color: '#fff',
              cursor: 'pointer',
              fontSize: 13,
              fontWeight: 500,
              opacity: isPending ? 0.6 : 1,
            }}
          >
            {isPending ? 'Registering…' : 'Register client'}
          </button>
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// New secret display dialog (shown once after register / regenerate)
// ---------------------------------------------------------------------------

interface SecretDisplayDialogProps {
  clientId: string;
  clientSecret: string;
  onClose: () => void;
  title?: string;
}

function SecretDisplayDialog({
  clientId,
  clientSecret,
  onClose,
  title = 'Client registered',
}: SecretDisplayDialogProps) {
  const dialogRef = useRef<HTMLDivElement | null>(null);
  const titleId = useId();
  const [copied, setCopied] = useState<'id' | 'secret' | null>(null);

  useFocusTrap(dialogRef, () => {
    onClose();
  });

  const copy = (text: string, field: 'id' | 'secret') => {
    void navigator.clipboard.writeText(text).then(() => {
      setCopied(field);
      setTimeout(() => setCopied(null), 2000);
    });
  };

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 950,
        background: 'rgba(0,0,0,0.55)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: 16,
      }}
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        style={{
          maxWidth: 520,
          width: '100%',
          background: 'var(--ppt-bg-surface, #fff)',
          borderRadius: 'var(--ppt-radius-lg, 12px)',
          padding: 24,
          display: 'flex',
          flexDirection: 'column',
          gap: 16,
          boxShadow: 'var(--ppt-shadow-modal, 0 10px 40px rgba(0,0,0,0.15))',
        }}
      >
        <h2
          id={titleId}
          style={{
            margin: 0,
            fontSize: 16,
            fontWeight: 600,
            color: 'var(--ppt-fg-primary, #111827)',
          }}
        >
          {title}
        </h2>

        <div
          style={{
            padding: '12px 14px',
            background: 'var(--ppt-warning-50, #fffbeb)',
            border: '1px solid var(--ppt-warning-300, #fcd34d)',
            borderRadius: 8,
            fontSize: 13,
            color: 'var(--ppt-warning-800, #92400e)',
          }}
        >
          <strong>Save the client secret now.</strong> It will not be shown again after you close
          this dialog.
        </div>

        {[
          { label: 'Client ID', value: clientId, field: 'id' as const },
          { label: 'Client Secret', value: clientSecret, field: 'secret' as const },
        ].map(({ label, value, field }) => (
          <div key={field} style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
            <span
              style={{
                fontSize: 12,
                fontWeight: 600,
                color: 'var(--ppt-fg-muted, #6b7280)',
                textTransform: 'uppercase',
                letterSpacing: '0.04em',
              }}
            >
              {label}
            </span>
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 8,
                padding: '8px 10px',
                background: 'var(--ppt-bg-subtle, #f3f4f6)',
                borderRadius: 8,
                border: '1px solid var(--ppt-border-default, #e5e7eb)',
              }}
            >
              <code
                style={{
                  flex: 1,
                  fontFamily: 'var(--ppt-font-mono, monospace)',
                  fontSize: 12,
                  color: 'var(--ppt-fg-primary, #111827)',
                  wordBreak: 'break-all',
                  userSelect: 'all',
                }}
              >
                {value}
              </code>
              <button
                type="button"
                onClick={() => copy(value, field)}
                style={{
                  flexShrink: 0,
                  padding: '4px 8px',
                  borderRadius: 6,
                  border: '1px solid var(--ppt-border-default, #e5e7eb)',
                  background: 'var(--ppt-bg-surface, #fff)',
                  cursor: 'pointer',
                  fontSize: 11,
                  fontWeight: 500,
                  color:
                    copied === field
                      ? 'var(--ppt-success-700, #15803d)'
                      : 'var(--ppt-fg-secondary, #374151)',
                }}
              >
                {copied === field ? 'Copied!' : 'Copy'}
              </button>
            </div>
          </div>
        ))}

        <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
          <button
            type="button"
            onClick={onClose}
            style={{
              padding: '7px 18px',
              borderRadius: 8,
              border: 'none',
              background: 'var(--ppt-brand-600, #2563eb)',
              color: '#fff',
              cursor: 'pointer',
              fontSize: 13,
              fontWeight: 500,
            }}
          >
            Done — I saved the secret
          </button>
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Edit client dialog
// ---------------------------------------------------------------------------

interface EditDialogProps {
  client: OAuthClientSummary;
  onClose: () => void;
}

function EditDialog({ client, onClose }: EditDialogProps) {
  const dialogRef = useRef<HTMLDivElement | null>(null);
  const titleId = useId();
  const { showToast } = useToast();

  // Initialise form from existing client values.
  // OAuthClientSummary doesn't expose redirectUris, so we can only edit
  // scopes, name, description, and active status in this view.
  const [name, setName] = useState(client.name);
  const [description, setDescription] = useState(client.description ?? '');
  const [scopes, setScopes] = useState<string[]>(client.scopes);
  const [isActive, setIsActive] = useState(client.isActive);
  const [nameError, setNameError] = useState('');

  const { mutateAsync, isPending } = useUpdateOAuthClient(client.id);

  useFocusTrap(dialogRef, () => {
    if (!isPending) onClose();
  });

  const handleSave = async () => {
    if (!name.trim()) {
      setNameError('Name is required');
      return;
    }
    setNameError('');
    const update: UpdateOAuthClientRequest = {
      name: name.trim(),
      description: description.trim() || undefined,
      scopes,
      isActive,
    };
    try {
      await mutateAsync(update);
      showToast({ type: 'success', title: 'Client updated', message: `${name} has been updated.` });
      onClose();
    } catch (err) {
      showToast({
        type: 'error',
        title: 'Update failed',
        message: err instanceof Error ? err.message : 'Unknown error',
      });
    }
  };

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 900,
        background: 'rgba(0,0,0,0.45)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        padding: 16,
        overflow: 'auto',
      }}
    >
      <div
        ref={dialogRef}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabIndex={-1}
        style={{
          maxWidth: 560,
          width: '100%',
          background: 'var(--ppt-bg-surface, #fff)',
          borderRadius: 'var(--ppt-radius-lg, 12px)',
          padding: 24,
          display: 'flex',
          flexDirection: 'column',
          gap: 16,
          boxShadow: 'var(--ppt-shadow-modal, 0 10px 40px rgba(0,0,0,0.15))',
          maxHeight: '90vh',
          overflowY: 'auto',
        }}
      >
        <h2
          id={titleId}
          style={{ margin: 0, fontSize: 16, fontWeight: 600, color: 'var(--ppt-fg-primary, #111827)' }}
        >
          Edit OAuth Client
        </h2>

        <div
          style={{
            fontSize: 12,
            color: 'var(--ppt-fg-muted, #6b7280)',
            padding: '6px 10px',
            background: 'var(--ppt-bg-subtle, #f3f4f6)',
            borderRadius: 6,
            fontFamily: 'var(--ppt-font-mono, monospace)',
          }}
        >
          {client.clientId}
        </div>

        <FormField label="Name" required htmlFor="edit-name" error={nameError}>
          <input
            id="edit-name"
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            disabled={isPending}
            style={INPUT_STYLE}
          />
        </FormField>

        <FormField label="Description" htmlFor="edit-desc">
          <textarea
            id="edit-desc"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            disabled={isPending}
            rows={2}
            style={{ ...INPUT_STYLE, resize: 'vertical', fontFamily: 'inherit' }}
          />
        </FormField>

        <FormField label="Scopes">
          <ScopeSelector value={scopes} onChange={setScopes} disabled={isPending} />
        </FormField>

        <FormField label="Status">
          <label
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 8,
              cursor: isPending ? 'not-allowed' : 'pointer',
              fontSize: 13,
              color: 'var(--ppt-fg-secondary, #374151)',
            }}
          >
            <input
              type="checkbox"
              checked={isActive}
              onChange={(e) => setIsActive(e.target.checked)}
              disabled={isPending}
            />
            Active (uncheck to deactivate without revoking)
          </label>
        </FormField>

        <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 10, paddingTop: 4 }}>
          <button
            type="button"
            onClick={onClose}
            disabled={isPending}
            style={{
              padding: '7px 14px',
              borderRadius: 8,
              border: '1px solid var(--ppt-border-default, #e5e7eb)',
              background: 'transparent',
              cursor: 'pointer',
              fontSize: 13,
              fontWeight: 500,
            }}
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={() => {
              void handleSave();
            }}
            disabled={isPending}
            style={{
              padding: '7px 14px',
              borderRadius: 8,
              border: 'none',
              background: 'var(--ppt-brand-600, #2563eb)',
              color: '#fff',
              cursor: 'pointer',
              fontSize: 13,
              fontWeight: 500,
              opacity: isPending ? 0.6 : 1,
            }}
          >
            {isPending ? 'Saving…' : 'Save changes'}
          </button>
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Client row card
// ---------------------------------------------------------------------------

interface ClientCardProps {
  client: OAuthClientSummary;
  onEdit: () => void;
  onRevoke: () => void;
  onRegenerate: () => void;
}

function ClientCard({ client, onEdit, onRevoke, onRegenerate }: ClientCardProps) {
  const { t } = useTranslation();

  return (
    <div
      style={{
        border: '1px solid var(--ppt-border-default, #e5e7eb)',
        borderRadius: 'var(--ppt-radius-lg, 12px)',
        padding: '16px 20px',
        background: client.isActive
          ? 'var(--ppt-bg-surface, #fff)'
          : 'var(--ppt-bg-subtle, #f3f4f6)',
        display: 'flex',
        flexDirection: 'column',
        gap: 10,
        opacity: client.isActive ? 1 : 0.75,
      }}
    >
      {/* Header row */}
      <div style={{ display: 'flex', alignItems: 'flex-start', gap: 10 }}>
        <div style={{ flex: 1 }}>
          <div
            style={{
              fontSize: 15,
              fontWeight: 600,
              color: 'var(--ppt-fg-primary, #111827)',
              display: 'flex',
              alignItems: 'center',
              gap: 8,
            }}
          >
            {client.name}
            {!client.isActive && (
              <span
                style={{
                  fontSize: 10,
                  fontWeight: 700,
                  padding: '2px 6px',
                  borderRadius: 4,
                  background: 'var(--ppt-danger-100, #fee2e2)',
                  color: 'var(--ppt-danger-700, #b91c1c)',
                  textTransform: 'uppercase',
                  letterSpacing: '0.05em',
                }}
              >
                Revoked
              </span>
            )}
          </div>
          {client.description && (
            <div
              style={{
                fontSize: 12,
                color: 'var(--ppt-fg-muted, #6b7280)',
                marginTop: 2,
              }}
            >
              {client.description}
            </div>
          )}
        </div>
        <div
          style={{
            fontSize: 11,
            color: 'var(--ppt-fg-muted, #6b7280)',
            flexShrink: 0,
            paddingTop: 2,
          }}
        >
          {new Date(client.createdAt).toLocaleDateString()}
        </div>
      </div>

      {/* Client ID */}
      <div
        style={{
          fontFamily: 'var(--ppt-font-mono, monospace)',
          fontSize: 11,
          color: 'var(--ppt-fg-muted, #6b7280)',
          background: 'var(--ppt-bg-subtle, #f3f4f6)',
          padding: '4px 8px',
          borderRadius: 6,
          wordBreak: 'break-all',
        }}
      >
        {client.clientId}
      </div>

      {/* Scopes */}
      <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4 }}>
        {client.scopes.map((scope) => (
          <span
            key={scope}
            style={{
              fontSize: 11,
              padding: '2px 6px',
              borderRadius: 4,
              background: 'var(--ppt-accent-soft-bg, #eff6ff)',
              color: 'var(--ppt-brand-700, #1d4ed8)',
              fontFamily: 'var(--ppt-font-mono, monospace)',
              fontWeight: 500,
            }}
          >
            {scope}
          </span>
        ))}
      </div>

      {/* Actions */}
      <div
        style={{
          display: 'flex',
          gap: 8,
          paddingTop: 4,
          borderTop: '1px solid var(--ppt-border-default, #e5e7eb)',
        }}
      >
        <button
          type="button"
          onClick={onEdit}
          style={{
            padding: '5px 12px',
            borderRadius: 6,
            border: '1px solid var(--ppt-border-default, #e5e7eb)',
            background: 'transparent',
            cursor: 'pointer',
            fontSize: 12,
            fontWeight: 500,
            color: 'var(--ppt-fg-secondary, #374151)',
          }}
        >
          {t('admin.oauthClients.actions.edit', 'Edit')}
        </button>
        <button
          type="button"
          onClick={onRegenerate}
          disabled={!client.isActive}
          style={{
            padding: '5px 12px',
            borderRadius: 6,
            border: '1px solid var(--ppt-border-default, #e5e7eb)',
            background: 'transparent',
            cursor: client.isActive ? 'pointer' : 'not-allowed',
            fontSize: 12,
            fontWeight: 500,
            color: 'var(--ppt-fg-secondary, #374151)',
            opacity: client.isActive ? 1 : 0.5,
          }}
        >
          {t('admin.oauthClients.actions.regenerateSecret', 'Regenerate secret')}
        </button>
        {client.isActive && (
          <button
            type="button"
            onClick={onRevoke}
            style={{
              padding: '5px 12px',
              borderRadius: 6,
              border: '1px solid var(--ppt-danger-300, #fca5a5)',
              background: 'transparent',
              cursor: 'pointer',
              fontSize: 12,
              fontWeight: 500,
              color: 'var(--ppt-danger-600, #dc2626)',
              marginLeft: 'auto',
            }}
          >
            {t('admin.oauthClients.actions.revoke', 'Revoke')}
          </button>
        )}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Page
// ---------------------------------------------------------------------------

const OAuthClientsPage: React.FC = () => {
  const { t } = useTranslation();
  const { showToast } = useToast();

  const { data: clients, isLoading, isError, error, refetch } = useOAuthClients();
  const revokeClient = useRevokeOAuthClient();
  const regenerateSecret = useRegenerateOAuthClientSecret();

  // Dialog state
  const [showRegister, setShowRegister] = useState(false);
  const [newSecret, setNewSecret] = useState<{
    clientId: string;
    secret: string;
    title: string;
  } | null>(null);
  const [editTarget, setEditTarget] = useState<OAuthClientSummary | null>(null);
  const [revokeTarget, setRevokeTarget] = useState<OAuthClientSummary | null>(null);
  const [regenerateTarget, setRegenerateTarget] = useState<OAuthClientSummary | null>(null);

  const handleRegisterSuccess = useCallback(
    (clientId: string, clientSecret: string) => {
      setShowRegister(false);
      setNewSecret({ clientId, secret: clientSecret, title: 'Client registered' });
      showToast({
        type: 'success',
        title: t('admin.oauthClients.toast.registeredTitle', 'Client registered'),
        message: t('admin.oauthClients.toast.registeredMessage', 'Save the client secret now.'),
      });
    },
    [showToast, t]
  );

  const handleRevoke = useCallback(
    async (client: OAuthClientSummary) => {
      try {
        await revokeClient.mutateAsync(client.id);
        setRevokeTarget(null);
        showToast({
          type: 'success',
          title: t('admin.oauthClients.toast.revokedTitle', 'Client revoked'),
          message: t('admin.oauthClients.toast.revokedMessage', '{{name}} has been revoked.', {
            name: client.name,
          }),
        });
      } catch (err) {
        showToast({
          type: 'error',
          title: t('admin.oauthClients.toast.revokeFailedTitle', 'Revoke failed'),
          message: err instanceof Error ? err.message : 'Unknown error',
        });
        throw err; // Let DestructiveConfirmDialog know to stay open
      }
    },
    [revokeClient, showToast, t]
  );

  const handleRegenerate = useCallback(
    async (client: OAuthClientSummary) => {
      try {
        const result = await regenerateSecret.mutateAsync(client.id);
        setRegenerateTarget(null);
        setNewSecret({
          clientId: client.clientId,
          secret: result.clientSecret,
          title: 'Secret regenerated',
        });
        showToast({
          type: 'success',
          title: t('admin.oauthClients.toast.secretRegeneratedTitle', 'Secret regenerated'),
          message: t(
            'admin.oauthClients.toast.secretRegeneratedMessage',
            'Save the new secret now.'
          ),
        });
      } catch (err) {
        showToast({
          type: 'error',
          title: t('admin.oauthClients.toast.regenerateFailedTitle', 'Regeneration failed'),
          message: err instanceof Error ? err.message : 'Unknown error',
        });
      }
    },
    [regenerateSecret, showToast, t]
  );

  return (
    <>
      {/* Register dialog */}
      {showRegister && (
        <RegisterDialog
          onClose={() => setShowRegister(false)}
          onSuccess={handleRegisterSuccess}
        />
      )}

      {/* New / regenerated secret display */}
      {newSecret && (
        <SecretDisplayDialog
          clientId={newSecret.clientId}
          clientSecret={newSecret.secret}
          title={newSecret.title}
          onClose={() => setNewSecret(null)}
        />
      )}

      {/* Edit dialog */}
      {editTarget && (
        <EditDialog client={editTarget} onClose={() => setEditTarget(null)} />
      )}

      {/* Regenerate confirm — simple confirm dialog before calling API */}
      {regenerateTarget && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            zIndex: 900,
            background: 'rgba(0,0,0,0.45)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            padding: 16,
          }}
        >
          <div
            role="dialog"
            aria-modal="true"
            style={{
              maxWidth: 440,
              width: '100%',
              background: 'var(--ppt-bg-surface, #fff)',
              borderRadius: 'var(--ppt-radius-lg, 12px)',
              padding: 24,
              display: 'flex',
              flexDirection: 'column',
              gap: 16,
              boxShadow: 'var(--ppt-shadow-modal, 0 10px 40px rgba(0,0,0,0.15))',
            }}
          >
            <h2 style={{ margin: 0, fontSize: 16, fontWeight: 600, color: 'var(--ppt-warning-800, #92400e)' }}>
              Regenerate client secret
            </h2>
            <p style={{ margin: 0, fontSize: 13, color: 'var(--ppt-fg-secondary, #374151)' }}>
              The current secret for <strong>{regenerateTarget.name}</strong> will be invalidated
              immediately. Any integration using the old secret will stop working. The new secret is
              shown only once.
            </p>
            <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 10 }}>
              <button
                type="button"
                onClick={() => setRegenerateTarget(null)}
                disabled={regenerateSecret.isPending}
                style={{
                  padding: '7px 14px',
                  borderRadius: 8,
                  border: '1px solid var(--ppt-border-default, #e5e7eb)',
                  background: 'transparent',
                  cursor: 'pointer',
                  fontSize: 13,
                  fontWeight: 500,
                }}
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={() => {
                  void handleRegenerate(regenerateTarget);
                }}
                disabled={regenerateSecret.isPending}
                style={{
                  padding: '7px 14px',
                  borderRadius: 8,
                  border: 'none',
                  background: 'var(--ppt-warning-600, #d97706)',
                  color: '#fff',
                  cursor: 'pointer',
                  fontSize: 13,
                  fontWeight: 500,
                  opacity: regenerateSecret.isPending ? 0.6 : 1,
                }}
              >
                {regenerateSecret.isPending ? 'Regenerating…' : 'Regenerate secret'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Revoke destructive confirm */}
      <DestructiveConfirmDialog
        open={revokeTarget !== null}
        title={t('admin.oauthClients.revokeDialog.title', 'Revoke OAuth client')}
        body={
          <span>
            {t(
              'admin.oauthClients.revokeDialog.body',
              'This will permanently deactivate client <strong>{{name}}</strong>. Active tokens issued to this client will continue to work until they expire, but no new tokens can be obtained.',
              { name: revokeTarget?.name ?? '' }
            )}
          </span>
        }
        confirmText={revokeTarget?.name ?? ''}
        confirmLabel={t(
          'admin.oauthClients.revokeDialog.confirmLabel',
          'Type the client name to confirm:'
        )}
        delayMs={2000}
        onConfirm={async () => {
          if (revokeTarget) await handleRevoke(revokeTarget);
        }}
        onCancel={() => setRevokeTarget(null)}
      />

      {/* Page body */}
      <section style={{ padding: '24px', maxWidth: 900 }}>
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            marginBottom: 20,
            gap: 12,
          }}
        >
          <h1 style={{ margin: 0, flex: 1, fontSize: 20, fontWeight: 700 }}>
            {t('admin.oauthClients.title', 'OAuth Clients')}
          </h1>
          <button
            type="button"
            onClick={() => setShowRegister(true)}
            style={{
              padding: '7px 14px',
              borderRadius: 8,
              border: 'none',
              background: 'var(--ppt-brand-600, #2563eb)',
              color: '#fff',
              cursor: 'pointer',
              fontSize: 13,
              fontWeight: 500,
            }}
          >
            {t('admin.oauthClients.actions.register', 'Register client')}
          </button>
        </div>

        <p style={{ margin: '0 0 24px', fontSize: 13, color: 'var(--ppt-fg-secondary, #374151)' }}>
          {t(
            'admin.oauthClients.description',
            'Manage third-party OAuth 2.0 clients that can obtain tokens on behalf of your users. Capability required: oauth_client_write.'
          )}
        </p>

        {isLoading && (
          <div
            role="status"
            aria-live="polite"
            style={{ color: 'var(--ppt-fg-muted, #6b7280)', fontSize: 13 }}
          >
            {t('admin.common.loading', 'Loading…')}
          </div>
        )}

        {isError && (
          <div role="alert" className="ppt-admin-error">
            <p style={{ color: 'var(--ppt-danger-700, #b91c1c)', fontSize: 13 }}>
              {t('admin.oauthClients.loadError', 'Failed to load OAuth clients: {{message}}', {
                message: error instanceof Error ? error.message : 'Unknown error',
              })}
            </p>
            <button
              type="button"
              onClick={() => {
                void refetch();
              }}
              style={{
                padding: '5px 12px',
                borderRadius: 6,
                border: '1px solid var(--ppt-border-default, #e5e7eb)',
                background: 'transparent',
                cursor: 'pointer',
                fontSize: 12,
              }}
            >
              {t('admin.common.retry', 'Retry')}
            </button>
          </div>
        )}

        {!isLoading && !isError && (
          <>
            {(clients ?? []).length === 0 ? (
              <div
                style={{
                  padding: '40px 20px',
                  textAlign: 'center',
                  color: 'var(--ppt-fg-muted, #6b7280)',
                  fontSize: 13,
                  border: '2px dashed var(--ppt-border-default, #e5e7eb)',
                  borderRadius: 12,
                }}
              >
                {t('admin.oauthClients.empty', 'No OAuth clients registered yet.')}{' '}
                <button
                  type="button"
                  onClick={() => setShowRegister(true)}
                  style={{
                    background: 'none',
                    border: 'none',
                    cursor: 'pointer',
                    color: 'var(--ppt-brand-600, #2563eb)',
                    fontSize: 13,
                    fontWeight: 500,
                    padding: 0,
                    textDecoration: 'underline',
                  }}
                >
                  {t('admin.oauthClients.actions.registerFirst', 'Register the first client')}
                </button>
              </div>
            ) : (
              <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
                {(clients ?? []).map((client) => (
                  <ClientCard
                    key={client.id}
                    client={client}
                    onEdit={() => setEditTarget(client)}
                    onRevoke={() => setRevokeTarget(client)}
                    onRegenerate={() => setRegenerateTarget(client)}
                  />
                ))}
              </div>
            )}
          </>
        )}
      </section>
    </>
  );
};

export default OAuthClientsPage;
