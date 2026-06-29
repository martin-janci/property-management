/**
 * Input component for form fields.
 */

import type React from 'react';
import { forwardRef, useId } from 'react';
import './Input.css';

export type InputType =
  | 'text'
  | 'email'
  | 'password'
  | 'number'
  | 'tel'
  | 'url'
  | 'search'
  | 'date';
export type InputSize = 'sm' | 'md' | 'lg';

export interface InputProps extends Omit<React.InputHTMLAttributes<HTMLInputElement>, 'size'> {
  /** Input type */
  type?: InputType;
  /** Label for the input */
  label?: string;
  /** Helper text shown below the input */
  helperText?: string;
  /** Error message (also sets error state) */
  error?: string;
  /** Size of the input */
  size?: InputSize;
  /** Whether the input takes full width */
  fullWidth?: boolean;
  /** Icon to show at the start of the input */
  startIcon?: React.ReactNode;
  /** Icon to show at the end of the input */
  endIcon?: React.ReactNode;
}

/**
 * Input component with label, helper text, and error state.
 *
 * @example
 * ```tsx
 * <Input
 *   label="Email"
 *   type="email"
 *   placeholder="Enter your email"
 *   error={errors.email}
 * />
 * ```
 */
export const Input = forwardRef<HTMLInputElement, InputProps>(
  (
    {
      type = 'text',
      label,
      helperText,
      error,
      size = 'md',
      fullWidth = false,
      startIcon,
      endIcon,
      disabled,
      required,
      id: providedId,
      className = '',
      ...props
    },
    ref
  ) => {
    const generatedId = useId();
    const id = providedId || generatedId;
    const helperId = `${id}-helper`;
    const errorId = `${id}-error`;

    const hasError = Boolean(error);

    const wrapperClasses = [
      'ppt-input-wrapper',
      `ppt-input-wrapper--${size}`,
      fullWidth && 'ppt-input-wrapper--full-width',
      hasError && 'ppt-input-wrapper--error',
      disabled && 'ppt-input-wrapper--disabled',
      className,
    ]
      .filter(Boolean)
      .join(' ');

    const inputClasses = [
      'ppt-input',
      startIcon && 'ppt-input--has-start-icon',
      endIcon && 'ppt-input--has-end-icon',
    ]
      .filter(Boolean)
      .join(' ');

    return (
      <div className={wrapperClasses}>
        {label && (
          <label htmlFor={id} className="ppt-input__label">
            {label}
            {required && (
              <span className="ppt-input__required" aria-hidden="true">
                *
              </span>
            )}
          </label>
        )}
        <div className="ppt-input__container">
          {startIcon && <span className="ppt-input__icon ppt-input__icon--start">{startIcon}</span>}
          <input
            ref={ref}
            id={id}
            type={type}
            className={inputClasses}
            disabled={disabled}
            required={required}
            aria-invalid={hasError}
            aria-describedby={hasError ? errorId : helperText ? helperId : undefined}
            {...props}
          />
          {endIcon && <span className="ppt-input__icon ppt-input__icon--end">{endIcon}</span>}
        </div>
        {error && (
          <p id={errorId} className="ppt-input__error" role="alert">
            {error}
          </p>
        )}
        {helperText && !error && (
          <p id={helperId} className="ppt-input__helper">
            {helperText}
          </p>
        )}
      </div>
    );
  }
);

Input.displayName = 'Input';
