/**
 * Anonymous analytics session context for reality-web.
 *
 * Conversion metrics need to stitch a visitor's listing views together across
 * a single browsing session without identifying the person. This module mints
 * a random, per-tab session id on first use and persists it in `sessionStorage`
 * (cleared when the tab closes — deliberately shorter-lived than a tracking
 * cookie, and gated by the same "no PII" contract as the rest of the portal).
 *
 * Everything here is SSR-safe: on the server (or when storage is unavailable /
 * blocked) it degrades to an ephemeral in-memory id rather than throwing, so a
 * caller in a React effect never has to guard the environment itself.
 */

/** `sessionStorage` key holding the anonymous session id. */
export const ANALYTICS_SESSION_KEY = 'ppt-analytics-session';

/** Shape handed to event helpers so views can be grouped per session. */
export interface AnalyticsSessionContext {
  /** Random per-tab identifier — not tied to any user account. */
  sessionId: string;
  /** Referrer URL captured at emit time (may be empty). */
  referrer: string;
}

/** In-memory fallback id when `sessionStorage` is unavailable (SSR, privacy). */
let ephemeralSessionId: string | null = null;

function randomId(): string {
  try {
    const c = (globalThis as { crypto?: Crypto }).crypto;
    if (c?.randomUUID) return c.randomUUID();
  } catch {
    // Fall through to the Math.random path below.
  }
  return `sess-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
}

/**
 * Return the current anonymous session id, creating and persisting one on first
 * use. Never throws — falls back to an ephemeral id when storage is blocked.
 */
export function getSessionId(): string {
  try {
    const storage = (globalThis as { sessionStorage?: Storage }).sessionStorage;
    if (storage) {
      const existing = storage.getItem(ANALYTICS_SESSION_KEY);
      if (existing) return existing;
      const fresh = randomId();
      storage.setItem(ANALYTICS_SESSION_KEY, fresh);
      return fresh;
    }
  } catch {
    // sessionStorage can throw in private mode / when disabled — degrade below.
  }
  if (!ephemeralSessionId) ephemeralSessionId = randomId();
  return ephemeralSessionId;
}

/** Best-effort document referrer; empty string on the server. */
export function getReferrer(): string {
  try {
    return (globalThis as { document?: Document }).document?.referrer ?? '';
  } catch {
    return '';
  }
}

/** Assemble the session context attached to every analytics event. */
export function getSessionContext(): AnalyticsSessionContext {
  return { sessionId: getSessionId(), referrer: getReferrer() };
}

/** Test hook — drop the in-memory fallback id. */
export function __resetEphemeralSessionId(): void {
  ephemeralSessionId = null;
}
