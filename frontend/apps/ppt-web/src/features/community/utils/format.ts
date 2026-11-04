/**
 * Locale-aware formatting helpers for the Community pages.
 */

import type { TFunction } from 'i18next';

export function formatDateTime(iso: string, locale: string, allDay = false): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return new Intl.DateTimeFormat(
    locale,
    allDay ? { dateStyle: 'medium' } : { dateStyle: 'medium', timeStyle: 'short' }
  ).format(d);
}

export function formatDate(iso: string, locale: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return new Intl.DateTimeFormat(locale, { dateStyle: 'medium' }).format(d);
}

/** "just now" / "5m ago" / "3h ago" / "2d ago", then an absolute date. */
export function formatRelative(iso: string, locale: string, t: TFunction, now = Date.now()) {
  const then = new Date(iso).getTime();
  if (Number.isNaN(then)) return iso;
  const minutes = Math.floor((now - then) / 60_000);
  if (minutes < 1) return t('community.post.justNow');
  if (minutes < 60) return t('community.post.minutesAgo', { count: minutes });
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return t('community.post.hoursAgo', { count: hours });
  const days = Math.floor(hours / 24);
  if (days < 7) return t('community.post.daysAgo', { count: days });
  return formatDate(iso, locale);
}

/** Format a server Decimal string as money; `null` when there is no usable price. */
export function formatMoney(
  amount: string | null,
  currency: string | null,
  locale: string
): string | null {
  if (amount === null || amount === '') return null;
  const value = Number(amount);
  if (!Number.isFinite(value)) return null;
  try {
    return new Intl.NumberFormat(locale, {
      style: 'currency',
      currency: currency || 'EUR',
    }).format(value);
  } catch {
    return `${value.toFixed(2)} ${currency || 'EUR'}`;
  }
}

/** Display name for a community author / seller (the API exposes ids only). */
export function personLabel(
  personId: string,
  currentUserId: string | undefined,
  t: TFunction,
  isAnonymous = false
): string {
  if (isAnonymous) return t('community.hub.anonymous');
  if (currentUserId && personId === currentUserId) return t('community.hub.you');
  return t('community.hub.neighbor');
}
