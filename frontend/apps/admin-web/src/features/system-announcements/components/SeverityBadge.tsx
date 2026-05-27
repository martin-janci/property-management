import type { SystemAnnouncementSeverity } from '@ppt/api-client';
import { getSeverityStyle } from '../lib/severity';

export function SeverityBadge({ severity }: { severity: SystemAnnouncementSeverity }) {
  const style = getSeverityStyle(severity);
  return (
    <span
      style={{
        display: 'inline-block',
        padding: '2px 8px',
        borderRadius: 9999,
        fontSize: 11,
        fontWeight: 600,
        background: style.bg,
        color: style.fg,
        border: `1px solid ${style.border}`,
        textTransform: 'uppercase',
        letterSpacing: '0.04em',
      }}
    >
      {severity}
    </span>
  );
}
