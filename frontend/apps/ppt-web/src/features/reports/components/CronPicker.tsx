/**
 * CronPicker — Visual builder for 5-field UNIX cron expressions.
 *
 * Story 81.1 - Report Schedule Editor
 *
 * Provides two modes:
 *   - "simple": preset helpers (daily, weekly, monthly, …) + time/day selectors
 *   - "custom": free-text input with live validation feedback
 *
 * The component always emits a valid 5-field UNIX cron string or null when
 * the current input fails validation.
 */

import { useCallback, useEffect, useState } from 'react';

// ─────────────────────────────────────────────────────────────
// Types & constants
// ─────────────────────────────────────────────────────────────

export interface CronPickerProps {
  /** Current cron expression, e.g. "0 8 * * 1". Empty string = unset. */
  value: string;
  /** Called with the new cron string whenever a valid expression is built. */
  onChange: (cron: string) => void;
  /** Shown below the picker when the expression is invalid. */
  error?: string;
  disabled?: boolean;
}

type SimplePreset = 'daily' | 'weekly' | 'monthly' | 'custom';

interface PresetDefinition {
  label: string;
  description: string;
}

const PRESETS: Record<SimplePreset, PresetDefinition> = {
  daily: { label: 'Daily', description: 'Every day at the chosen time' },
  weekly: { label: 'Weekly', description: 'Once a week on the chosen day' },
  monthly: { label: 'Monthly', description: 'Once a month on the chosen day' },
  custom: { label: 'Custom', description: 'Enter a cron expression manually' },
};

const DAYS_OF_WEEK = [
  { value: '0', label: 'Sunday' },
  { value: '1', label: 'Monday' },
  { value: '2', label: 'Tuesday' },
  { value: '3', label: 'Wednesday' },
  { value: '4', label: 'Thursday' },
  { value: '5', label: 'Friday' },
  { value: '6', label: 'Saturday' },
];

const HOURS = Array.from({ length: 24 }, (_, i) => ({
  value: String(i),
  label: `${String(i).padStart(2, '0')}:00`,
}));

const MINUTES = [
  { value: '0', label: ':00' },
  { value: '15', label: ':15' },
  { value: '30', label: ':30' },
  { value: '45', label: ':45' },
];

// ─────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────

/**
 * Validate a 5-field UNIX cron expression — mirrors the backend authority
 * `validate_cron_expression` (backend/servers/api-server/src/routes/reports.rs).
 *
 * Field-parse order is load-bearing and MUST match the backend exactly to avoid
 * the #1368 cross-validator drift: split on `,` FIRST (a field is a list of
 * parts), then on `/` (step), then on `-` (range). Parsing `/` before `,` makes
 * the combined form `1-5/2,10` a false-negative — and because the read path
 * (`scheduleToInitialCron`) gates surfacing the persisted cron on this
 * validator, a false-negative silently flattens a backend-accepted expression
 * back to the legacy `time`-derived form, reintroducing #616 for that subset.
 */
export function isValidCron(expr: string): boolean {
  const fields = expr.trim().split(/\s+/);
  if (fields.length !== 5) return false;

  function validField(field: string, min: number, max: number): boolean {
    // A field is a comma-separated list of parts (split on `,` FIRST — same as
    // the backend). Each part is an optional `base/step`.
    for (const part of field.split(',')) {
      const slash = part.indexOf('/');
      const base = slash === -1 ? part : part.slice(0, slash);
      const step = slash === -1 ? undefined : part.slice(slash + 1);

      // step must be a positive integer when present
      if (step !== undefined) {
        const s = Number(step);
        if (!Number.isInteger(s) || s <= 0) return false;
      }

      if (base !== '*') {
        if (base.includes('-')) {
          // range: x-y
          const [a, b] = base.split('-').map(Number);
          if (!Number.isInteger(a) || !Number.isInteger(b) || a > b) return false;
          if (a < min || b > max) return false;
        } else {
          // single number
          const n = Number(base);
          if (!Number.isInteger(n) || n < min || n > max) return false;
        }
      }
    }
    return true;
  }

  return (
    validField(fields[0], 0, 59) && // minute
    validField(fields[1], 0, 23) && // hour
    validField(fields[2], 1, 31) && // dom
    validField(fields[3], 1, 12) && // month
    validField(fields[4], 0, 7) // dow (0 and 7 are both Sunday)
  );
}

/** Derive the SimplePreset from a cron expression, falling back to "custom". */
export function detectPreset(cron: string): SimplePreset {
  const fields = cron.trim().split(/\s+/);
  if (fields.length !== 5) return 'custom';
  const [, , dom, month, dow] = fields;

  if (month === '*') {
    if (dom === '*' && dow === '*') return 'daily';
    if (dom === '*' && /^\d+$/.test(dow)) return 'weekly';
    if (/^\d+$/.test(dom) && dow === '*') return 'monthly';
  }
  return 'custom';
}

