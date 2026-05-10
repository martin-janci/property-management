/**
 * ChipGroup — horizontal row of pill chips for filtering or category selection.
 *
 * Supports single-select or multi-select via the `mode` prop.
 * Each chip can optionally show a count badge.
 *
 * Usage (single):
 *   <ChipGroup mode="single" chips={[...]} value="cat" onChange={setValue} />
 *
 * Usage (multi):
 *   <ChipGroup mode="multi" chips={[...]} value={['cat', 'dog']} onChange={setValues} />
 */

import type React from 'react';
import styles from './ChipGroup.module.css';

export interface Chip {
  key: string;
  label: string;
  count?: number;
  /** Disabled state for this individual chip. */
  disabled?: boolean;
}

export type ChipGroupMode = 'single' | 'multi';

interface ChipGroupSingleProps {
  mode: 'single';
  chips: Chip[];
  value: string | null;
  onChange: (value: string) => void;
  className?: string;
  'aria-label'?: string;
}

interface ChipGroupMultiProps {
  mode: 'multi';
  chips: Chip[];
  value: string[];
  onChange: (value: string[]) => void;
  className?: string;
  'aria-label'?: string;
}

export type ChipGroupProps = ChipGroupSingleProps | ChipGroupMultiProps;

/** ChipGroup: single or multi-select pill chips for category filtering. */
export const ChipGroup: React.FC<ChipGroupProps> = (props) => {
  const { chips, className } = props;

  const isActive = (key: string): boolean => {
    if (props.mode === 'single') return props.value === key;
    return props.value.includes(key);
  };

  const handleClick = (key: string) => {
    if (props.mode === 'single') {
      props.onChange(key);
    } else {
      const current = props.value;
      if (current.includes(key)) {
        props.onChange(current.filter((k) => k !== key));
      } else {
        props.onChange([...current, key]);
      }
    }
  };

  return (
    <div
      className={[styles.group, className].filter(Boolean).join(' ')}
      role="group"
      aria-label={props['aria-label']}
    >
      {chips.map((chip) => {
        const active = isActive(chip.key);
        return (
          <button
            key={chip.key}
            type="button"
            className={[styles.chip, active ? styles.chipActive : ''].filter(Boolean).join(' ')}
            onClick={() => !chip.disabled && handleClick(chip.key)}
            aria-pressed={active}
            disabled={chip.disabled}
          >
            {chip.label}
            {chip.count != null && (
              <span className={styles.count} aria-label={`${chip.count} items`}>
                {chip.count}
              </span>
            )}
          </button>
        );
      })}
    </div>
  );
};

export default ChipGroup;
