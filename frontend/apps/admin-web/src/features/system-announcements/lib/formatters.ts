/**
 * Date / status formatting helpers for the system-announcements admin page.
 * Extracted from SystemAnnouncementsPage.tsx (#521) so the page only
 * orchestrates and these are individually testable.
 */

import type { SystemAnnouncement } from '@ppt/api-client';

export function formatDatetime(iso: string): string {
  try {
    return new Intl.DateTimeFormat('en-GB', {
      dateStyle: 'medium',
      timeStyle: 'short',
    }).format(new Date(iso));
  } catch {
    return iso;
  }
}

export function toLocalDatetimeInput(iso: string | null | undefined): string {
  if (!iso) return '';
  try {
    const d = new Date(iso);
    const pad = (n: number) => String(n).padStart(2, '0');
    return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
  } catch {
    return '';
  }
}

export function toIso(localDatetime: string): string {
  if (!localDatetime) return '';
  return new Date(localDatetime).toISOString();
}

export function isActive(ann: SystemAnnouncement, now: number = Date.now()): boolean {
  const start = new Date(ann.start_at).getTime();
  const end = ann.end_at ? new Date(ann.end_at).getTime() : Number.POSITIVE_INFINITY;
  return start <= now && now < end;
}

export function isScheduled(ann: SystemAnnouncement, now: number = Date.now()): boolean {
  return new Date(ann.start_at).getTime() > now;
}

export type AnnouncementLifecycle = 'active' | 'scheduled' | 'expired';

export function lifecycleOf(
  ann: SystemAnnouncement,
  now: number = Date.now()
): AnnouncementLifecycle {
  if (isActive(ann, now)) return 'active';
  if (isScheduled(ann, now)) return 'scheduled';
  return 'expired';
}
