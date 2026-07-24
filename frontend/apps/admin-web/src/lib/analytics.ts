/**
 * Lightweight, transport-agnostic analytics event bus for admin-web.
 *
 * The app has no bundled analytics SDK (PostHog / Segment / GA are injected
 * at deploy time, if at all). This module gives feature code a single,
 * dependency-free `trackEvent` entry point that:
 *
 *   1. pushes a GTM-style record onto `window.dataLayer` when one is present
 *      (the deploy-time tag manager consumes it), and
 *   2. fans the event out to any in-process sinks registered via
 *      `registerAnalyticsSink` (used by tests and by future in-app dashboards).
 *
 * Analytics must never break the UX: every sink call is wrapped so a throwing
 * sink can't bubble into a React event handler.
 */

export interface AnalyticsEvent {
  /** Dot-namespaced event name, e.g. `onboarding.tour_completed`. */
  event: string;
  /** Arbitrary, JSON-serialisable event properties. */
  properties: Record<string, unknown>;
  /** ISO-8601 capture timestamp. */
  timestamp: string;
}

export type AnalyticsSink = (event: AnalyticsEvent) => void;

const sinks = new Set<AnalyticsSink>();

/**
 * Register an in-process analytics sink. Returns an unsubscribe function.
 */
export function registerAnalyticsSink(sink: AnalyticsSink): () => void {
  sinks.add(sink);
  return () => {
    sinks.delete(sink);
  };
}

/** Test/reset hook — drops all registered in-process sinks. */
export function __clearAnalyticsSinks(): void {
  sinks.clear();
}

interface DataLayerWindow {
  dataLayer?: unknown[];
}

/**
 * Emit an analytics event. Safe to call from any render/event-handler path —
 * it never throws.
 */
export function trackEvent(event: string, properties: Record<string, unknown> = {}): void {
  const payload: AnalyticsEvent = {
    event,
    properties,
    timestamp: new Date().toISOString(),
  };

  // 1. GTM-style dataLayer (deploy-time tag manager), if present.
  try {
    const dl = (globalThis as DataLayerWindow).dataLayer;
    if (Array.isArray(dl)) {
      dl.push({ event, ...properties, timestamp: payload.timestamp });
    }
  } catch {
    // Never let analytics transport break the caller.
  }

  // 2. In-process sinks.
  for (const sink of sinks) {
    try {
      sink(payload);
    } catch {
      // Swallow — a broken sink must not break the UX.
    }
  }
}
