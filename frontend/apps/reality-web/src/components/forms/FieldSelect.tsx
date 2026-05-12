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

  // Keep activeIndex pointed at the currently-selected option whenever
  // `value` or `options` change from the parent — otherwise the highlight
  // can drift after a controlled-value swap (reopening the menu).
  useEffect(() => {
    const i = options.findIndex((o) => o.value === value);
    if (i >= 0) setActiveIndex(i);
  }, [value, options]);

  // Listbox uses a document-level keydown listener instead of an onKeyDown
  // attached to the menu element, because the listbox never receives focus
  // (the trigger button keeps focus while the menu is open, per APG
  // listbox/combobox pattern when no edit field is used). Arrow keys move
  // the highlight, Enter/Space commits, Escape closes.
  useEffect(() => {
    if (!open) return;
    const onClick = (e: MouseEvent) => {
      if (wrapRef.current && !wrapRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        setOpen(false);
        triggerRef.current?.focus();
        return;
      }
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setActiveIndex((i) => Math.min(options.length - 1, i + 1));
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setActiveIndex((i) => Math.max(0, i - 1));
      } else if (e.key === 'Home') {
        e.preventDefault();
        setActiveIndex(0);
      } else if (e.key === 'End') {
        e.preventDefault();
        setActiveIndex(options.length - 1);
      } else if (e.key === 'Enter' || e.key === ' ') {
        // Use the latest activeIndex from state at fire time
        e.preventDefault();
        setActiveIndex((i) => {
          const opt = options[i];
          if (opt && !opt.disabled) {
            onChange(opt.value);
            setOpen(false);
            triggerRef.current?.focus();
          }
          return i;
        });
      }
    };
    document.addEventListener('mousedown', onClick);
    document.addEventListener('keydown', onKey);
    return () => {
      document.removeEventListener('mousedown', onClick);
      document.removeEventListener('keydown', onKey);
    };
  }, [open, options, onChange]);

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
        // aria-activedescendant lives on the focusable trigger (not the
        // listbox) because focus stays here when the menu is open — that's
        // the APG combobox-with-aria-activedescendant variant.
        aria-activedescendant={
          open && options[activeIndex] ? `${fieldId}-opt-${options[activeIndex].value}` : undefined
        }
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
      {/* Disabled fields shouldn't participate in form submission — pass
          through the disabled prop so the value is excluded when the
          form is submitted. */}
      <input type="hidden" name={name} value={value} disabled={disabled} />
      {open && (
        // <div role="listbox"> rather than <ul role="listbox"> per ARIA
        // Combobox pattern — biome rejects the listbox role on semantic
        // list elements (noNoninteractiveElementToInteractiveRole).
        <div id={listboxId} className="field-select-menu" role="listbox" aria-labelledby={fieldId}>
          {options.map((opt, idx) => (
            <button
              key={opt.value}
              id={`${fieldId}-opt-${opt.value}`}
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
          ))}
        </div>
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
