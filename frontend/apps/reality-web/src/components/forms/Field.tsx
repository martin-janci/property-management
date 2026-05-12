'use client';

/**
 * Modern form-field primitives — floating-label inputs and textarea.
 *
 * Mirrors the `m-field / m-input / m-textarea` rules from the design
 * bundle (pages/sell.html, profile.html, contact.html). The label sits
 * on top of the input like a placeholder, then animates up + shrinks
 * when the field has a value or is focused.
 *
 * Implementation note: the "has value" state uses the CSS
 * `:not(:placeholder-shown)` trick (the input is rendered with
 * `placeholder=" "` so it always has a placeholder for the CSS selector
 * to flip). This avoids React state for value tracking — works for both
 * controlled and uncontrolled inputs, and stays accessible to
 * `react-hook-form` `register()` without extra plumbing.
 *
 * The custom dropdown lives in ./FieldSelect because it needs a popover
 * with keyboard nav and search — outside the scope of CSS-only.
 */

import {
  forwardRef,
  type InputHTMLAttributes,
  type ReactNode,
  type TextareaHTMLAttributes,
  useId,
} from 'react';
import './forms.css';

type CommonProps = {
  /** Field label rendered as floating placeholder. */
  label: ReactNode;
  /** Helper text shown below the input when there's no error. */
  helperText?: ReactNode;
  /** Validation error — replaces helperText, switches border to danger. */
  error?: ReactNode;
  /** Optional prefix (e.g. currency symbol) shown inside the field. */
  prefix?: ReactNode;
  /** Optional suffix (e.g. unit) shown inside the field. */
  suffix?: ReactNode;
};

export type FieldInputProps = CommonProps &
  Omit<InputHTMLAttributes<HTMLInputElement>, 'placeholder'>;

export const FieldInput = forwardRef<HTMLInputElement, FieldInputProps>(
  ({ label, helperText, error, prefix, suffix, className, id, ...rest }, ref) => {
    const reactId = useId();
    const inputId = id || reactId;
    const helperId = `${inputId}-helper`;
    return (
      <div
        className={[
          'field',
          error ? 'field-error' : '',
          prefix ? 'field-has-prefix' : '',
          suffix ? 'field-has-suffix' : '',
          className,
        ]
          .filter(Boolean)
          .join(' ')}
      >
        {prefix && <span className="field-prefix">{prefix}</span>}
        <input
          ref={ref}
          id={inputId}
          /* placeholder=" " is required for the :not(:placeholder-shown)
             trick that drives the floating-label animation. Don't set a
             real placeholder — the label IS the placeholder. */
          placeholder=" "
          /* Chrome 120+ injects caret-color into the inline style of any
             input it suspects is being autofilled, which races with the
             SSR-rendered style and triggers a hydration warning. Suppress
             it so the console stays clean — the DOM rebuilds correctly. */
          suppressHydrationWarning
          aria-invalid={Boolean(error) || undefined}
          aria-describedby={error || helperText ? helperId : undefined}
          {...rest}
        />
        <label htmlFor={inputId}>{label}</label>
        {suffix && <span className="field-suffix">{suffix}</span>}
        {(error || helperText) && (
          <p
            id={helperId}
            className={error ? 'field-msg field-msg-error' : 'field-msg'}
            role={error ? 'alert' : undefined}
          >
            {error || helperText}
          </p>
        )}
      </div>
    );
  }
);
FieldInput.displayName = 'FieldInput';

export type FieldTextareaProps = CommonProps &
  Omit<TextareaHTMLAttributes<HTMLTextAreaElement>, 'placeholder'>;

export const FieldTextarea = forwardRef<HTMLTextAreaElement, FieldTextareaProps>(
  ({ label, helperText, error, className, id, rows = 4, ...rest }, ref) => {
    const reactId = useId();
    const inputId = id || reactId;
    const helperId = `${inputId}-helper`;
    return (
      <div
        className={['field', 'field-textarea', error ? 'field-error' : '', className]
          .filter(Boolean)
          .join(' ')}
      >
        <textarea
          ref={ref}
          id={inputId}
          rows={rows}
          placeholder=" "
          aria-invalid={Boolean(error) || undefined}
          aria-describedby={error || helperText ? helperId : undefined}
          {...rest}
        />
        <label htmlFor={inputId}>{label}</label>
        {(error || helperText) && (
          <p
            id={helperId}
            className={error ? 'field-msg field-msg-error' : 'field-msg'}
            role={error ? 'alert' : undefined}
          >
            {error || helperText}
          </p>
        )}
      </div>
    );
  }
);
FieldTextarea.displayName = 'FieldTextarea';
