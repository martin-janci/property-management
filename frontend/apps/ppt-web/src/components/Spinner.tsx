/**
 * Spinner
 *
 * Shared inline loading spinner used across ppt-web. Replaces the
 * duplicated `animate-spin rounded-full ... border-b-2` markup that was
 * inlined in several places (e.g. the mediation workspace + chat thread).
 *
 * Renders an accessible spinning indicator: a `role="status"` element with
 * an `aria-label` (defaults to the shared `common.loading` translation).
 */

import { useTranslation } from 'react-i18next';

export type SpinnerSize = 'sm' | 'md' | 'lg';

const sizeClasses: Record<SpinnerSize, string> = {
  sm: 'h-5 w-5',
  md: 'h-8 w-8',
  lg: 'h-10 w-10',
};

export interface SpinnerProps {
  /** Visual size. Defaults to `md` (h-8 w-8). */
  size?: SpinnerSize;
  /** Accessible label. Defaults to the `common.loading` translation. */
  label?: string;
  /** Extra classes appended after the base spinner classes. */
  className?: string;
}

export function Spinner({ size = 'md', label, className }: SpinnerProps) {
  const { t } = useTranslation();

  return (
    <div
      role="status"
      aria-label={label ?? t('common.loading')}
      className={`animate-spin rounded-full ${sizeClasses[size]} border-b-2 border-violet-600${
        className ? ` ${className}` : ''
      }`}
    />
  );
}
