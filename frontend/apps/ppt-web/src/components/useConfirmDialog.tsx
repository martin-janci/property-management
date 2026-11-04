/**
 * Promise-based replacement for `window.confirm()` backed by the accessible
 * {@link ConfirmationDialog}.
 *
 *   const { confirm, confirmDialog } = useConfirmDialog();
 *   if (!(await confirm({ title, message, variant: 'danger' }))) return;
 *   …
 *   return <>{…}{confirmDialog}</>;
 *
 * Unlike the native dialog it is styled, translatable, focus-trapped and does
 * not block the main thread. Only one confirmation is pending at a time; a
 * new `confirm()` call resolves any previous one as cancelled.
 */
import { type ReactNode, useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ConfirmationDialog } from './ConfirmationDialog';

export interface ConfirmOptions {
  /** Dialog title (defaults to the shared "Are you sure?" title). */
  title?: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  variant?: 'default' | 'danger';
}

export function useConfirmDialog(): {
  confirm: (options: ConfirmOptions) => Promise<boolean>;
  confirmDialog: ReactNode;
} {
  const { t } = useTranslation();
  const [options, setOptions] = useState<ConfirmOptions | null>(null);
  const resolverRef = useRef<((value: boolean) => void) | null>(null);

  const settle = useCallback((value: boolean) => {
    resolverRef.current?.(value);
    resolverRef.current = null;
    setOptions(null);
  }, []);

  const confirm = useCallback((next: ConfirmOptions) => {
    resolverRef.current?.(false);
    setOptions(next);
    return new Promise<boolean>((resolve) => {
      resolverRef.current = resolve;
    });
  }, []);

  // Never leave a caller awaiting forever if the component unmounts.
  useEffect(() => () => resolverRef.current?.(false), []);

  const confirmDialog = (
    <ConfirmationDialog
      isOpen={options !== null}
      title={options?.title ?? t('common.confirmTitle')}
      message={options?.message ?? ''}
      confirmLabel={options?.confirmLabel ?? t('common.confirm')}
      cancelLabel={options?.cancelLabel ?? t('common.cancel')}
      variant={options?.variant ?? 'default'}
      onConfirm={() => settle(true)}
      onCancel={() => settle(false)}
    />
  );

  return { confirm, confirmDialog };
}
