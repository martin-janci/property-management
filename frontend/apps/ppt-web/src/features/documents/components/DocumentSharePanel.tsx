/**
 * DocumentSharePanel (Story 7A.5)
 *
 * Web inline share panel for DocumentDetail.
 * Mirrors functionality of mobile DocumentShareSheet (PR #445).
 * Supports: user, role, building, link (+ password + expiry) share types.
 */

import {
  type CreateShareRequest,
  SHARE_TYPE,
  type ShareType,
  type ShareWithDocument,
  useCreateDocumentShare,
  useDocumentShares,
  useRevokeDocumentShare,
} from '@ppt/api-client';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ConfirmationDialog } from '../../../components/ConfirmationDialog';
import { useToast } from '../../../components/Toast';

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/** i18n key for the human-readable share type label. */
function shareTypeLabelKey(type: ShareType): string {
  switch (type) {
    case SHARE_TYPE.USER:
      return 'documents.share.type.user';
    case SHARE_TYPE.ROLE:
      return 'documents.share.type.role';
    case SHARE_TYPE.BUILDING:
      return 'documents.share.type.building';
    case SHARE_TYPE.LINK:
      return 'documents.share.type.link';
  }
}

function isActiveShare(share: ShareWithDocument): boolean {
  if (share.revoked_at) return false;
  if (share.expires_at && new Date(share.expires_at) < new Date()) return false;
  return true;
}

function fmtExpiry(d: string, locale: string): string {
  return new Date(d).toLocaleDateString(locale, {
    month: 'short',
    day: 'numeric',
    year: 'numeric',
  });
}

