/**
 * DestructiveConfirmDialog — portal modal for destructive operations.
 *
 * Behavior:
 *   - Renders via React portal at document.body
 *   - User must type confirmText exactly (case-sensitive) to enable confirm
 *   - Paste is blocked on the confirm input
 *   - Optional delayMs: confirm button disabled for N ms after typing matches
 *   - Optional reasonAction: embeds <AuditReasonPrompt> before the confirm input
 *   - ESC key cancels; focus trap inside modal; backdrop click cancels
 *   - During onConfirm(): spinner + disabled state
 */

import {
  type KeyboardEvent,
  type ReactNode,
  useCallback,
  useEffect,
  useId,
  useRef,
  useState,
} from 'react';
import { createPortal } from 'react-dom';

import {
  type AuditReasonAction,
  AuditReasonPrompt,
  auditReasonMinLength,
  useAuditReasonValid,
} from './AuditReasonPrompt';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface DestructiveConfirmDialogProps {
  open: boolean;
  title: string;
  body: ReactNode;
  confirmText: string;
  confirmLabel?: string;
  delayMs?: number;
  reasonAction?: AuditReasonAction;
  /**
   * When false (and `reasonAction` is set), an empty reason is accepted —
   * but a non-empty reason must still pass audit-reason validation.
   * Defaults to true (reason required).
   */
  reasonRequired?: boolean;
  /**
   * Called with the audit reason (trimmed; empty string when no
   * `reasonAction` is configured or an optional reason was left blank).
   */
  onConfirm: (reason: string) => Promise<void>;
  onCancel: () => void;
}

// ---------------------------------------------------------------------------
// Styles
// ---------------------------------------------------------------------------

const STYLE_ID = 'ppt-destructive-dialog-styles';

function ensureStyles() {
  if (typeof document === 'undefined') return;
  if (document.getElementById(STYLE_ID)) return;
  const el = document.createElement('style');
  el.id = STYLE_ID;
  el.textContent = `
    .ppt-dcd-backdrop {
      position: fixed;
      inset: 0;
      z-index: 9999;
      background: rgba(0, 0, 0, 0.5);
      display: flex;
      align-items: center;
      justify-content: center;
      padding: 16px;
    }
    .ppt-dcd-panel {
      max-width: 480px;
      width: 100%;
      background: var(--ppt-bg-surface, #ffffff);
      border-radius: var(--ppt-radius-lg, 12px);
      padding: 24px;
      box-shadow: var(--ppt-shadow-modal, 0 10px 40px rgba(0,0,0,0.15));
      display: flex;
      flex-direction: column;
      gap: 16px;
      position: relative;
    }
    .ppt-dcd-title {
      font-size: 16px;
      font-weight: 600;
      color: var(--ppt-danger-700, #b91c1c);
      margin: 0;
    }
    .ppt-dcd-body {
      font-size: 14px;
      color: var(--ppt-fg-secondary, #374151);
      line-height: 1.55;
    }
    .ppt-dcd-confirm-text {
      font-family: var(--ppt-font-mono, 'JetBrains Mono', 'SF Mono', 'Menlo', 'Consolas', monospace);
      font-size: 13px;
      background: var(--ppt-bg-subtle, #f3f4f6);
      padding: 3px 8px;
      border-radius: 4px;
      color: var(--ppt-fg-primary, #111827);
      word-break: break-all;
    }
    .ppt-dcd-input-group {
      display: flex;
      flex-direction: column;
      gap: 6px;
    }
    .ppt-dcd-input-label {
      font-size: 13px;
      font-weight: 500;
      color: var(--ppt-fg-secondary, #374151);
    }
    .ppt-dcd-input {
      width: 100%;
      box-sizing: border-box;
      font-family: var(--ppt-font-mono, 'JetBrains Mono', 'SF Mono', 'Menlo', 'Consolas', monospace);
      font-size: 13px;
      padding: 8px 10px;
      border: 1px solid var(--ppt-border-default, #e5e7eb);
      border-radius: var(--ppt-radius-md, 8px);
      background: var(--ppt-bg-input, #ffffff);
      color: var(--ppt-fg-primary, #111827);
      outline: none;
      transition: border-color 120ms ease, box-shadow 120ms ease;
    }
    .ppt-dcd-input:focus {
      border-color: var(--ppt-brand-500, #3b82f6);
      box-shadow: 0 0 0 2px var(--ppt-brand-500, #3b82f6);
    }
    .ppt-dcd-actions {
      display: flex;
      justify-content: flex-end;
      gap: 10px;
      margin-top: 4px;
    }
    .ppt-dcd-btn {
      display: inline-flex;
      align-items: center;
      gap: 8px;
      padding: 8px 16px;
      border-radius: var(--ppt-radius-md, 8px);
      font-size: 14px;
      font-weight: 500;
      cursor: pointer;
      border: none;
      transition: background 120ms ease, opacity 120ms ease;
      outline: none;
    }
    .ppt-dcd-btn:focus-visible {
      box-shadow: 0 0 0 3px rgba(37, 99, 235, 0.35);
    }
    .ppt-dcd-btn--cancel {
      background: transparent;
      color: var(--ppt-fg-secondary, #374151);
      border: 1px solid var(--ppt-border-default, #e5e7eb);
    }
    .ppt-dcd-btn--cancel:hover {
      background: var(--ppt-bg-hover, #f3f4f6);
    }
    .ppt-dcd-btn--confirm {
      background: var(--ppt-danger-600, #dc2626);
      color: #ffffff;
    }
    .ppt-dcd-btn--confirm:hover:not(:disabled) {
      background: var(--ppt-danger-700, #b91c1c);
    }
    .ppt-dcd-btn--confirm:disabled {
      opacity: 0.45;
      cursor: not-allowed;
    }
    .ppt-dcd-spinner {
      width: 14px;
      height: 14px;
      border: 2px solid rgba(255,255,255,0.4);
      border-top-color: #ffffff;
      border-radius: 50%;
      animation: ppt-dcd-spin 0.7s linear infinite;
      flex-shrink: 0;
    }
    @keyframes ppt-dcd-spin {
      to { transform: rotate(360deg); }
    }
  `;
  document.head.appendChild(el);
}

