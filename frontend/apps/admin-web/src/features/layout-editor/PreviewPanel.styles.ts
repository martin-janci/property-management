/**
 * Presentational style constants for PreviewPanel.
 *
 * Extracted from the component so the bridge/resolve lifecycle logic reads
 * without the inline CSSProperties noise (same pattern as
 * LayoutEditorPage.styles.ts, #2464). Plain module constants — no behaviour.
 * Inline, --ppt-* token vars — admin-web house style.
 */

import type React from 'react';

export const PANEL_STYLE: React.CSSProperties = {
  display: 'flex',
  flexDirection: 'column',
  gap: 12,
};

export const ROW_STYLE: React.CSSProperties = {
  display: 'flex',
  gap: 8,
  alignItems: 'center',
  flexWrap: 'wrap',
};

export const INPUT_STYLE: React.CSSProperties = {
  flex: 1,
  minWidth: 240,
  padding: '6px 10px',
  fontSize: 14,
  borderRadius: 6,
  border: '1px solid var(--ppt-border-default, #e5e7eb)',
};

export const BTN_STYLE: React.CSSProperties = {
  padding: '6px 14px',
  fontSize: 14,
  borderRadius: 6,
  cursor: 'pointer',
  border: '1px solid var(--ppt-border-default, #e5e7eb)',
  fontWeight: 500,
  background: 'var(--ppt-bg-subtle, #f3f4f6)',
  color: 'var(--ppt-fg-primary, #111827)',
};

export const ERROR_STYLE: React.CSSProperties = {
  fontSize: 12,
  color: 'var(--ppt-danger-600, #dc2626)',
};

export const NOTE_STYLE: React.CSSProperties = {
  fontSize: 12,
  color: 'var(--ppt-warning-700, #b45309)',
};

export const IFRAME_STYLE: React.CSSProperties = {
  width: '100%',
  height: 600,
  border: '1px solid var(--ppt-border-default, #e5e7eb)',
  borderRadius: 8,
};
