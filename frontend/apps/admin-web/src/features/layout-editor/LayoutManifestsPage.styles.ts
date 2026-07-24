/**
 * Presentational style constants for LayoutManifestsPage.
 *
 * Extracted from the page so the list query / upload-validation logic reads
 * without the inline CSSProperties noise (same pattern as
 * LayoutEditorPage.styles.ts, #2464). Plain module constants — no behaviour.
 */

import type React from 'react';

export const PAGE_STYLE: React.CSSProperties = {
  padding: '24px',
  maxWidth: 900,
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
  verticalAlign: 'top',
};

export const PRE_STYLE: React.CSSProperties = {
  margin: '8px 0 0 0',
  padding: '8px',
  background: 'var(--ppt-bg-muted, #f9fafb)',
  border: '1px solid var(--ppt-border-default, #e5e7eb)',
  borderRadius: 6,
  fontSize: 11,
  overflowX: 'auto',
  maxHeight: 200,
  overflowY: 'auto',
};

export const SELECT_STYLE: React.CSSProperties = {
  padding: '6px 10px',
  fontSize: 14,
  borderRadius: 6,
  border: '1px solid var(--ppt-border-default, #e5e7eb)',
  minWidth: 140,
};

export const TEXTAREA_STYLE: React.CSSProperties = {
  width: '100%',
  minHeight: 160,
  padding: '8px 10px',
  fontSize: 13,
  fontFamily: 'monospace',
  borderRadius: 6,
  border: '1px solid var(--ppt-border-default, #e5e7eb)',
  resize: 'vertical',
  boxSizing: 'border-box',
};

export const BTN_PRIMARY: React.CSSProperties = {
  padding: '6px 16px',
  fontSize: 14,
  borderRadius: 6,
  cursor: 'pointer',
  border: '1px solid transparent',
  fontWeight: 500,
  background: 'var(--ppt-primary-600, #2563eb)',
  color: '#fff',
  borderColor: 'var(--ppt-primary-600, #2563eb)',
};

export const ALERT_STYLE: React.CSSProperties = {
  background: 'var(--ppt-danger-50, #fef2f2)',
  border: '1px solid var(--ppt-danger-200, #fecaca)',
  borderRadius: 8,
  padding: '10px 14px',
  color: 'var(--ppt-danger-800, #991b1b)',
  fontSize: 13,
};

export const HINT_STYLE: React.CSSProperties = {
  fontSize: 12,
  color: 'var(--ppt-fg-secondary, #6b7280)',
  fontFamily: 'monospace',
};