function CreateShareForm({
  documentId,
  onCreated,
}: {
  documentId: string;
  onCreated: (url?: string | null) => void;
}) {
  const { t } = useTranslation();
  const [shareType, setShareType] = useState<ShareType>(SHARE_TYPE.LINK);
  const [targetId, setTargetId] = useState('');
  const [targetIdTouched, setTargetIdTouched] = useState(false);
  const [targetRole, setTargetRole] = useState('');
  const [usePassword, setUsePassword] = useState(false);
  const [password, setPassword] = useState('');
  const [useExpiry, setUseExpiry] = useState(false);
  const [expiryDays, setExpiryDays] = useState('7');
  const [formError, setFormError] = useState<string | null>(null);
  const createMutation = useCreateDocumentShare(documentId);

  const handleCreate = useCallback(async () => {
    setFormError(null);
    if (shareType === SHARE_TYPE.USER) {
      const trimmed = targetId.trim();
      if (!trimmed) {
        setFormError(t('documents.share.errors.userIdRequired'));
        return;
      }
      if (!UUID_RE.test(trimmed)) {
        setFormError(t('documents.share.errors.userIdInvalid'));
        return;
      }
    }
    if (shareType === SHARE_TYPE.ROLE && !targetRole.trim()) {
      setFormError(t('documents.share.errors.roleRequired'));
      return;
    }
    if (usePassword && password.trim().length < 4) {
      setFormError(t('documents.share.errors.passwordTooShort'));
      return;
    }
    const req: CreateShareRequest = { share_type: shareType };
    if (shareType === SHARE_TYPE.USER) req.target_id = targetId.trim();
    if (shareType === SHARE_TYPE.ROLE) req.target_role = targetRole.trim();
    if (usePassword && password.trim()) req.password = password.trim();
    if (useExpiry) {
      const days = parseInt(expiryDays, 10);
      if (Number.isNaN(days) || days < 1) {
        setFormError(t('documents.share.errors.expiryTooShort'));
        return;
      }
      const expiry = new Date();
      expiry.setDate(expiry.getDate() + days);
      req.expires_at = expiry.toISOString();
    }
    try {
      const result = await createMutation.mutateAsync(req);
      setTargetId('');
      setTargetRole('');
      setPassword('');
      setUsePassword(false);
      setUseExpiry(false);
      onCreated(result.share_url);
    } catch (err) {
      setFormError(err instanceof Error ? err.message : t('documents.share.errors.createFailed'));
    }
  }, [
    shareType,
    targetId,
    targetRole,
    usePassword,
    password,
    useExpiry,
    expiryDays,
    createMutation,
    onCreated,
    t,
  ]);

  return (
    <div className="sp-form">
      <h4 className="sp-form-title">{t('documents.share.newShare')}</h4>
      <div className="sp-type-row">
        {(Object.values(SHARE_TYPE) as ShareType[]).map((type) => (
          <button
            key={type}
            type="button"
            className={`sp-chip${shareType === type ? ' active' : ''}`}
            onClick={() => {
              setShareType(type);
              setFormError(null);
            }}
          >
            {t(shareTypeLabelKey(type))}
          </button>
        ))}
      </div>
      {shareType === SHARE_TYPE.USER && (
        <div className="sp-field">
          <label className="sp-label" htmlFor="sp-user-id">
            {t('documents.share.userId')}
          </label>
          <input
            id="sp-user-id"
            className="sp-input"
            type="text"
            placeholder={t('documents.share.userIdPlaceholder')}
            value={targetId}
            onChange={(e) => setTargetId(e.target.value)}
            onBlur={() => setTargetIdTouched(true)}
            autoComplete="off"
            aria-invalid={
              targetIdTouched && targetId.trim() !== '' && !UUID_RE.test(targetId.trim())
            }
            aria-describedby="sp-user-id-hint"
          />
          {targetIdTouched && targetId.trim() !== '' && !UUID_RE.test(targetId.trim()) && (
            <span id="sp-user-id-hint" className="sp-error" role="alert">
              {t('documents.share.mustBeUuid')}
            </span>
          )}
        </div>
      )}
      {shareType === SHARE_TYPE.ROLE && (
        <div className="sp-field">
          <label className="sp-label" htmlFor="sp-role">
            {t('documents.share.roleName')}
          </label>
          <input
            id="sp-role"
            className="sp-input"
            type="text"
            placeholder={t('documents.share.roleNamePlaceholder')}
            value={targetRole}
            onChange={(e) => setTargetRole(e.target.value)}
            autoComplete="off"
          />
        </div>
      )}
      {shareType === SHARE_TYPE.LINK && (
        <div className="sp-toggle-row">
          <span className="sp-toggle-label">{t('documents.share.passwordProtect')}</span>
          <label className="sp-switch" aria-label={t('aria.enablePasswordProtection')}>
            <input
              type="checkbox"
              checked={usePassword}
              onChange={(e) => setUsePassword(e.target.checked)}
            />
            <span className="sp-switch-track" />
          </label>
        </div>
      )}
      {shareType === SHARE_TYPE.LINK && usePassword && (
        <div className="sp-field">
          <label className="sp-label" htmlFor="sp-password">
            {t('documents.share.password')}
          </label>
          <input
            id="sp-password"
            className="sp-input"
            type="password"
            placeholder={t('documents.share.passwordPlaceholder')}
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            autoComplete="new-password"
          />
        </div>
      )}
      <div className="sp-toggle-row">
        <span className="sp-toggle-label">{t('documents.share.setExpiry')}</span>
        <label className="sp-switch" aria-label={t('aria.enableExpiry')}>
          <input
            type="checkbox"
            checked={useExpiry}
            onChange={(e) => setUseExpiry(e.target.checked)}
          />
          <span className="sp-switch-track" />
        </label>
      </div>
      {useExpiry && (
        <div className="sp-expiry-row">
          <input
            className="sp-input sp-expiry-input"
            type="number"
            min="1"
            max="365"
            value={expiryDays}
            onChange={(e) => setExpiryDays(e.target.value)}
            aria-label={t('aria.expiryDays')}
          />
          <span className="sp-expiry-unit">
            {t('documents.share.days', { count: Number(expiryDays) || 0 })}
          </span>
        </div>
      )}
      {formError && <p className="sp-error">{formError}</p>}
      <button
        type="button"
        className="sp-create-btn"
        onClick={handleCreate}
        disabled={
          createMutation.isPending ||
          (shareType === SHARE_TYPE.USER &&
            (targetId.trim() === '' || !UUID_RE.test(targetId.trim())))
        }
      >
        {createMutation.isPending ? t('documents.share.creating') : t('documents.share.create')}
      </button>
    </div>
  );
}

