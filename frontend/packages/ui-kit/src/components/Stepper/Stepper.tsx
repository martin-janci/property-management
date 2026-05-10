/**
 * Stepper — horizontal numbered step progress indicator for wizard flows.
 *
 * Displays step circles connected by lines, with active, done, and upcoming states.
 * Steps before `current` are marked done (green check). The `current` step is
 * highlighted in accent blue. Steps after are neutral.
 *
 * Usage:
 *   <Stepper steps={['Details', 'Photos', 'Review', 'Publish']} current={1} />
 */

import type React from 'react';
import styles from './Stepper.module.css';

export interface StepperProps {
  /** Step labels. Index corresponds to step number. */
  steps: string[];
  /** Zero-based index of the currently active step. */
  current: number;
  className?: string;
}

const CheckIcon = () => (
  <svg
    width="12"
    height="12"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="3"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <polyline points="20 6 9 17 4 12" />
  </svg>
);

/** Stepper: horizontal numbered wizard step list. */
export const Stepper: React.FC<StepperProps> = ({ steps, current, className }) => (
  <nav aria-label="Progress" className={[styles.stepper, className].filter(Boolean).join(' ')}>
    <ol
      className={styles.stepper}
      style={{ listStyle: 'none', padding: 0, margin: 0, display: 'contents' }}
    >
      {steps.map((label, index) => {
        const isActive = index === current;
        const isDone = index < current;

        return (
          <li
            key={index}
            className={[styles.step, isDone ? styles.stepDone : ''].filter(Boolean).join(' ')}
            aria-current={isActive ? 'step' : undefined}
          >
            <div
              className={[
                styles.circle,
                isActive ? styles.circleActive : '',
                isDone ? styles.circleDone : '',
              ]
                .filter(Boolean)
                .join(' ')}
              aria-label={
                isDone
                  ? `Step ${index + 1}: ${label} — completed`
                  : isActive
                    ? `Step ${index + 1}: ${label} — current`
                    : `Step ${index + 1}: ${label}`
              }
            >
              {isDone ? <CheckIcon /> : index + 1}
            </div>
            <span
              className={[
                styles.label,
                isActive ? styles.labelActive : '',
                isDone ? styles.labelDone : '',
              ]
                .filter(Boolean)
                .join(' ')}
            >
              {label}
            </span>
          </li>
        );
      })}
    </ol>
  </nav>
);

export default Stepper;
