/**
 * Unit tests for the ppt-web content-moderation decision analytics helpers.
 * Locks in the canonical `moderation.*` event names + payloads and the shared
 * bus guarantees (dataLayer mirroring, never-throws, no rationale/PII leakage).
 */
/// <reference types="vitest/globals" />
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import {
  __clearAnalyticsSinks,
  type AnalyticsEvent,
  registerAnalyticsSink,
} from '../../lib/analytics';
import {
  MODERATION_EVENTS,
  trackModerationActionTaken,
  trackModerationAppealDecided,
} from './analytics';

let captured: AnalyticsEvent[];

beforeEach(() => {
  captured = [];
  registerAnalyticsSink((e) => captured.push(e));
});

afterEach(() => {
  __clearAnalyticsSinks();
  (globalThis as { dataLayer?: unknown[] }).dataLayer = undefined;
});

describe('ppt-web moderation decision analytics', () => {
  it('fires action_taken with the typed action union', () => {
    trackModerationActionTaken('remove');
    expect(captured).toHaveLength(1);
    expect(captured[0].event).toBe(MODERATION_EVENTS.actionTaken);
    expect(captured[0].properties).toEqual({ action_type: 'remove' });
    expect(captured[0].timestamp).toEqual(expect.any(String));
  });

  it('fires appeal_decided with the typed decision union', () => {
    trackModerationAppealDecided('overturn');
    expect(captured).toHaveLength(1);
    expect(captured[0].event).toBe(MODERATION_EVENTS.appealDecided);
    expect(captured[0].properties).toEqual({ decision: 'overturn' });
  });

  it('carries only the decision shape — no rationale text or PII', () => {
    trackModerationActionTaken('warn');
    trackModerationAppealDecided('uphold');
    for (const e of captured) {
      expect(Object.keys(e.properties).sort()).not.toContain('rationale');
      expect(Object.keys(e.properties).length).toBe(1);
    }
  });

  it('uses the canonical data-team schema names', () => {
    expect(MODERATION_EVENTS).toEqual({
      actionTaken: 'moderation.action_taken',
      appealDecided: 'moderation.appeal_decided',
    });
  });

  it('mirrors events onto window.dataLayer when present', () => {
    (globalThis as { dataLayer?: unknown[] }).dataLayer = [];
    trackModerationActionTaken('restrict');
    const dl = (globalThis as { dataLayer?: Array<Record<string, unknown>> }).dataLayer;
    expect(dl).toHaveLength(1);
    expect(dl?.[0]).toMatchObject({
      event: MODERATION_EVENTS.actionTaken,
      action_type: 'restrict',
    });
  });

  it('never throws when a sink misbehaves', () => {
    registerAnalyticsSink(() => {
      throw new Error('boom');
    });
    expect(() => trackModerationActionTaken('approve')).not.toThrow();
    expect(captured).toHaveLength(1);
  });
});
