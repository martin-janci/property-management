/**
 * Severity colour tokens for system-announcement banner / chip styling.
 * Extracted from SystemAnnouncementsPage.tsx (#521).
 */

export interface SeverityStyle {
  bg: string;
  fg: string;
  border: string;
}

export const SEVERITY_STYLE: Record<string, SeverityStyle> = {
  info: { bg: '#eff6ff', fg: '#1e40af', border: '#bfdbfe' },
  warning: { bg: '#fffbeb', fg: '#92400e', border: '#fde68a' },
  critical: { bg: '#fef2f2', fg: '#991b1b', border: '#fecaca' },
};

export function getSeverityStyle(severity: string): SeverityStyle {
  return SEVERITY_STYLE[severity] ?? SEVERITY_STYLE.info;
}
