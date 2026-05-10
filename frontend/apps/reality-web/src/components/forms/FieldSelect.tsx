'use client';

/**
 * FieldSelect — custom dropdown with floating label.
 *
 * Native <select> renders OS-styled and can't be themed to match the
 * design's `m-dropdown` look (rounded, big, animated chevron). This is
 * a controlled component with a popover trigger + listbox, keyboard
 * navigable (Enter/Space to open, Esc to close, Up/Down + Enter to
 * select), closes on outside click. Mirrors LanguageSwitcher's
 * popover pattern but adds floating-label affordance + options API.
 */

import type { ReactNode } from 'react';
import { useEffect, useId, useRef, useState } from 'react';
import './forms.css';

export type FieldSelectOption = {
  value: string;
  label: ReactNode;
  /** Optional secondary text (e.g. count or hint). */
  meta?: ReactNode;
  disabled?: boolean;
};

export type FieldSelectProps = {
  label: ReactNode;
  value: string;
  onChange: (value: string) => void;
  options: FieldSelectOption[];
  /** Helper text shown below the field when there's no error. */
  helperText?: ReactNode;
  /** Validation error — replaces helperText, switches border to danger. */
  error?: ReactNode;
  /** Placeholder text shown in the trigger when no value is selected. */
  placeholder?: ReactNode;
  disabled?: boolean;
  required?: boolean;
  className?: string;
  id?: string;
  name?: string;
};

export function FieldSelect({
  label,
  value,
  onChange,
  options,
  helperText,
  error,
  placeholder,
  disabled,
  required,
  className,
  id,
  name,
}: FieldSelectProps) {
  const reactId = useId();
  const fieldId = id || reactId;
  const helperId = `${fieldId}-helper`;
  const listboxId = `${fieldId}-listbox`;

  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState<number>(() =>
    Math.max(
      0,
      options.findIndex((o) => o.value === value)
    )
  );
  const wrapRef = useRef<HTMLDivElement>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);

  // Outside click + Escape — same pattern as LanguageSwitcher.
  useEffect(() => {
    if (!open) return;
    const onClick = (e: MouseEvent) => {
      if (wrapRef.current && !wrapRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        setOpen(false);
        triggerRef.current?.focus();
      }
    };
    document.addEventListener('mousedown', onClick);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onClick);
      document.removeEventListener('keydown', onKey);
    };
  }, [open]);

  const selected = options.find((o) => o.value === value);

  const handleTriggerKey = (e: React.KeyboardEvent<HTMLButtonElement>) => {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      setOpen(true);
    } else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      setOpen((v) => !v);
    }
  };

  const handleListKey = (e: React.KeyboardEvent<HTMLUListElement>) => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setActiveIndex((i) => Math.min(options.length - 1, i + 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setActiveIndex((i) => Math.max(0, i - 1));
    } else if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      const opt = options[activeIndex];
      if (opt && !opt.disabled) {
        onChange(opt.value);
        setOpen(false);
        triggerRef.current?.focus();
      }
    }
  };

  const choose = (opt: FieldSelectOption) => {
    if (opt.disabled) return;
    onChange(opt.value);
    setOpen(false);
    triggerRef.current?.focus();
  };

  return (
    <div
      className={[
        'field',
        'field-select',
        open ? 'field-select-open' : '',
        value ? 'field-has-value' : '',
        error ? 'field-error' : '',
        className,
      ]
        .filter(Boolean)
        .join(' ')}
      ref={wrapRef}
    >
      <button
        type="button"
        ref={triggerRef}
        id={fieldId}
        className="field-select-trigger"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-controls={listboxId}
        aria-invalid={Boolean(error) || undefined}
        aria-describedby={error || helperText ? helperId : undefined}
        aria-required={required || undefined}
        disabled={disabled}
        onClick={() => setOpen((v) => !v)}
        onKeyDown={handleTriggerKey}
      >
        <span className="field-select-value">
          {selected ? (
            selected.label
          ) : (
            <span className="field-select-placeholder">{placeholder}</span>
          )}
        </span>
        <svg
          className="field-select-caret"
          width="14"
          height="14"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2.5"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden="true"
        >
          <polyline points="6 9 12 15 18 9" />
        </svg>
      </button>
      <label htmlFor={fieldId}>{label}</label>
      {/* Hidden native input so the value is included in form submissions
          without extra plumbing in consumers. */}
      <input type="hidden" name={name} value={value} />
      {open && (
        <ul
          id={listboxId}
          className="field-select-menu"
          role="listbox"
          aria-labelledby={fieldId}
          tabIndex={-1}
          // biome-ignore lint/a11y/noNoninteractiveTabindex: listbox is keyboard-navigable
          onKeyDown={handleListKey}
        >
          {options.map((opt, idx) => (
            <li key={opt.value}>
              <button
                type="button"
                role="option"
                aria-selected={opt.value === value}
                disabled={opt.disabled}
                className={[
                  'field-select-option',
                  opt.value === value ? 'active' : '',
                  idx === activeIndex ? 'highlight' : '',
                ]
                  .filter(Boolean)
                  .join(' ')}
                onClick={() => choose(opt)}
                onMouseEnter={() => setActiveIndex(idx)}
              >
                <span className="field-select-option-label">{opt.label}</span>
                {opt.meta && <span className="field-select-option-meta">{opt.meta}</span>}
                {opt.value === value && (
                  <svg
                    width="14"
                    height="14"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2.5"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    aria-hidden="true"
                  >
                    <polyline points="20 6 9 17 4 12" />
                  </svg>
                )}
              </button>
            </li>
          ))}
        </ul>
      )}
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
