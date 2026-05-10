/**
 * RadioCards — grid of radio-button-styled cards with icon + label + description.
 *
 * Each card acts as a radio input. Supports configurable column count.
 * WCAG: uses real `<input type="radio">` under the hood for keyboard nav.
 *
 * Usage:
 *   <RadioCards
 *     options={[
 *       { value: 'owner', label: 'Owner', description: 'I own the property', icon: <HomeIcon /> },
 *     ]}
 *     value="owner"
 *     onChange={setValue}
 *     columns={2}
 *   />
 */

import type React from 'react';
import { useId } from 'react';
import styles from './RadioCards.module.css';

export interface RadioCardOption {
  value: string;
  label: string;
  description?: string;
  icon?: React.ReactNode;
  disabled?: boolean;
}

export interface RadioCardsProps {
  options: RadioCardOption[];
  value: string;
  onChange: (value: string) => void;
  /** Number of grid columns. Default: auto based on option count. */
  columns?: 1 | 2 | 3 | 4;
  name?: string;
  'aria-label'?: string;
  className?: string;
}

/** RadioCards: accessible radio input rendered as visual cards with icon. */
export const RadioCards: React.FC<RadioCardsProps> = ({
  options,
  value,
  onChange,
  columns,
  name: nameProp,
  'aria-label': ariaLabel,
  className,
}) => {
  const uid = useId();
  const groupName = nameProp ?? `radio-cards-${uid}`;
  // Clamp to ≥1 — Math.min(0, 3) === 0 produces invalid `repeat(0, 1fr)` CSS
  // when options is empty.
  const cols = columns ?? Math.max(1, Math.min(options.length, 3));

  return (
    <div
      role="radiogroup"
      aria-label={ariaLabel}
      className={[styles.grid, className].filter(Boolean).join(' ')}
      style={{ gridTemplateColumns: `repeat(${cols}, 1fr)` }}
    >
      {options.map((opt, idx) => {
        const isActive = value === opt.value;
        // Index-based ID — opt.value may contain spaces / special chars unsafe
        // for HTML `id` attribute and CSS selectors. The radio's `value` prop
        // (below) keeps form semantics intact.
        const inputId = `${groupName}-${idx}`;

        return (
          <label key={opt.value} htmlFor={inputId} className={styles.label}>
            <input
              id={inputId}
              type="radio"
              name={groupName}
              value={opt.value}
              checked={isActive}
              onChange={() => onChange(opt.value)}
              disabled={opt.disabled}
              className={styles.radio}
            />
            <div
              className={[styles.card, isActive ? styles.cardActive : ''].filter(Boolean).join(' ')}
            >
              {opt.icon && (
                <div
                  className={[styles.icon, isActive ? styles.iconActive : '']
                    .filter(Boolean)
                    .join(' ')}
                  aria-hidden="true"
                >
                  {opt.icon}
                </div>
              )}
              <div className={styles.body}>
                <p className={styles.cardLabel}>{opt.label}</p>
                {opt.description && <p className={styles.description}>{opt.description}</p>}
              </div>
              <div
                className={[styles.indicator, isActive ? styles.indicatorActive : '']
                  .filter(Boolean)
                  .join(' ')}
                aria-hidden="true"
              />
            </div>
          </label>
        );
      })}
    </div>
  );
};

export default RadioCards;
