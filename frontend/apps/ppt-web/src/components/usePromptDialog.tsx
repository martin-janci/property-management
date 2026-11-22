/**
 * Promise-based replacement for `window.prompt()` backed by the accessible
 * {@link PromptDialog}.
 *
 *   const { prompt, promptDialog } = usePromptDialog();
 *   const reason = await prompt({ title, message });
 *   if (reason === null) return; // cancelled
 *   …
 *   return <>{…}{promptDialog}</>;
 *
 * Unlike the native dialog it is styled, translatable, focus-trapped and does
 * not block the main thread. It resolves to the entered string, or `null` when
 * the user cancels. Only one prompt is pending at a time; a new `prompt()` call
 * resolves any previous one as cancelled (`null`).
 */
import { type ReactNode, useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { PromptDialog } from './PromptDialog';

export interface PromptOptions {
  /** Dialog title (defaults to the shared "Are you sure?" title). */
  title?: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  variant?: 'default' | 'danger';
  placeholder?: string;
  defaultValue?: string;
  multiline?: boolean;
  /** Disable submit while the field is empty/whitespace (default: true). */
  requireValue?: boolean;
  /** Accessible label for the input (defaults to the dialog title). */
  inputLabel?: string;
}

export function usePromptDialog(): {
  prompt: (options: PromptOptions) => Promise<string | null>;
  promptDialog: ReactNode;
} {
  const { t } = useTranslation();
  const [options, setOptions] = useState<PromptOptions | null>(null);
  const resolverRef = useRef<((value: string | null) => void) | null>(null);

  const settle = useCallback((value: string | null) => {
    resolverRef.current?.(value);
    resolverRef.current = null;
    setOptions(null);
  }, []);

  const prompt = useCallback((next: PromptOptions) => {
    resolverRef.current?.(null);
    setOptions(next);
    return new Promise<string | null>((resolve) => {
      resolverRef.current = resolve;
    });
  }, []);

  // Never leave a caller awaiting forever if the component unmounts.
  useEffect(() => () => resolverRef.current?.(null), []);

  const promptDialog = (
    <PromptDialog
      isOpen={options !== null}
      title={options?.title ?? t('common.confirmTitle')}
      message={options?.message ?? ''}
      confirmLabel={options?.confirmLabel ?? t('common.confirm')}
      cancelLabel={options?.cancelLabel ?? t('common.cancel')}
      variant={options?.variant ?? 'default'}
      placeholder={options?.placeholder}
      defaultValue={options?.defaultValue ?? ''}
      multiline={options?.multiline ?? false}
      requireValue={options?.requireValue ?? true}
      inputLabel={options?.inputLabel}
      onConfirm={(value) => settle(value)}
      onCancel={() => settle(null)}
    />
  );

  return { prompt, promptDialog };
}
