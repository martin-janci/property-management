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

import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { Manifest, SectionConfig } from './api';
import {
  ADD_ROW_STYLE,
  badgeStyle,
  btnStyle,
  ERROR_STYLE,
  ROW_HEADER_STYLE,
  ROW_STYLE,
  SELECT_STYLE,
  TEXTAREA_STYLE,
  TYPE_LABEL_STYLE,
} from './SectionTreeEditor.styles';

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
  highlightType?: string;
}

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
  highlightType,
}: Props) {
  const { t } = useTranslation();

  // Local state: in-flight props edit for the one section currently being edited.
  // Addressed by INDEX (not type): duplicate types — possible via rollback or
  // server data — would otherwise make edits hit all copies at once.
  // When null (or index doesn't match), the textarea derives its value from the
  // sections prop — so external updates (draft reload, rollback) always win.
  const [editingProps, setEditingProps] = useState<{
    index: number;
    text: string;
    error: boolean;
  } | null>(null);

  // Add-section select state
  const [addType, setAddType] = useState('');

  // Refs for each row — used to scroll to the highlighted row
  const rowRefs = useRef<Record<string, HTMLDivElement | null>>({});

  // Scroll highlighted row into view when highlightType changes
  useEffect(() => {
    if (!highlightType) return;
    const el = rowRefs.current[highlightType];
    el?.scrollIntoView?.({ behavior: 'smooth', block: 'nearest' });
  }, [highlightType]);

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

  function toggleVisible(index: number) {
    onChange(sections.map((s, i) => (i === index ? { ...s, visible: !(s.visible ?? true) } : s)));
  }

  function changeMode(index: number, mode: string) {
    onChange(sections.map((s, i) => (i === index ? { ...s, mode } : s)));
  }

  function removeAt(index: number) {
    onChange(sections.filter((_, i) => i !== index));
  }

  function handlePropsFocus(index: number, section: SectionConfig) {
    // Start editing: seed text from prop (or current in-flight text if already editing this row)
    if (editingProps?.index !== index) {
      setEditingProps({ index, text: JSON.stringify(section.props ?? {}, null, 2), error: false });
    }
  }

  function handlePropsChange(index: number, text: string) {
    setEditingProps({ index, text, error: false });
  }

  function commitProps(index: number) {
    const raw =
      editingProps?.index === index
        ? editingProps.text
        : JSON.stringify(sections[index]?.props ?? {}, null, 2);
    try {
      const parsed = JSON.parse(raw) as Record<string, unknown>;
      setEditingProps(null);
      onChange(sections.map((s, i) => (i === index ? { ...s, props: parsed } : s)));
    } catch {
      setEditingProps((prev) => (prev?.index === index ? { ...prev, error: true } : prev));
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
        const isEditingThis = editingProps?.index === idx;
        const hasPropsError = isEditingThis && editingProps.error;
        const currentPropsText = isEditingThis
          ? editingProps.text
          : JSON.stringify(section.props ?? {}, null, 2);

        const isHighlighted = highlightType === type;

        return (
          <div
            // Keyed by index: duplicate types (possible via rollback/server
            // data) would otherwise produce duplicate React keys.
            key={idx}
            ref={(el) => {
              rowRefs.current[type] = el;
            }}
            data-testid={`section-row-${type}`}
            style={{
              ...ROW_STYLE,
              ...(isHighlighted
                ? { outline: '2px solid var(--ppt-primary-600, #2563eb)', outlineOffset: 2 }
                : {}),
            }}
          >
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
                  onClick={() => toggleVisible(idx)}
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
                  onClick={() => removeAt(idx)}
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
                    onChange={(e) => changeMode(idx, e.target.value)}
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
                onFocus={() => handlePropsFocus(idx, section)}
                onChange={(e) => handlePropsChange(idx, e.target.value)}
                onBlur={() => commitProps(idx)}
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
