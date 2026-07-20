/**
 * SectionTreeEditor — pure controlled component (Task 2, Layout Editor MVP)
 *
 * Props:
 *   sections  — ordered list of SectionConfig in the draft
 *   manifest  — manifest for the current platform (nullable while loading)
 *   kills     — array of killed section types
 *   onChange  — called with the full updated sections array
 *   onKill    — called with the section type to kill
 *   onUnkill  — called with the section type to unkill
 *
 * Local state ONLY for in-progress props textarea text (per section type) and
 * per-section JSON parse error flag; everything else derives from props.
 */

import type React from 'react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { Manifest, SectionConfig } from './api';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface Props {
  sections: SectionConfig[];
  manifest: Manifest | null;
  kills: string[];
  onChange: (next: SectionConfig[]) => void;
  onKill: (type: string) => void;
  onUnkill: (type: string) => void;
}

// ---------------------------------------------------------------------------
// Styles (inline, --ppt-* token vars — admin-web house style)
// ---------------------------------------------------------------------------

const ROW_STYLE: React.CSSProperties = {
  border: '1px solid var(--ppt-border-default, #e5e7eb)',
  borderRadius: 'var(--ppt-radius-lg, 10px)',
  padding: '12px 16px',
  display: 'flex',
  flexDirection: 'column',
  gap: 8,
  background: 'var(--ppt-bg-surface, #fff)',
};

const ROW_HEADER_STYLE: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: 8,
  flexWrap: 'wrap',
};

const TYPE_LABEL_STYLE: React.CSSProperties = {
  fontFamily: 'monospace',
  fontSize: 13,
  fontWeight: 600,
  color: 'var(--ppt-fg-primary, #111827)',
};

const TEXTAREA_STYLE: React.CSSProperties = {
  width: '100%',
  minHeight: 80,
  fontFamily: 'monospace',
  fontSize: 12,
  padding: 8,
  border: '1px solid var(--ppt-border-default, #e5e7eb)',
  borderRadius: 6,
  resize: 'vertical',
  boxSizing: 'border-box',
};

const ERROR_STYLE: React.CSSProperties = {
  color: 'var(--ppt-danger-600, #dc2626)',
  fontSize: 12,
  marginTop: 2,
};

const ADD_ROW_STYLE: React.CSSProperties = {
  display: 'flex',
  gap: 8,
  alignItems: 'center',
  paddingTop: 8,
  borderTop: '1px solid var(--ppt-border-default, #e5e7eb)',
  marginTop: 8,
};

// Inline replacements for ui-kit Badge / Button (admin-web uses no ui-kit)
const BADGE_BASE: React.CSSProperties = {
  display: 'inline-flex',
  alignItems: 'center',
  borderRadius: 4,
  padding: '1px 6px',
  fontSize: 11,
  fontWeight: 600,
  lineHeight: '18px',
};

const BADGE_VARIANT: Record<string, React.CSSProperties> = {
  secondary: {
    background: 'var(--ppt-bg-subtle, #f3f4f6)',
    color: 'var(--ppt-fg-secondary, #6b7280)',
  },
  warning: {
    background: 'var(--ppt-warning-100, #fef3c7)',
    color: 'var(--ppt-warning-700, #b45309)',
  },
  danger: { background: 'var(--ppt-danger-100, #fee2e2)', color: 'var(--ppt-danger-700, #b91c1c)' },
};

const BTN_BASE: React.CSSProperties = {
  display: 'inline-flex',
  alignItems: 'center',
  justifyContent: 'center',
  borderRadius: 6,
  padding: '3px 10px',
  fontSize: 12,
  fontWeight: 500,
  cursor: 'pointer',
  border: '1px solid transparent',
  lineHeight: '18px',
  background: 'none',
};

const BTN_VARIANT: Record<string, React.CSSProperties> = {
  ghost: { color: 'var(--ppt-fg-secondary, #6b7280)', border: '1px solid transparent' },
  danger: {
    background: 'var(--ppt-danger-600, #dc2626)',
    color: '#fff',
    border: '1px solid var(--ppt-danger-700, #b91c1c)',
  },
  secondary: {
    background: 'var(--ppt-bg-subtle, #f3f4f6)',
    color: 'var(--ppt-fg-primary, #111827)',
    border: '1px solid var(--ppt-border-default, #e5e7eb)',
  },
  primary: {
    background: 'var(--ppt-primary-600, #2563eb)',
    color: '#fff',
    border: '1px solid var(--ppt-primary-700, #1d4ed8)',
  },
};

function badgeStyle(variant: string): React.CSSProperties {
  return { ...BADGE_BASE, ...(BADGE_VARIANT[variant] ?? {}) };
}

function btnStyle(variant: string, disabled?: boolean): React.CSSProperties {
  return {
    ...BTN_BASE,
    ...(BTN_VARIANT[variant] ?? {}),
    ...(disabled ? { opacity: 0.4, cursor: 'not-allowed' } : {}),
  };
}

