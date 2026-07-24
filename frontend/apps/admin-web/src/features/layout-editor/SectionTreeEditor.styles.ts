/**
 * Presentational style constants + helpers for SectionTreeEditor.
 *
 * Extracted from the component so the controlled-list logic (reorder, kill,
 * props editing) reads without the inline CSSProperties noise (same pattern as
 * LayoutEditorPage.styles.ts, #2464). The badgeStyle/btnStyle helpers are pure
 * variant->CSSProperties functions — no state, no behaviour.
 * Inline, --ppt-* token vars — admin-web house style.
 */

import type React from 'react';

export const ROW_STYLE: React.CSSProperties = {
  border: '1px solid var(--ppt-border-default, #e5e7eb)',
  borderRadius: 'var(--ppt-radius-lg, 10px)',
  padding: '12px 16px',
  display: 'flex',
  flexDirection: 'column',
  gap: 8,
  background: 'var(--ppt-bg-surface, #fff)',
};

export const ROW_HEADER_STYLE: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  gap: 8,
  flexWrap: 'wrap',
};

export const TYPE_LABEL_STYLE: React.CSSProperties = {
  fontFamily: 'monospace',
  fontSize: 13,
  fontWeight: 600,
  color: 'var(--ppt-fg-primary, #111827)',
};

export const TEXTAREA_STYLE: React.CSSProperties = {
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

export const ERROR_STYLE: React.CSSProperties = {
  color: 'var(--ppt-danger-600, #dc2626)',
  fontSize: 12,
  marginTop: 2,
};

export const ADD_ROW_STYLE: React.CSSProperties = {
  display: 'flex',
  gap: 8,
  alignItems: 'center',
  paddingTop: 8,
  borderTop: '1px solid var(--ppt-border-default, #e5e7eb)',
  marginTop: 8,
};

// Inline replacements for ui-kit Badge / Button (admin-web uses no ui-kit)
export const BADGE_BASE: React.CSSProperties = {
  display: 'inline-flex',
  alignItems: 'center',
  borderRadius: 4,
  padding: '1px 6px',
  fontSize: 11,
  fontWeight: 600,
  lineHeight: '18px',
};

export const BADGE_VARIANT: Record<string, React.CSSProperties> = {
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

export const BTN_BASE: React.CSSProperties = {
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

export const BTN_VARIANT: Record<string, React.CSSProperties> = {
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

export function badgeStyle(variant: string): React.CSSProperties {
  return { ...BADGE_BASE, ...(BADGE_VARIANT[variant] ?? {}) };
}

export function btnStyle(variant: string, disabled?: boolean): React.CSSProperties {
  return {
    ...BTN_BASE,
    ...(BTN_VARIANT[variant] ?? {}),
    ...(disabled ? { opacity: 0.4, cursor: 'not-allowed' } : {}),
  };
}

export const SELECT_STYLE: React.CSSProperties = {
  padding: '5px 8px',
  borderRadius: 6,
  border: '1px solid var(--ppt-border-default, #e5e7eb)',
  fontSize: 13,
};
