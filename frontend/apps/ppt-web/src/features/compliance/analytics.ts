/**
 * Content-moderation decision instrumentation for the ppt-web (manager) SPA.
 *
 * The data team wants a single, canonical signal each time a moderator commits
 * a regulated decision on a content case — a take-action verdict
 * (approve / warn / restrict / remove) or an appeal decision (uphold /
 * overturn). These helpers emit one `moderation.*` event per decision, fired
 * ONLY after the corresponding server mutation succeeds, so events count real,
 * audited decisions rather than dialog submits that the API later rejected.
 *
 * Follows the same pattern as `features/auth/analytics.ts`: a thin,
 * side-effect-only wrapper over the shared `trackEvent` bus
 * (`lib/analytics.ts`). The event names are the public schema contract — keep
 * them stable and in sync with the data-team moderation funnel. Properties
 * carry only the decision shape (the typed action / decision union), never the
 * rationale text, the content body, or any owner PII.
 */

import type { DecideAppealRequest, TakeModerationActionRequest } from '@ppt/api-client';
import { trackEvent } from '../../lib/analytics';

/** The typed moderation action union (`approve | warn | restrict | remove`). */
type ModerationActionType = TakeModerationActionRequest['action_type'];
/** The typed appeal-decision union (`uphold | overturn`). */
type AppealDecision = DecideAppealRequest['decision'];

/** Canonical moderation-decision event names — the data-team schema contract. */
export const MODERATION_EVENTS = {
  actionTaken: 'moderation.action_taken',
  appealDecided: 'moderation.appeal_decided',
} as const;

export type ModerationEventName = (typeof MODERATION_EVENTS)[keyof typeof MODERATION_EVENTS];

/**
 * A moderator recorded a take-action verdict on a content case, accepted by the
 * server (Epic 67 / Epic 90). `action_type` is the typed decision union; no
 * rationale text or content/owner PII is attached.
 */
export function trackModerationActionTaken(actionType: ModerationActionType): void {
  trackEvent(MODERATION_EVENTS.actionTaken, { action_type: actionType });
}

/**
 * A moderator decided an appeal on a content case, accepted by the server.
 * `decision` is the typed appeal-decision union; no rationale text or PII is
 * attached.
 */
export function trackModerationAppealDecided(decision: AppealDecision): void {
  trackEvent(MODERATION_EVENTS.appealDecided, { decision });
}
