/**
 * SegmentedControl — pill-shaped single-select segment control.
 *
 * Renders a row of option buttons in a pill-shaped track.
 * Commonly used for view-mode toggles (e.g. List / Map / Grid).
 *
 * Usage:
 *   <SegmentedControl
 *     options={[{ value: 'list', label: 'List' }, { value: 'map', label: 'Map' }]}
 *     value="list"
 *     onChange={setValue}
 *   />
 */

import React from 'react';
import styles from './SegmentedControl.module.css';

export interface SegmentOption {
  value: string;
  label: string;
  /** Optional icon rendered before the label. */
  icon?: React.ReactNode;
  disabled?: boolean;
}

export interface SegmentedControlProps {
  options: SegmentOption[];
  value: string;
  onChange: (value: string) => void;
  'aria-label'?: string;
  className?: string;
}

/** SegmentedControl: pill-shaped single-select segmented button group. */
export const SegmentedControl: React.FC<SegmentedControlProps> = ({
  options,
  value,
  onChange,
  'aria-label': ariaLabel,
  className,
}) => (
  <div
    className={[styles.control, className].filter(Boolean).join(' ')}
    role="group"
    aria-label={ariaLabel}
  >
    {options.map((opt) => (
      <button
        key={opt.value}
        type="button"
        className={[
          styles.option,
          value === opt.value ? styles.optionActive : '',
        ].filter(Boolean).join(' ')}
        onClick={() => !opt.disabled && onChange(opt.value)}
        aria-pressed={value === opt.value}
        disabled={opt.disabled}
      >
        {opt.icon && <span aria-hidden="true">{opt.icon}</span>}
        {opt.label}
      </button>
    ))}
  </div>
);

export default SegmentedControl;