const SELECT_STYLE: React.CSSProperties = {
  padding: '5px 8px',
  borderRadius: 6,
  border: '1px solid var(--ppt-border-default, #e5e7eb)',
  fontSize: 13,
};

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function SectionTreeEditor({
  sections,
  manifest,
  kills,
  onChange,
  onKill,
  onUnkill,
}: Props) {
  const { t } = useTranslation();

  // Local state: in-flight props edit for the one section currently being edited.
  // When null (or type doesn't match), the textarea derives its value from the
  // sections prop — so external updates (draft reload, rollback) always win.
  const [editingProps, setEditingProps] = useState<{
    type: string;
    text: string;
    error: boolean;
  } | null>(null);

  // Add-section select state
  const [addType, setAddType] = useState('');

  // -------------------------------------------------------------------------
  // Helpers
  // -------------------------------------------------------------------------

  function getComponent(type: string) {
    return manifest?.components[type] ?? null;
  }

  function isRequired(type: string): boolean {
    return getComponent(type)?.required === true;
  }

  function isKilled(type: string): boolean {
    return kills.includes(type);
  }

  function isUnknown(type: string): boolean {
    return manifest !== null && !(type in (manifest?.components ?? {}));
  }

  // -------------------------------------------------------------------------
  // Mutation helpers — always call onChange with a new array
  // -------------------------------------------------------------------------

  function reorder(from: number, to: number) {
    const next = [...sections];
    const [item] = next.splice(from, 1);
    next.splice(to, 0, item);
    onChange(next);
  }

  function toggleVisible(type: string) {
    onChange(sections.map((s) => (s.type === type ? { ...s, visible: !(s.visible ?? true) } : s)));
  }

  function changeMode(type: string, mode: string) {
    onChange(sections.map((s) => (s.type === type ? { ...s, mode } : s)));
  }

  function handlePropsFocus(type: string, section: SectionConfig) {
    // Start editing: seed text from prop (or current in-flight text if already editing this type)
    if (editingProps?.type !== type) {
      setEditingProps({ type, text: JSON.stringify(section.props ?? {}, null, 2), error: false });
    }
  }

  function handlePropsChange(type: string, text: string) {
    setEditingProps({ type, text, error: false });
  }

  function commitProps(type: string) {
    const raw =
      editingProps?.type === type
        ? editingProps.text
        : JSON.stringify(sections.find((s) => s.type === type)?.props ?? {}, null, 2);
    try {
      const parsed = JSON.parse(raw) as Record<string, unknown>;
      setEditingProps(null);
      onChange(sections.map((s) => (s.type === type ? { ...s, props: parsed } : s)));
    } catch {
      setEditingProps((prev) => (prev?.type === type ? { ...prev, error: true } : prev));
    }
  }

  function handleKill(type: string) {
    if (
      window.confirm(
        t('admin.layout.killConfirm', {
          defaultValue: `Kill section "${type}"? This will hide it for all users.`,
        })
      )
    ) {
      onKill(type);
    }
  }

  function handleUnkill(type: string) {
    if (
      window.confirm(
        t('admin.layout.unkillConfirm', {
          defaultValue: `Unkill section "${type}"? It will become visible again.`,
        })
      )
    ) {
      onUnkill(type);
    }
  }

  function handleAdd() {
    if (!addType) return;
    onChange([...sections, { type: addType, visible: true }]);
    setAddType('');
  }

  // -------------------------------------------------------------------------
  // Add-select: manifest types not already present
  // -------------------------------------------------------------------------

  const presentTypes = new Set(sections.map((s) => s.type));
  const addableTypes = manifest
    ? Object.keys(manifest.components).filter((typeKey) => !presentTypes.has(typeKey))
    : [];

  // -------------------------------------------------------------------------
  // Render
  // -------------------------------------------------------------------------

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      {sections.map((section, idx) => {
        const { type } = section;
        const component = getComponent(type);
        const required = isRequired(type);
        const killed = isKilled(type);
        const unknown = isUnknown(type);
        const supportedModes = component?.supported_modes;
        const isEditingThis = editingProps?.type === type;
        const hasPropsError = isEditingThis && editingProps.error;
        const currentPropsText = isEditingThis
          ? editingProps.text
          : JSON.stringify(section.props ?? {}, null, 2);

        return (
          <div key={type} style={ROW_STYLE}>
            {/* Header row */}
            <div style={ROW_HEADER_STYLE}>
              {/* Type label */}
              <span style={TYPE_LABEL_STYLE}>{type}</span>

              {/* Required lock badge — required sections have NO hide control */}
              {required && (
                <span style={badgeStyle('secondary')} data-testid={`lock-badge-${type}`}>
                  {t('admin.layout.requiredBadge', { defaultValue: 'Required' })}
                </span>
              )}

              {/* Unknown-type warning badge */}
              {unknown && (
                <span style={badgeStyle('warning')} data-testid={`unknown-badge-${type}`}>
                  {t('admin.layout.unknownBadge', { defaultValue: 'Unknown type' })}
                </span>
              )}

              {/* Killed badge */}
              {killed && (
                <span style={badgeStyle('danger')} data-testid={`killed-badge-${type}`}>
                  {t('admin.layout.killedBadge', { defaultValue: 'Killed' })}
                </span>
              )}

              {/* Eye toggle — only for optional sections */}
              {!required && (
                <button
                  type="button"
                  style={btnStyle('ghost')}
                  data-testid={`hide-btn-${type}`}
                  onClick={() => toggleVisible(type)}
                  title={
                    (section.visible ?? true)
                      ? t('admin.layout.hideSection', { defaultValue: 'Hide section' })
                      : t('admin.layout.showSection', { defaultValue: 'Show section' })
                  }
                >
                  {(section.visible ?? true) ? '👁' : '🙈'}
                </button>
              )}

              {/* Kill / Unkill button */}
              {killed ? (
                <button
                  type="button"
                  style={btnStyle('danger')}
                  data-testid={`unkill-btn-${type}`}
                  onClick={() => handleUnkill(type)}
                >
                  {t('admin.layout.unkillBtn', { defaultValue: 'Unkill' })}
                </button>
              ) : (
                <button
                  type="button"
                  style={btnStyle('danger')}
                  data-testid={`kill-btn-${type}`}
                  onClick={() => handleKill(type)}
                >
                  {t('admin.layout.killBtn', { defaultValue: 'Kill' })}
                </button>
              )}

              {/* Remove button — optional sections only */}
              {!required && (
                <button
                  type="button"
                  style={btnStyle('secondary')}
                  data-testid={`remove-btn-${type}`}
                  onClick={() => onChange(sections.filter((s) => s.type !== type))}
                >
                  {t('admin.layout.removeBtn', { defaultValue: 'Remove' })}
                </button>
              )}

              {/* Reorder buttons */}
              <button
                type="button"
                style={btnStyle('ghost', idx === 0)}
                data-testid={`move-up-${type}`}
                disabled={idx === 0}
                onClick={() => reorder(idx, idx - 1)}
                aria-label={t('admin.layout.moveUp', { defaultValue: 'Move up' })}
              >
                ↑
              </button>
              <button
                type="button"
                style={btnStyle('ghost', idx === sections.length - 1)}
                data-testid={`move-down-${type}`}
                disabled={idx === sections.length - 1}
                onClick={() => reorder(idx, idx + 1)}
                aria-label={t('admin.layout.moveDown', { defaultValue: 'Move down' })}
              >
                ↓
              </button>
            </div>

            {/* Mode select — only when manifest entry has supported_modes */}
            {supportedModes && supportedModes.length > 0 && (
              <div>
                <label style={{ fontSize: 12, color: 'var(--ppt-fg-secondary, #6b7280)' }}>
                  {t('admin.layout.mode', { defaultValue: 'Mode' })}
                  <select
                    data-testid={`mode-select-${type}`}
                    value={section.mode ?? component?.default_mode ?? supportedModes[0]}
                    onChange={(e) => changeMode(type, e.target.value)}
                    style={{ ...SELECT_STYLE, marginLeft: 6 }}
                  >
                    {supportedModes.map((m) => (
                      <option key={m} value={m}>
                        {m}
                      </option>
                    ))}
                  </select>
                </label>
              </div>
            )}

            {/* Props textarea */}
            <div>
              <label style={{ fontSize: 12, color: 'var(--ppt-fg-secondary, #6b7280)' }}>
                {t('admin.layout.props', { defaultValue: 'Props (JSON)' })}
              </label>
              <textarea
                data-testid={`props-textarea-${type}`}
                value={currentPropsText}
                onFocus={() => handlePropsFocus(type, section)}
                onChange={(e) => handlePropsChange(type, e.target.value)}
                onBlur={() => commitProps(type)}
                style={{
                  ...TEXTAREA_STYLE,
                  borderColor: hasPropsError
                    ? 'var(--ppt-danger-600, #dc2626)'
                    : 'var(--ppt-border-default, #e5e7eb)',
                }}
                spellCheck={false}
                aria-invalid={hasPropsError}
              />
              {hasPropsError && (
                <div data-testid={`props-error-${type}`} role="alert" style={ERROR_STYLE}>
                  {t('admin.layout.propsInvalidJson', { defaultValue: 'Invalid JSON' })}
                </div>
              )}
            </div>
          </div>
        );
      })}

      {/* Add section row */}
      {manifest && addableTypes.length > 0 && (
        <div style={ADD_ROW_STYLE}>
          <select
            data-testid="add-section-select"
            value={addType}
            onChange={(e) => setAddType(e.target.value)}
            style={SELECT_STYLE}
          >
            <option value="">
              {t('admin.layout.addSectionPlaceholder', { defaultValue: '— select type —' })}
            </option>
            {addableTypes.map((tp) => (
              <option key={tp} value={tp}>
                {tp}
              </option>
            ))}
          </select>
          <button
            type="button"
            style={btnStyle('primary', !addType)}
            data-testid="add-section-btn"
            disabled={!addType}
            onClick={handleAdd}
          >
            {t('admin.layout.addSectionBtn', { defaultValue: 'Add section' })}
          </button>
        </div>
      )}
    </div>
  );
}
