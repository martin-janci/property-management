/**
 * RailsEditor — pure controlled component (Task 3, Layout Editor MVP)
 *
 * Props:
 *   rails        — current Rails config
 *   sectionTypes — ordered list of known section type strings
 *   onChange     — called with the full updated Rails object
 *
 * Local state ONLY for in-progress whitelist text per section type; everything
 * else derives from props. External `rails` prop updates always win when the
 * user is not actively editing that type's input (single editingWhitelist slot,
 * same derive-unless-editing discipline as SectionTreeEditor's props textarea).
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { Rails } from './api';
import {
  GLOBAL_ROW_STYLE,
  INPUT_STYLE,
  LABEL_STYLE,
  SECTION_STYLE,
  TABLE_STYLE,
  TD_STYLE,
  TH_STYLE,
  TYPE_LABEL_STYLE,
} from './RailsEditor.styles';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface Props {
  rails: Rails;
  sectionTypes: string[];
  onChange: (next: Rails) => void;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function parseWhitelist(text: string): string[] {
  return text
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean);
}

function formatWhitelist(list: string[]): string {
  return list.join(', ');
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function RailsEditor({ rails, sectionTypes, onChange }: Props) {
  const { t } = useTranslation();

  // In-flight whitelist edit: { type, text } or null when not editing.
  // When null (or type doesn't match), the input derives its value from rails.
  const [editingWhitelist, setEditingWhitelist] = useState<{
    type: string;
    text: string;
  } | null>(null);

  // -------------------------------------------------------------------------
  // Mutation helpers — always call onChange with a new Rails object
  // -------------------------------------------------------------------------

  function toggleReorderable() {
    onChange({ ...rails, reorderable: !rails.reorderable });
  }

  function toggleHideable(type: string) {
    const next = rails.hideable.includes(type)
      ? rails.hideable.filter((t) => t !== type)
      : [...rails.hideable, type];
    onChange({ ...rails, hideable: next });
  }

  function toggleModeEditable(type: string) {
    const next = rails.mode_editable.includes(type)
      ? rails.mode_editable.filter((t) => t !== type)
      : [...rails.mode_editable, type];
    onChange({ ...rails, mode_editable: next });
  }

  function handleWhitelistChange(type: string, text: string) {
    setEditingWhitelist({ type, text });
  }

  function commitWhitelist(type: string) {
    const raw = editingWhitelist?.type === type ? editingWhitelist.text : null;
    const text = raw ?? formatWhitelist(rails.prop_whitelist[type] ?? []);
    const parsed = parseWhitelist(text);
    setEditingWhitelist(null);

    const nextWhitelist = { ...rails.prop_whitelist };
    if (parsed.length === 0) {
      delete nextWhitelist[type];
    } else {
      nextWhitelist[type] = parsed;
    }
    onChange({ ...rails, prop_whitelist: nextWhitelist });
  }

  // -------------------------------------------------------------------------
  // Render
  // -------------------------------------------------------------------------

  return (
    <div style={SECTION_STYLE}>
      {/* Global: reorderable checkbox */}
      <div style={GLOBAL_ROW_STYLE}>
        <input
          type="checkbox"
          id="reorderable-checkbox"
          data-testid="reorderable-checkbox"
          checked={rails.reorderable}
          onChange={toggleReorderable}
        />
        <label htmlFor="reorderable-checkbox" style={LABEL_STYLE}>
          {t('admin.layout.rails.reorderable', { defaultValue: 'Reorderable' })}
        </label>
      </div>

      {/* Per-type table */}
      {sectionTypes.length > 0 && (
        <div style={TABLE_STYLE}>
          <table style={{ width: '100%', borderCollapse: 'collapse' }}>
            <thead>
              <tr>
                <th style={TH_STYLE}>
                  {t('admin.layout.rails.typeColumn', { defaultValue: 'Section type' })}
                </th>
                <th style={{ ...TH_STYLE, textAlign: 'center' }}>
                  {t('admin.layout.rails.hideableColumn', { defaultValue: 'Hideable' })}
                </th>
                <th style={{ ...TH_STYLE, textAlign: 'center' }}>
                  {t('admin.layout.rails.modeEditableColumn', { defaultValue: 'Mode editable' })}
                </th>
                <th style={TH_STYLE}>
                  {t('admin.layout.rails.whitelistColumn', {
                    defaultValue: 'Prop whitelist (comma-separated)',
                  })}
                </th>
              </tr>
            </thead>
            <tbody>
              {sectionTypes.map((type, idx) => {
                const isEditingThis = editingWhitelist?.type === type;
                const whitelistText = isEditingThis
                  ? editingWhitelist.text
                  : formatWhitelist(rails.prop_whitelist[type] ?? []);
                const isLast = idx === sectionTypes.length - 1;
                const tdStyle = isLast ? { ...TD_STYLE, borderBottom: 'none' } : TD_STYLE;

                return (
                  <tr key={type}>
                    <td style={tdStyle}>
                      <span style={TYPE_LABEL_STYLE}>{type}</span>
                    </td>
                    <td style={{ ...tdStyle, textAlign: 'center' }}>
                      <input
                        type="checkbox"
                        data-testid={`hideable-${type}`}
                        checked={rails.hideable.includes(type)}
                        onChange={() => toggleHideable(type)}
                        aria-label={t('admin.layout.rails.hideableLabel', {
                          defaultValue: `Hideable: ${type}`,
                        })}
                      />
                    </td>
                    <td style={{ ...tdStyle, textAlign: 'center' }}>
                      <input
                        type="checkbox"
                        data-testid={`mode-editable-${type}`}
                        checked={rails.mode_editable.includes(type)}
                        onChange={() => toggleModeEditable(type)}
                        aria-label={t('admin.layout.rails.modeEditableLabel', {
                          defaultValue: `Mode editable: ${type}`,
                        })}
                      />
                    </td>
                    <td style={tdStyle}>
                      <input
                        type="text"
                        data-testid={`whitelist-${type}`}
                        value={whitelistText}
                        onChange={(e) => handleWhitelistChange(type, e.target.value)}
                        onBlur={() => commitWhitelist(type)}
                        style={INPUT_STYLE}
                        placeholder={t('admin.layout.rails.whitelistPlaceholder', {
                          defaultValue: 'e.g. title, limit',
                        })}
                      />
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
