/**
 * Regression tests for the deep-link → in-app screen+params mapping.
 *
 * Pins the document-detail deep-link payload path wired in PR #992: a parsed
 * `Documents` deep link carrying an `id` must resolve to the `DocumentDetail`
 * screen with `params: { documentId: id }` — the exact shape `App.tsx` hands
 * to `handleNavigate` and that `DocumentDetailScreen` reads as `documentId`.
 *
 * Pure-logic test (no `@testing-library/react-native`), matching the
 * `backgroundNotifications.test.ts` style — the routing was extracted from the
 * `App` component (gap-85-3) so it can be exercised without mounting the tree.
 */

import type { ParsedDeepLink } from '../qrcode';
import { resolveDeepLinkTarget } from './deepLinkRouting';

function link(partial: Partial<ParsedDeepLink>): ParsedDeepLink {
  return { success: true, ...partial };
}

describe('resolveDeepLinkTarget — document-detail payload (PR #992)', () => {
  it('routes a Documents deep link with an id to DocumentDetail + documentId param', () => {
    const target = resolveDeepLinkTarget(link({ screen: 'Documents', params: { id: 'doc-123' } }));

    expect(target).toEqual({
      screen: 'DocumentDetail',
      params: { documentId: 'doc-123' },
    });
  });

  it('uses the param key `documentId` (the key DocumentDetailScreen reads)', () => {
    const target = resolveDeepLinkTarget(link({ screen: 'Documents', params: { id: 'abc' } }));

    // Guards against a silent regression where the payload key drifts (e.g.
    // back to `id`), which would leave DocumentDetailScreen with no id and the
    // detail query disabled.
    expect(target?.params).toHaveProperty('documentId', 'abc');
    expect(target?.params).not.toHaveProperty('id');
  });

  it('routes a Documents deep link without an id to the Documents list', () => {
    const target = resolveDeepLinkTarget(link({ screen: 'Documents' }));

    expect(target).toEqual({ screen: 'Documents' });
    expect(target?.params).toBeUndefined();
  });
});

describe('resolveDeepLinkTarget — guards & sibling detail routes', () => {
  it('returns null for an unsuccessful parse', () => {
    expect(resolveDeepLinkTarget(link({ success: false, screen: 'Documents' }))).toBeNull();
  });

  it('returns null when no screen was resolved', () => {
    expect(resolveDeepLinkTarget(link({ screen: undefined }))).toBeNull();
  });

  it('maps the other detail screens to their domain-specific param keys', () => {
    expect(resolveDeepLinkTarget(link({ screen: 'Announcements', params: { id: '42' } }))).toEqual({
      screen: 'AnnouncementDetail',
      params: { announcementId: '42' },
    });

    expect(resolveDeepLinkTarget(link({ screen: 'Voting', params: { id: '9' } }))).toEqual({
      screen: 'VoteDetail',
      params: { voteId: '9' },
    });

    expect(resolveDeepLinkTarget(link({ screen: 'Messages', params: { id: '3' } }))).toEqual({
      screen: 'ThreadDetail',
      params: { threadId: '3' },
    });
  });

  it('passes 1:1 routes through verbatim with their params', () => {
    expect(resolveDeepLinkTarget(link({ screen: 'Faults', params: { id: '7' } }))).toEqual({
      screen: 'Faults',
      params: { id: '7' },
    });
  });

  it('folds WidgetSettings onto the Settings screen', () => {
    expect(resolveDeepLinkTarget(link({ screen: 'WidgetSettings' }))).toEqual({
      screen: 'Settings',
    });
  });
});
