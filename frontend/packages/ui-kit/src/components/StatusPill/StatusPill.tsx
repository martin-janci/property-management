/**
 * StatusPill — generic status badge pill with an optional leading dot.
 *
 * Covers all 7 semantic variants used across the 8 state-machine workflows
 * (fault, vote, announcement, reservation, invoice, listing, delegation, job).
 *
 * Usage:
 *   <StatusPill variant="success" label="Resolved" dot />
 *   <StatusPill variant="danger" label="Overdue" />
 */

import type React from 'react';
import styles from './StatusPill.module.css';

export type StatusPillVariant =
  | 'success'
  | 'warning'
  | 'danger'
  | 'info'
  | 'event'
  | 'neutral'
  | 'brand';

export interface StatusPillProps {
  variant: StatusPillVariant;
  label: string;
  /** Show a leading filled dot (uses current text color). Default: false. */
  dot?: boolean;
  className?: string;
}

const variantClassMap: Record<StatusPillVariant, string> = {
  success: styles.success ?? '',
  warning: styles.warning ?? '',
  danger: styles.danger ?? '',
  info: styles.info ?? '',
  event: styles.event ?? '',
  neutral: styles.neutral ?? '',
  brand: styles.brand ?? '',
};

/** StatusPill: compact pill badge for state-machine status labels. */
export const StatusPill: React.FC<StatusPillProps> = ({
  variant,
  label,
  dot = false,
  className,
}) => (
  <span
    className={[styles.pill, variantClassMap[variant], className].filter(Boolean).join(' ')}
    aria-label={label}
  >
    {dot && <span className={styles.dot} aria-hidden="true" />}
    {label}
  </span>
);

export default StatusPill;
