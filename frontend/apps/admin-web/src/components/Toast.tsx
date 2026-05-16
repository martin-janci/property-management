/**
 * Toast Notification Component.
 *
 * Provides an accessible toast notification system with support for:
 * - Multiple toast types (success, error, warning, info)
 * - Title and message fields
 * - Optional action buttons
 * - Configurable duration (0 = persistent)
 * - Copy button for error messages
 * - Maximum 3 visible toasts
 *
 * Uses aria-live for screen reader announcements.
 */

import type React from 'react';
import { createContext, useCallback, useContext, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import './Toast.css';

export type ToastType = 'success' | 'error' | 'info' | 'warning';

export interface ToastAction {
  label: string;
  onClick: () => void;
}

export interface Toast {
  id: string;
  type: ToastType;
  title: string;
  message?: string;
  duration?: number; // 0 = persistent
  action?: ToastAction;
}

interface ToastContextValue {
  showToast: (toast: Omit<Toast, 'id'>) => void;
  removeToast: (id: string) => void;
}

const ToastContext = createContext<ToastContextValue | null>(null);

/**
 * Hook to access toast functionality.
 * Must be used within a ToastProvider.
 */
export function useToast(): ToastContextValue {
  const context = useContext(ToastContext);
  if (!context) {
    throw new Error('useToast must be used within a ToastProvider');
  }
  return context;
}

/**
 * Maximum number of toasts visible at once.
 * When exceeded, only the most recent toasts are shown.
 */
const MAX_VISIBLE_TOASTS = 3;
const DEFAULT_SUCCESS_DURATION = 5000;
const DEFAULT_INFO_DURATION = 5000;
const DEFAULT_WARNING_DURATION = 7000;
/**
 * Error toasts are persistent (duration = 0) by design.
 * This ensures users see and acknowledge errors before dismissing them.
 * Users can still manually dismiss error toasts via the close button.
 */
const ERROR_DURATION = 0;

function getDefaultDuration(type: ToastType): number {
  switch (type) {
    case 'success':
      return DEFAULT_SUCCESS_DURATION;
    case 'info':
      return DEFAULT_INFO_DURATION;
    case 'warning':
      return DEFAULT_WARNING_DURATION;
    case 'error':
      return ERROR_DURATION;
    default:
      return DEFAULT_INFO_DURATION;
  }
}

interface ToastProviderProps {
  children: React.ReactNode;
}

/**
 * Toast provider component.
 * Wrap your app with this to enable toast notifications.
 */
export function ToastProvider({ children }: ToastProviderProps) {
  const { t } = useTranslation();
  const [toasts, setToasts] = useState<Toast[]>([]);
  // Track timeout IDs for cleanup on unmount (using ReturnType for browser compatibility)
  const timeoutRefs = useRef<Map<string, ReturnType<typeof setTimeout>>>(new Map());

  // Clean up all timeouts on unmount
  useEffect(() => {
    return () => {
      for (const timeout of timeoutRefs.current.values()) {
        clearTimeout(timeout);
      }
      timeoutRefs.current.clear();
    };
  }, []);

  const removeToast = useCallback((id: string) => {
    // Clear the timeout when manually dismissed
    const timeoutId = timeoutRefs.current.get(id);
    if (timeoutId) {
      clearTimeout(timeoutId);
      timeoutRefs.current.delete(id);
    }
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  const showToast = useCallback((toastInput: Omit<Toast, 'id'>) => {
    const id = `toast-${Date.now()}-${Math.random().toString(36).slice(2, 11)}`;
    const duration = toastInput.duration ?? getDefaultDuration(toastInput.type);

    const toast: Toast = {
      ...toastInput,
      id,
      duration,
    };

    setToasts((prev) => [...prev, toast]);

    // Only set auto-dismiss if duration > 0
    if (duration > 0) {
      const timeoutId = setTimeout(() => {
        setToasts((prev) => prev.filter((t) => t.id !== id));
        timeoutRefs.current.delete(id);
      }, duration);
      timeoutRefs.current.set(id, timeoutId);
    }
  }, []);

  // Get visible toasts (limited to MAX_VISIBLE_TOASTS)
  // Note: When more than 3 toasts exist, only the 3 most recent are displayed.
  // Older toasts are hidden but still tracked; they will auto-dismiss if they have a duration.
  const visibleToasts = toasts.slice(-MAX_VISIBLE_TOASTS);

  return (
    <ToastContext.Provider value={{ showToast, removeToast }}>
      {children}
      <div className="toast-container" role="region" aria-label={t('common.notifications')}>
        {visibleToasts.map((toast) => (
          <ToastItem key={toast.id} toast={toast} onDismiss={() => removeToast(toast.id)} />
        ))}
      </div>
    </ToastContext.Provider>
  );
}

ToastProvider.displayName = 'ToastProvider';

interface ToastItemProps {
  toast: Toast;
  onDismiss: () => void;
}

/**
 * Individual toast item component.
 */
function ToastItem({ toast, onDismiss }: ToastItemProps) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    const textToCopy = toast.message ? `${toast.title}: ${toast.message}` : toast.title;
    try {
      await navigator.clipboard.writeText(textToCopy);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Fallback for older browsers
      const textArea = document.createElement('textarea');
      textArea.value = textToCopy;
      document.body.appendChild(textArea);
      textArea.select();
      document.execCommand('copy');
      document.body.removeChild(textArea);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  }, [toast.title, toast.message]);

  const ariaLive = toast.type === 'error' ? 'assertive' : 'polite';
  const copiedLabel = t('common.copied');
  const copyLabel = t('common.copyErrorMessage');
  const dismissLabel = t('common.dismissNotification');

  return (
    <output className={`toast toast--${toast.type}`} role="alert" aria-live={ariaLive}>
      <div className="toast-icon">{getIcon(toast.type)}</div>
      <div className="toast-content">
        <div className="toast-title">{toast.title}</div>
        {toast.message && <div className="toast-message">{toast.message}</div>}
      </div>
      <div className="toast-actions">
        {toast.action && (
          <button type="button" className="toast-action-btn" onClick={toast.action.onClick}>
            {toast.action.label}
          </button>
        )}
        {toast.type === 'error' && (
          <button
            type="button"
            className="toast-copy-btn"
            onClick={handleCopy}
            aria-label={copied ? copiedLabel : copyLabel}
            title={copied ? copiedLabel : copyLabel}
          >
            {copied ? (
              <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
              >
                <polyline points="20 6 9 17 4 12" />
              </svg>
            ) : (
              <svg
                width="16"
                height="16"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
              >
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
              </svg>
            )}
          </button>
        )}
        <button type="button" className="toast-close" onClick={onDismiss} aria-label={dismissLabel}>
          <svg
            width="16"
            height="16"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
          >
            <line x1="18" y1="6" x2="6" y2="18" />
            <line x1="6" y1="6" x2="18" y2="18" />
          </svg>
        </button>
      </div>
    </output>
  );
}

function getIcon(type: ToastType): React.ReactNode {
  switch (type) {
    case 'success':
      return (
        <svg
          width="20"
          height="20"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
        >
          <circle cx="12" cy="12" r="10" />
          <polyline points="9 12 12 15 16 10" />
        </svg>
      );
    case 'error':
      return (
        <svg
          width="20"
          height="20"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
        >
          <circle cx="12" cy="12" r="10" />
          <line x1="15" y1="9" x2="9" y2="15" />
          <line x1="9" y1="9" x2="15" y2="15" />
        </svg>
      );
    case 'warning':
      return (
        <svg
          width="20"
          height="20"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
        >
          <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z" />
          <line x1="12" y1="9" x2="12" y2="13" />
          <line x1="12" y1="17" x2="12.01" y2="17" />
        </svg>
      );
    default:
      return (
        <svg
          width="20"
          height="20"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
        >
          <circle cx="12" cy="12" r="10" />
          <line x1="12" y1="16" x2="12" y2="12" />
          <line x1="12" y1="8" x2="12.01" y2="8" />
        </svg>
      );
  }
}
