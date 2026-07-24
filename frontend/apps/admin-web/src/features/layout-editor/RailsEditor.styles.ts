/**
 * Presentational style constants for RailsEditor.
 *
 * Extracted from the component so the controlled-input logic reads without the
 * inline CSSProperties noise (same pattern as LayoutEditorPage.styles.ts, #2464).
 * These are plain module constants — no behaviour, no state.
 * Inline, --ppt-* token vars — admin-web house style.
 */

import type React from 'react';

export const SECTION_STYLE: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: 16,
};

export const GLOBAL_ROW_STYLE: React.CSSProperties = {
  border: '1px solid var(--ppt-border-default, #e5e7eb)',
  borderRadius: 'var(--ppt-radius-lg, 10px)',
  padding: '12px 16px',
  background: 'var(--ppt-bg-surface, #fff)',
  display: 'flex',
  alignItems: 'center',
  gap: 8,
};

export const TABLE_STYLE: React.CSSProperties = {
  border: '1px solid var(--ppt-border-default, #e5e7eb)',
  borderRadius: 'var(--ppt-radius-lg, 10px)',
  background: 'var(--ppt-bg-surface, #fff)',
  overflow: 'hidden',
};

export const TH_STYLE: React.CSSProperties = {
  padding: '8px 12px',
  textAlign: 'left',
  fontSize: 12,
  fontWeight: 600,
  color: 'var(--ppt-fg-secondary, #6b7280)',
  borderBottom: '1px solid var(--ppt-border-default, #e5e7eb)',
  background: 'var(--ppt-bg-muted, #f9fafb)',
};

export const TD_STYLE: React.CSSProperties = {
  padding: '8px 12px',
  fontSize: 13,
  borderBottom: '1px solid var(--ppt-border-default, #e5e7eb)',
  verticalAlign: 'middle',
};

export const TYPE_LABEL_STYLE: React.CSSProperties = {
  fontFamily: 'monospace',
  fontSize: 13,
  fontWeight: 600,
  color: 'var(--ppt-fg-primary, #111827)',
};

export const INPUT_STYLE: React.CSSProperties = {
  width: '100%',
  padding: '4px 8px',
  fontSize: 12,
  fontFamily: 'monospace',
  border: '1px solid var(--ppt-border-default, #e5e7eb)',
  borderRadius: 6,
  boxSizing: 'border-box',
};

export const LABEL_STYLE: React.CSSProperties = {
  fontSize: 13,
  fontWeight: 500,
  color: 'var(--ppt-fg-primary, #111827)',
};
