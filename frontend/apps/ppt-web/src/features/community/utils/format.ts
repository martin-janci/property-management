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
  // Validate the string is numeric, but do NOT keep the coerced double: a
  // server `Decimal` can exceed Number.MAX_SAFE_INTEGER, so round-tripping it
  // through IEEE-754 would lose precision (issue #3310). We format the exact
  // decimal string instead.
  if (!Number.isFinite(Number(amount))) return null;
  const code = currency || 'EUR';
  try {
    // Intl.NumberFormat#format accepts a string/BigInt at runtime (ES2020+) and
    // formats it at full decimal precision — unlike a Number() round-trip. The
    // TS lib types the parameter as `number | bigint`, hence the narrow cast.
    return new Intl.NumberFormat(locale, {
      style: 'currency',
      currency: code,
    }).format(amount as unknown as number);
  } catch {
    // Unknown currency code etc. — fall back to the raw decimal (still no
    // float round-trip).
    return `${amount} ${code}`;
  }
}

/**
 * Display name for a community author / seller.
 *
 * Prefers a `resolvedName` when the caller has one (e.g. from an author
 * projection embedded in the DTO or a batch user-lookup), honouring the
 * `isAnonymous` flag first and the current viewer second. Falls back to the
 * generic "neighbor" label only when a name is genuinely unavailable — which
 * is still the case wherever the API exposes ids only (see issue #3310).
 */
export function personLabel(
  personId: string,
  currentUserId: string | undefined,
  t: TFunction,
  isAnonymous = false,
  resolvedName?: string | null
): string {
  if (isAnonymous) return t('community.hub.anonymous');
  if (currentUserId && personId === currentUserId) return t('community.hub.you');
  const name = resolvedName?.trim();
  if (name) return name;
  return t('community.hub.neighbor');
}