function ShareList({ documentId, shares }: { documentId: string; shares: ShareWithDocument[] }) {
  const { t, i18n } = useTranslation();
  const { showToast } = useToast();
  const revokeMutation = useRevokeDocumentShare(documentId);
  const [confirmingRevokeId, setConfirmingRevokeId] = useState<string | null>(null);

  const handleCopyLink = useCallback(
    (token: string) => {
      const url = `${window.location.origin}/documents/shared/${token}`;
      navigator.clipboard.writeText(url).then(
        () => {
          showToast({
            type: 'success',
            title: t('documents.share.linkCopied'),
            message: url,
            duration: 3000,
          });
        },
        () => {
          showToast({
            type: 'error',
            title: t('documents.share.copyFailed'),
            message: t('documents.share.copyFailedMessage'),
          });
        }
      );
    },
    [showToast, t]
  );

  const handleRevokeRequest = useCallback((shareId: string) => {
    setConfirmingRevokeId(shareId);
  }, []);

  const handleRevokeConfirm = useCallback(async () => {
    const shareId = confirmingRevokeId;
    if (!shareId) return;
    try {
      await revokeMutation.mutateAsync(shareId);
      showToast({ type: 'success', title: t('documents.share.revoked'), duration: 3000 });
    } catch (err) {
      showToast({
        type: 'error',
        title: t('documents.share.revokeFailed'),
        message: err instanceof Error ? err.message : t('documents.share.revokeFailedMessage'),
      });
    } finally {
      setConfirmingRevokeId(null);
    }
  }, [confirmingRevokeId, revokeMutation, showToast, t]);

  const active = shares.filter(isActiveShare);
  if (active.length === 0) return <p className="sp-empty">{t('documents.share.empty')}</p>;

  return (
    <div className="sp-list">
      <h4 className="sp-list-title">{t('documents.share.activeShares')}</h4>
      {active.map((share) => (
        <div key={share.id} className="sp-row">
          <div className="sp-row-info">
            <span className="sp-row-type">{t(shareTypeLabelKey(share.share_type))}</span>
            {share.target_role && (
              <span className="sp-row-meta">
                {t('documents.share.roleLabel', { role: share.target_role })}
              </span>
            )}
            {share.target_id && (
              <span className="sp-row-meta">
                {t('documents.share.userLabel')} <code className="sp-code">{share.target_id}</code>
              </span>
            )}
            {share.expires_at && (
              <span className="sp-row-meta">
                {t('documents.share.expires', {
                  date: fmtExpiry(share.expires_at, i18n.language),
                })}
              </span>
            )}
            {share.share_token && (
              <button
                type="button"
                className="sp-copy-link"
                onClick={() => handleCopyLink(share.share_token as string)}
              >
                {t('documents.share.copyLink')}
              </button>
            )}
          </div>
          <button
            type="button"
            className="sp-revoke-btn"
            onClick={() => handleRevokeRequest(share.id)}
            disabled={revokeMutation.isPending && revokeMutation.variables === share.id}
          >
            {t('documents.share.revoke')}
          </button>
        </div>
      ))}
      <ConfirmationDialog
        isOpen={confirmingRevokeId !== null}
        title={t('documents.share.revokeConfirmTitle')}
        message={t('documents.share.revokeConfirmMessage')}
        confirmLabel={t('documents.share.revoke')}
        cancelLabel={t('common.cancel')}
        variant="danger"
        isLoading={revokeMutation.isPending}
        onConfirm={handleRevokeConfirm}
        onCancel={() => setConfirmingRevokeId(null)}
      />
    </div>
  );
}

