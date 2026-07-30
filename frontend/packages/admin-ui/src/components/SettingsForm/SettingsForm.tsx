/**
 * Schema-driven settings form. Capability-gated: when the user lacks the
 * required capability, the form is rendered read-only.
 *
 * Intentionally simple — a real production form would use a schema lib
 * (Zod, react-hook-form). The Phase 5 admin console uses this for tenant
 * settings and platform-wide settings until that wave lands.
 */

import { type FormEvent, type ReactNode, useState } from 'react';
import type { Capability } from '../../capabilities';
import { useCapability } from '../../hooks/useCapability';

export type SettingsField =
  | { kind: 'text'; key: string; label: string; placeholder?: string }
  | { kind: 'number'; key: string; label: string; min?: number; max?: number }
  | { kind: 'boolean'; key: string; label: string }
  | {
      kind: 'select';
      key: string;
      label: string;
      options: ReadonlyArray<{ value: string; label: string }>;
    };

export interface SettingsFormProps<T extends Record<string, unknown>> {
  /** Field definitions, rendered in order. */
  fields: ReadonlyArray<SettingsField>;
  /** Initial values keyed by field key. */
  initialValues: T;
  /** Capability required to submit. UI is read-only without it. */
  capability: Capability;
  /** Submit handler. Called with the new values on form submit. */
  onSubmit: (values: T) => Promise<void> | void;
  /** Optional header. */
  header?: ReactNode;
  /**
   * Force the form into read-only mode regardless of capability. Use when the
   * backing write endpoint does not exist yet, so the form can display values
   * without presenting a Save control that silently does nothing.
   */
  readOnly?: boolean;
  /**
   * Optional persistent banner rendered above the fields — e.g. to explain why
   * the form is read-only.
   */
  notice?: ReactNode;
}

export function SettingsForm<T extends Record<string, unknown>>({
  fields,
  initialValues,
  capability,
  onSubmit,
  header,
  readOnly = false,
  notice,
}: SettingsFormProps<T>) {
  const hasCapability = useCapability(capability);
  const canWrite = hasCapability && !readOnly;
  const [values, setValues] = useState<T>(initialValues);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    if (!canWrite) return;
    setSubmitting(true);
    setError(null);
    try {
      await onSubmit(values);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Save failed.');
    } finally {
      setSubmitting(false);
    }
  }

  function update<K extends keyof T>(key: K, value: T[K]) {
    setValues((v) => ({ ...v, [key]: value }));
  }

  return (
    <form className="ppt-admin-settings-form" onSubmit={handleSubmit}>
      {header}
      {notice ? (
        <div role="status" className="ppt-admin-form-notice">
          {notice}
        </div>
      ) : null}
      {fields.map((field) => (
        <FieldRow
          key={field.key}
          field={field}
          value={values[field.key as keyof T]}
          onChange={(v) => update(field.key as keyof T, v as T[keyof T])}
          disabled={!canWrite || submitting}
        />
      ))}
      {error ? (
        <div role="alert" className="ppt-admin-form-error">
          {error}
        </div>
      ) : null}
      <div className="ppt-admin-form-actions">
        <button type="submit" disabled={!canWrite || submitting}>
          {canWrite ? (submitting ? 'Saving…' : 'Save') : 'Read-only'}
        </button>
      </div>
    </form>
  );
}

function FieldRow({
  field,
  value,
  onChange,
  disabled,
}: {
  field: SettingsField;
  value: unknown;
  onChange: (v: unknown) => void;
  disabled: boolean;
}) {
  const id = `ppt-admin-${field.key}`;
  return (
    <label htmlFor={id} className="ppt-admin-field">
      <span className="ppt-admin-field-label">{field.label}</span>
      {renderInput(field, id, value, onChange, disabled)}
    </label>
  );
}

function renderInput(
  field: SettingsField,
  id: string,
  value: unknown,
  onChange: (v: unknown) => void,
  disabled: boolean
): ReactNode {
  switch (field.kind) {
    case 'text':
      return (
        <input
          id={id}
          type="text"
          placeholder={field.placeholder}
          value={(value as string) ?? ''}
          onChange={(e) => onChange(e.target.value)}
          disabled={disabled}
        />
      );
    case 'number':
      return (
        <input
          id={id}
          type="number"
          min={field.min}
          max={field.max}
          value={Number.isFinite(value as number) ? (value as number) : 0}
          onChange={(e) => onChange(Number(e.target.value))}
          disabled={disabled}
        />
      );
    case 'boolean':
      return (
        <input
          id={id}
          type="checkbox"
          checked={Boolean(value)}
          onChange={(e) => onChange(e.target.checked)}
          disabled={disabled}
        />
      );
    case 'select':
      return (
        <select
          id={id}
          value={(value as string) ?? ''}
          onChange={(e) => onChange(e.target.value)}
          disabled={disabled}
        >
          {field.options.map((o) => (
            <option key={o.value} value={o.value}>
              {o.label}
            </option>
          ))}
        </select>
      );
  }
}
