import type { SystemAnnouncementSeverity } from '@ppt/api-client';
import { getSeverityStyle } from '../lib/severity';

interface BannerPreviewProps {
  title: string;
  message: string;
  severity: SystemAnnouncementSeverity;
  isDismissible: boolean;
}

export function BannerPreview({ title, message, severity, isDismissible }: BannerPreviewProps) {
  const style = getSeverityStyle(severity);
  return (
    <div
      role="alert"
      style={{
        background: style.bg,
        border: `1px solid ${style.border}`,
        borderRadius: 8,
        padding: '12px 16px',
        display: 'flex',
        alignItems: 'flex-start',
        gap: 12,
      }}
    >
      <div style={{ flex: 1 }}>
        <div style={{ fontWeight: 600, color: style.fg, fontSize: 14 }}>
          {title || <em style={{ opacity: 0.5 }}>Announcement title</em>}
        </div>
        {message && (
          <div style={{ fontSize: 13, marginTop: 4, color: style.fg, opacity: 0.9 }}>{message}</div>
        )}
      </div>
      {isDismissible && (
        <button
          type="button"
          aria-label="Dismiss (preview only)"
          style={{
            background: 'none',
            border: 'none',
            cursor: 'default',
            color: style.fg,
            fontSize: 16,
            opacity: 0.6,
            padding: 0,
          }}
        >
          ×
        </button>
      )}
    </div>
  );
}