/** Build a cron string from simple preset parts. */
export function buildCron(
  preset: SimplePreset,
  minute: string,
  hour: string,
  dow: string,
  dom: string
): string {
  switch (preset) {
    case 'daily':
      return `${minute} ${hour} * * *`;
    case 'weekly':
      return `${minute} ${hour} * * ${dow}`;
    case 'monthly':
      return `${minute} ${hour} ${dom} * *`;
    default:
      return '';
  }
}

/** Parse hour and minute from the first two fields of a cron expression. */
function parseCronTime(cron: string): { hour: string; minute: string } {
  const fields = cron.trim().split(/\s+/);
  if (fields.length < 2) return { hour: '8', minute: '0' };
  const minute = /^\d+$/.test(fields[0]) ? fields[0] : '0';
  const hour = /^\d+$/.test(fields[1]) ? fields[1] : '8';
  return { hour, minute };
}

/** Parse day-of-week from field 5 (index 4). */
function parseCronDow(cron: string): string {
  const fields = cron.trim().split(/\s+/);
  if (fields.length < 5) return '1';
  return /^\d+$/.test(fields[4]) ? fields[4] : '1';
}

/** Parse day-of-month from field 3 (index 2). */
function parseCronDom(cron: string): string {
  const fields = cron.trim().split(/\s+/);
  if (fields.length < 3) return '1';
  return /^\d+$/.test(fields[2]) ? fields[2] : '1';
}

// ─────────────────────────────────────────────────────────────
// Component
// ─────────────────────────────────────────────────────────────

