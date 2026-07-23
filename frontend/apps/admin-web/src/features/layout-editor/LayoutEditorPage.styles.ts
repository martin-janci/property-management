/**
 * Presentational style constants for LayoutEditorPage.
 *
 * Extracted from the page component so the wiring logic (queries, mutations,
 * seeding effect, handlers) reads without ~115 lines of inline CSSProperties
 * noise. These are plain module constants — no behaviour, no state.
 */

import type React from 'react';

export const PAGE_STYLE: React.CSSProperties = {
  padding: '24px',
  maxWidth: 960,
  margin: '0 auto',
  display: 'flex',
  flexDirection: 'column',
  gap: 24,
};

export const CARD_STYLE: React.CSSProperties = {
  border: '1px solid var(--ppt-border-default, #e5e7eb)',
  borderRadius: 'var(--ppt-radius-lg, 10px)',
  padding: '20px 24px',
  background: 'var(--ppt-bg-surface, #fff)',
  display: 'flex',
  flexDirection: 'column',
  gap: 16,
};

export const CARD_TITLE_STYLE: React.CSSProperties = {
  fontSize: 16,
  fontWeight: 600,
  color: 'var(--ppt-fg-primary, #111827)',
  margin: 0,
};

export const ROW_STYLE: React.CSSProperties = {
  display: 'flex',
  gap: 8,
  alignItems: 'center',
  flexWrap: 'wrap',
};

export const SELECT_STYLE: React.CSSProperties = {
  padding: '6px 10px',
  fontSize: 14,
  borderRadius: 6,
  border: '1px solid var(--ppt-border-default, #e5e7eb)',
  minWidth: 200,
};

export const INPUT_STYLE: React.CSSProperties = {
  padding: '6px 10px',
  fontSize: 14,
  borderRadius: 6,
  border: '1px solid var(--ppt-border-default, #e5e7eb)',
  minWidth: 240,
};

export const BTN_BASE: React.CSSProperties = {
  padding: '6px 14px',
  fontSize: 14,
  borderRadius: 6,
  cursor: 'pointer',
  border: '1px solid transparent',
  fontWeight: 500,
};

export const BTN_PRIMARY: React.CSSProperties = {
  ...BTN_BASE,
  background: 'var(--ppt-primary-600, #2563eb)',
  color: '#fff',
  borderColor: 'var(--ppt-primary-600, #2563eb)',
};

export const BTN_SECONDARY: React.CSSProperties = {
  ...BTN_BASE,
  background: 'var(--ppt-bg-surface, #fff)',
  color: 'var(--ppt-fg-primary, #111827)',
  borderColor: 'var(--ppt-border-default, #e5e7eb)',
};

export const BTN_DANGER: React.CSSProperties = {
  ...BTN_BASE,
  background: 'var(--ppt-danger-600, #dc2626)',
  color: '#fff',
  borderColor: 'var(--ppt-danger-600, #dc2626)',
};

export const ALERT_STYLE: React.CSSProperties = {
  background: 'var(--ppt-danger-50, #fef2f2)',
  border: '1px solid var(--ppt-danger-200, #fecaca)',
  borderRadius: 8,
  padding: '12px 16px',
  color: 'var(--ppt-danger-800, #991b1b)',
};

export const TABLE_STYLE: React.CSSProperties = {
  width: '100%',
  borderCollapse: 'collapse',
  fontSize: 13,
};

export const TH_STYLE: React.CSSProperties = {
  textAlign: 'left',
  padding: '6px 10px',
  fontWeight: 600,
  color: 'var(--ppt-fg-secondary, #6b7280)',
  borderBottom: '1px solid var(--ppt-border-default, #e5e7eb)',
};

export const TD_STYLE: React.CSSProperties = {
  padding: '6px 10px',
  borderBottom: '1px solid var(--ppt-border-default, #e5e7eb)',
  verticalAlign: 'middle',
};

export const TOGGLE_STYLE: React.CSSProperties = {
  display: 'flex',
  gap: 4,
  border: '1px solid var(--ppt-border-default, #e5e7eb)',
  borderRadius: 6,
  overflow: 'hidden',
};