// ---------------------------------------------------------------------------
// Focus trap helper
// ---------------------------------------------------------------------------

const FOCUSABLE_SELECTOR =
  'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';

function trapFocus(container: HTMLElement, e: KeyboardEvent<HTMLDivElement>) {
  const focusable = Array.from(container.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR));
  if (focusable.length === 0) return;
  const first = focusable[0];
  const last = focusable[focusable.length - 1];
  if (e.key === 'Tab') {
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
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function DestructiveConfirmDialog({
  open,
  title,
  body,
  confirmText,
  confirmLabel = 'Type the name to confirm',
  delayMs,
  reasonAction,
  reasonRequired = true,
  onConfirm,
  onCancel,
}: DestructiveConfirmDialogProps) {
  ensureStyles();

  const [typed, setTyped] = useState('');
  const [reason, setReason] = useState('');
  const [delayRemainingMs, setDelayRemainingMs] = useState(0);
  const [isPending, setIsPending] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);
  const firstFocusableRef = useRef<HTMLElement | null>(null);
  const titleId = useId();
  const delayTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const reasonMinLength = reasonAction ? auditReasonMinLength(reasonAction) : 20;
  const reasonIsValid = useAuditReasonValid(reason, reasonMinLength);
  const typedMatches = typed === confirmText;
  const reasonSatisfied = reasonAction
    ? reasonIsValid || (!reasonRequired && reason.trim() === '')
    : true;

  // Start delay countdown when typed matches (and delay configured)
  useEffect(() => {
    if (!typedMatches || !delayMs || !reasonSatisfied) {
      setDelayRemainingMs(0);
      if (delayTimerRef.current) {
        clearInterval(delayTimerRef.current);
        delayTimerRef.current = null;
      }
      return;
    }
    // Reset and start countdown
    setDelayRemainingMs(delayMs);
    const interval = 100;
    delayTimerRef.current = setInterval(() => {
      setDelayRemainingMs((prev) => {
        const next = prev - interval;
        if (next <= 0) {
          if (delayTimerRef.current) {
            clearInterval(delayTimerRef.current);
            delayTimerRef.current = null;
          }
          return 0;
        }
        return next;
      });
    }, interval);
    return () => {
      if (delayTimerRef.current) {
        clearInterval(delayTimerRef.current);
        delayTimerRef.current = null;
      }
    };
  }, [typedMatches, reasonSatisfied, delayMs]);

  // Reset state when dialog opens/closes
  useEffect(() => {
    if (!open) {
      setTyped('');
      setReason('');
      setDelayRemainingMs(0);
      setIsPending(false);
      if (delayTimerRef.current) {
        clearInterval(delayTimerRef.current);
        delayTimerRef.current = null;
      }
    }
  }, [open]);

  // Focus first focusable element on open
  useEffect(() => {
    if (!open || !panelRef.current) return;
    const focusable = panelRef.current.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR);
    if (focusable.length > 0) {
      firstFocusableRef.current = focusable[0];
      setTimeout(() => focusable[0].focus(), 0);
    }
  }, [open]);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent<HTMLDivElement>) => {
      if (e.key === 'Escape') {
        onCancel();
        return;
      }
      if (panelRef.current) trapFocus(panelRef.current, e);
    },
    [onCancel]
  );

  const handleConfirm = useCallback(async () => {
    if (isPending) return;
    setIsPending(true);
    try {
      await onConfirm(reason.trim());
    } finally {
      setIsPending(false);
    }
  }, [isPending, onConfirm, reason]);

  const delayInProgress = delayMs !== undefined && delayMs > 0 && delayRemainingMs > 0;
  const confirmDisabled = !typedMatches || !reasonSatisfied || delayInProgress || isPending;

  const confirmButtonLabel = isPending
    ? 'Confirming…'
    : delayInProgress
      ? `Wait ${Math.ceil(delayRemainingMs / 1000)}s`
      : 'Confirm';

  if (!open) return null;

  return createPortal(
    <div
      className="ppt-dcd-backdrop"
      role="dialog"
      aria-modal="true"
      aria-labelledby={titleId}
      onKeyDown={handleKeyDown}
      onClick={(e) => {
        // Cancel on backdrop click (not panel click)
        if (e.target === e.currentTarget) onCancel();
      }}
    >
      <div ref={panelRef} className="ppt-dcd-panel" onClick={(e) => e.stopPropagation()}>
        <h2 id={titleId} className="ppt-dcd-title">
          {title}
        </h2>

        <div className="ppt-dcd-body">{body}</div>

        {reasonAction && (
          <AuditReasonPrompt
            action={reasonAction}
            value={reason}
            onChange={setReason}
            required={reasonRequired}
          />
        )}

        <div className="ppt-dcd-input-group">
          <label className="ppt-dcd-input-label" htmlFor="ppt-dcd-confirm-input">
            {confirmLabel}{' '}
            <span className="ppt-dcd-confirm-text" aria-label={`Type: ${confirmText}`}>
              {confirmText}
            </span>
          </label>
          <input
            id="ppt-dcd-confirm-input"
            className="ppt-dcd-input"
            type="text"
            value={typed}
            onChange={(e) => setTyped(e.target.value)}
            onPaste={(e) => e.preventDefault()}
            autoComplete="off"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck={false}
            aria-label={`Type "${confirmText}" to confirm`}
          />
        </div>

        <div className="ppt-dcd-actions">
          <button
            type="button"
            className="ppt-dcd-btn ppt-dcd-btn--cancel"
            onClick={onCancel}
            disabled={isPending}
          >
            Cancel
          </button>
          <button
            type="button"
            className="ppt-dcd-btn ppt-dcd-btn--confirm"
            onClick={handleConfirm}
            disabled={confirmDisabled}
            aria-disabled={confirmDisabled}
          >
            {isPending && <span className="ppt-dcd-spinner" aria-hidden="true" />}
            {confirmButtonLabel}
          </button>
        </div>
      </div>
    </div>,
    document.body
  );
}

DestructiveConfirmDialog.displayName = 'DestructiveConfirmDialog';