export function DocumentSharePanel({
  documentId,
  documentTitle,
}: {
  documentId: string;
  documentTitle: string;
}) {
  const { t } = useTranslation();
  const { showToast } = useToast();
  const { data, isLoading, error, refetch } = useDocumentShares(documentId);

  const handleCreated = useCallback(
    (shareUrl?: string | null) => {
      if (shareUrl) {
        navigator.clipboard.writeText(shareUrl).then(
          () => {
            showToast({
              type: 'success',
              title: t('documents.share.created'),
              message: t('documents.share.publicLinkCopied'),
              duration: 5000,
            });
          },
          () => {
            showToast({
              type: 'success',
              title: t('documents.share.created'),
              message: t('documents.share.linkMessage', { url: shareUrl }),
              duration: 5000,
            });
          }
        );
      } else {
        showToast({ type: 'success', title: t('documents.share.created'), duration: 3000 });
      }
      refetch();
    },
    [showToast, refetch, t]
  );

  return (
    <div className="sp-panel">
      <div className="sp-panel-header">
        <svg
          width="16"
          height="16"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          aria-hidden="true"
        >
          <circle cx="18" cy="5" r="3" />
          <circle cx="6" cy="12" r="3" />
          <circle cx="18" cy="19" r="3" />
          <line x1="8.59" y1="13.51" x2="15.42" y2="17.49" />
          <line x1="15.41" y1="6.51" x2="8.59" y2="10.49" />
        </svg>
        <div className="sp-panel-titles">
          <span className="sp-panel-title">{t('documents.share.title')}</span>
          <span className="sp-panel-subtitle">{documentTitle}</span>
        </div>
      </div>
      <CreateShareForm documentId={documentId} onCreated={handleCreated} />
      <div className="sp-divider" />
      {isLoading && <p className="sp-loading">{t('documents.share.loading')}</p>}
      {error && <p className="sp-fetch-error">{t('documents.share.loadError')}</p>}
      {data && <ShareList documentId={documentId} shares={data.shares} />}
      <style>{panelStyles}</style>
    </div>
  );
}