export function CronPicker({ value, onChange, error, disabled }: CronPickerProps) {
  const [preset, setPreset] = useState<SimplePreset>(() => (value ? detectPreset(value) : 'daily'));
  const [hour, setHour] = useState(() => parseCronTime(value).hour);
  const [minute, setMinute] = useState(() => parseCronTime(value).minute);
  const [dow, setDow] = useState(() => parseCronDow(value));
  const [dom, setDom] = useState(() => parseCronDom(value));
  const [customExpr, setCustomExpr] = useState(value || '');
  const [customValid, setCustomValid] = useState(true);

  // Sync internal state when `value` changes externally
  useEffect(() => {
    if (!value) return;
    const detectedPreset = detectPreset(value);
    setPreset(detectedPreset);
    const { hour: h, minute: m } = parseCronTime(value);
    setHour(h);
    setMinute(m);
    setDow(parseCronDow(value));
    setDom(parseCronDom(value));
    if (detectedPreset === 'custom') {
      setCustomExpr(value);
    }
  }, [value]);

  // Emit updated cron expression for simple presets
  const emitSimple = useCallback(
    (p: SimplePreset, h: string, m: string, d: string, dm: string) => {
      if (p === 'custom') return;
      const cron = buildCron(p, m, h, d, dm);
      if (cron) onChange(cron);
    },
    [onChange]
  );

  const handlePresetChange = useCallback(
    (newPreset: SimplePreset) => {
      setPreset(newPreset);
      if (newPreset !== 'custom') {
        emitSimple(newPreset, hour, minute, dow, dom);
      }
    },
    [hour, minute, dow, dom, emitSimple]
  );

  const handleHourChange = useCallback(
    (h: string) => {
      setHour(h);
      emitSimple(preset, h, minute, dow, dom);
    },
    [preset, minute, dow, dom, emitSimple]
  );

  const handleMinuteChange = useCallback(
    (m: string) => {
      setMinute(m);
      emitSimple(preset, hour, m, dow, dom);
    },
    [preset, hour, dow, dom, emitSimple]
  );

  const handleDowChange = useCallback(
    (d: string) => {
      setDow(d);
      emitSimple(preset, hour, minute, d, dom);
    },
    [preset, hour, minute, dom, emitSimple]
  );

  const handleDomChange = useCallback(
    (dm: string) => {
      const n = Number(dm);
      if (Number.isNaN(n) || n < 1 || n > 31) return;
      setDom(dm);
      emitSimple(preset, hour, minute, dow, dm);
    },
    [preset, hour, minute, dow, emitSimple]
  );

  const handleCustomChange = useCallback(
    (expr: string) => {
      setCustomExpr(expr);
      const valid = isValidCron(expr);
      setCustomValid(valid);
      if (valid) {
        onChange(expr.trim());
      }
    },
    [onChange]
  );

  // Current expression preview (for display)
  const previewCron = preset === 'custom' ? customExpr : buildCron(preset, minute, hour, dow, dom);

  const selectClass = `block w-full px-3 py-2 border border-gray-300 rounded-md shadow-sm text-sm
    focus:outline-none focus:ring-blue-500 focus:border-blue-500
    disabled:bg-gray-100 disabled:cursor-not-allowed`;

  return (
    <div className="space-y-4">
      {/* Preset tabs */}
      <div className="flex gap-2 flex-wrap">
        {(Object.keys(PRESETS) as SimplePreset[]).map((p) => (
          <button
            key={p}
            type="button"
            disabled={disabled}
            onClick={() => handlePresetChange(p)}
            className={`px-3 py-1.5 text-sm font-medium rounded-md transition-colors
              disabled:opacity-50 disabled:cursor-not-allowed
              ${
                preset === p
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-100 text-gray-700 hover:bg-gray-200'
              }`}
          >
            {PRESETS[p].label}
          </button>
        ))}
      </div>

      {/* Description */}
      <p className="text-xs text-gray-500">{PRESETS[preset].description}</p>

      {/* Simple preset controls */}
      {preset !== 'custom' && (
        <div className="grid grid-cols-2 gap-4">
          {/* Hour */}
          <div>
            <label htmlFor="cron-hour" className="block text-xs font-medium text-gray-700 mb-1">
              Hour
            </label>
            <select
              id="cron-hour"
              value={hour}
              onChange={(e) => handleHourChange(e.target.value)}
              disabled={disabled}
              className={selectClass}
            >
              {HOURS.map((h) => (
                <option key={h.value} value={h.value}>
                  {h.label}
                </option>
              ))}
            </select>
          </div>

          {/* Minute */}
          <div>
            <label htmlFor="cron-minute" className="block text-xs font-medium text-gray-700 mb-1">
              Minute
            </label>
            <select
              id="cron-minute"
              value={minute}
              onChange={(e) => handleMinuteChange(e.target.value)}
              disabled={disabled}
              className={selectClass}
            >
              {MINUTES.map((m) => (
                <option key={m.value} value={m.value}>
                  {m.label}
                </option>
              ))}
            </select>
          </div>

          {/* Day of week (weekly only) */}
          {preset === 'weekly' && (
            <div className="col-span-2">
              <label htmlFor="cron-dow" className="block text-xs font-medium text-gray-700 mb-1">
                Day of week
              </label>
              <select
                id="cron-dow"
                value={dow}
                onChange={(e) => handleDowChange(e.target.value)}
                disabled={disabled}
                className={selectClass}
              >
                {DAYS_OF_WEEK.map((d) => (
                  <option key={d.value} value={d.value}>
                    {d.label}
                  </option>
                ))}
              </select>
            </div>
          )}

          {/* Day of month (monthly only) */}
          {preset === 'monthly' && (
            <div className="col-span-2">
              <label htmlFor="cron-dom" className="block text-xs font-medium text-gray-700 mb-1">
                Day of month (1 – 31)
              </label>
              <input
                id="cron-dom"
                type="number"
                min="1"
                max="31"
                value={dom}
                onChange={(e) => handleDomChange(e.target.value)}
                disabled={disabled}
                className={`${selectClass} ${disabled ? '' : 'bg-white'}`}
              />
              <p className="mt-1 text-xs text-gray-400">
                For shorter months the schedule runs on the last available day.
              </p>
            </div>
          )}
        </div>
      )}

      {/* Custom cron input */}
      {preset === 'custom' && (
        <div>
          <label
            htmlFor="cron-custom-expr"
            className="block text-xs font-medium text-gray-700 mb-1"
          >
            Cron expression{' '}
            <span className="text-gray-400 font-normal">
              (minute hour day-of-month month day-of-week)
            </span>
          </label>
          <input
            id="cron-custom-expr"
            type="text"
            value={customExpr}
            onChange={(e) => handleCustomChange(e.target.value)}
            disabled={disabled}
            placeholder="0 8 * * 1"
            className={`block w-full px-3 py-2 border rounded-md shadow-sm text-sm font-mono
              focus:outline-none focus:ring-blue-500 focus:border-blue-500
              disabled:bg-gray-100 disabled:cursor-not-allowed
              ${!customValid && customExpr ? 'border-red-400' : 'border-gray-300'}`}
          />
          {!customValid && customExpr && (
            <p className="mt-1 text-xs text-red-600">
              Invalid cron — use 5 fields, e.g. <code className="font-mono">0 8 * * 1</code>
            </p>
          )}
          <p className="mt-1 text-xs text-gray-400">
            Examples: <code className="font-mono">0 8 * * *</code> (daily 8am) &nbsp;·&nbsp;{' '}
            <code className="font-mono">30 17 * * 5</code> (Fri 17:30) &nbsp;·&nbsp;{' '}
            <code className="font-mono">0 6 1 * *</code> (1st of month 6am)
          </p>
        </div>
      )}

      {/* Expression preview */}
      {previewCron && (
        <div className="flex items-center gap-2 px-3 py-2 bg-gray-50 rounded-md border border-gray-200">
          <span className="text-xs text-gray-500">Expression:</span>
          <code className="text-xs font-mono text-gray-800">{previewCron}</code>
        </div>
      )}

      {/* External error */}
      {error && <p className="text-sm text-red-600">{error}</p>}
    </div>
  );
}