const panelStyles = `
  .sp-panel{padding:1.25rem;background:var(--ppt-bg-surface);border:1px solid var(--ppt-border-default);border-radius:0.75rem}
  .sp-panel-header{display:flex;align-items:flex-start;gap:0.625rem;margin-bottom:1rem;color:var(--ppt-fg-secondary)}
  .sp-panel-header svg{flex-shrink:0;margin-top:2px}
  .sp-panel-titles{display:flex;flex-direction:column}
  .sp-panel-title{font-size:0.9375rem;font-weight:600;color:var(--ppt-fg-primary);line-height:1.3}
  .sp-panel-subtitle{font-size:0.8125rem;color:var(--ppt-fg-muted);white-space:nowrap;overflow:hidden;text-overflow:ellipsis;max-width:24rem}
  .sp-form{display:flex;flex-direction:column;gap:0.75rem}
  .sp-form-title{margin:0;font-size:0.875rem;font-weight:600;color:var(--ppt-fg-primary)}
  .sp-type-row{display:flex;flex-wrap:wrap;gap:0.5rem}
  .sp-chip{padding:0.25rem 0.875rem;border-radius:9999px;border:1px solid var(--ppt-border-default);background:var(--ppt-bg-app);font-size:0.8125rem;font-weight:500;color:var(--ppt-fg-muted);cursor:pointer;transition:all 0.12s}
  .sp-chip.active{background:var(--ppt-brand-500);border-color:var(--ppt-brand-500);color:var(--ppt-fg-on-accent)}
  .sp-chip:hover:not(.active){border-color:var(--ppt-border-strong);color:var(--ppt-fg-secondary)}
  .sp-field{display:flex;flex-direction:column;gap:0.3125rem}
  .sp-label{font-size:0.8125rem;font-weight:500;color:var(--ppt-fg-secondary)}
  .sp-input{padding:0.5rem 0.75rem;background:var(--ppt-bg-app);border:1px solid var(--ppt-border-default);border-radius:0.375rem;font-size:0.875rem;color:var(--ppt-fg-primary);width:100%;box-sizing:border-box}
  .sp-input:focus{outline:none;border-color:var(--ppt-brand-500)}
  .sp-toggle-row{display:flex;align-items:center;justify-content:space-between;gap:0.75rem}
  .sp-toggle-label{font-size:0.875rem;color:var(--ppt-fg-primary)}
  .sp-switch{position:relative;display:inline-block;width:36px;height:20px;flex-shrink:0;cursor:pointer}
  .sp-switch input{opacity:0;width:0;height:0;position:absolute}
  .sp-switch-track{position:absolute;inset:0;background:var(--ppt-border-strong);border-radius:9999px;transition:background 0.15s}
  .sp-switch-track::before{content:'';position:absolute;left:2px;top:2px;width:16px;height:16px;background:#fff;border-radius:50%;transition:transform 0.15s}
  .sp-switch input:checked+.sp-switch-track{background:var(--ppt-brand-500)}
  .sp-switch input:checked+.sp-switch-track::before{transform:translateX(16px)}
  .sp-expiry-row{display:flex;align-items:center;gap:0.5rem}
  .sp-expiry-input{width:80px;flex-shrink:0}
  .sp-expiry-unit{font-size:0.875rem;color:var(--ppt-fg-muted)}
  .sp-error{margin:0;padding:0.5rem 0.75rem;background:color-mix(in srgb,var(--ppt-color-danger) 10%,transparent);border:1px solid color-mix(in srgb,var(--ppt-color-danger) 30%,transparent);border-radius:0.375rem;font-size:0.8125rem;color:var(--ppt-color-danger)}
  .sp-create-btn{padding:0.5625rem 1rem;background:var(--ppt-brand-500);border:none;border-radius:0.375rem;font-size:0.875rem;font-weight:600;color:var(--ppt-fg-on-accent);cursor:pointer;transition:background 0.12s;align-self:flex-start}
  .sp-create-btn:hover:not(:disabled){background:var(--ppt-color-primary)}
  .sp-create-btn:disabled{opacity:0.6;cursor:not-allowed}
  .sp-divider{height:1px;background:var(--ppt-border-default);margin:1rem 0}
  .sp-list-title{margin:0 0 0.625rem;font-size:0.875rem;font-weight:600;color:var(--ppt-fg-primary)}
  .sp-row{display:flex;align-items:flex-start;justify-content:space-between;gap:0.75rem;padding:0.625rem 0;border-bottom:1px solid var(--ppt-border-default)}
  .sp-row:last-child{border-bottom:none}
  .sp-row-info{display:flex;flex-direction:column;gap:0.1875rem;flex:1;min-width:0}
  .sp-row-type{font-size:0.875rem;font-weight:600;color:var(--ppt-fg-primary)}
  .sp-row-meta{font-size:0.75rem;color:var(--ppt-fg-muted)}
  .sp-code{font-family:monospace;font-size:0.6875rem;background:var(--ppt-bg-app);padding:0.0625rem 0.25rem;border-radius:0.1875rem}
  .sp-copy-link{font-size:0.75rem;font-weight:500;color:var(--ppt-brand-500);background:none;border:none;padding:0;cursor:pointer;text-align:left}
  .sp-copy-link:hover{text-decoration:underline}
  .sp-revoke-btn{padding:0.3125rem 0.75rem;border:1px solid var(--ppt-color-danger);border-radius:0.375rem;background:none;font-size:0.75rem;font-weight:500;color:var(--ppt-color-danger);cursor:pointer;flex-shrink:0;transition:all 0.12s}
  .sp-revoke-btn:hover:not(:disabled){background:color-mix(in srgb,var(--ppt-color-danger) 10%,transparent)}
  .sp-revoke-btn:disabled{opacity:0.5;cursor:not-allowed}
  .sp-empty{margin:0;font-size:0.875rem;color:var(--ppt-fg-muted);text-align:center;padding:1rem 0}
  .sp-loading,.sp-fetch-error{margin:0;font-size:0.875rem;color:var(--ppt-fg-muted)}
  .sp-fetch-error{color:var(--ppt-color-danger)}
`;
