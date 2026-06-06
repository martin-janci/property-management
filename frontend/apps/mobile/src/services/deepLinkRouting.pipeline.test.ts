/**
 * End-to-end pipeline regression test for the deep-link -> document-detail
 * navigation payload path wired in `App.tsx` (the path that churned across
 * PR #992 and its predecessors).
 *
 * `deepLinkRouting.test.ts` exercises `resolveDeepLinkTarget` in isolation with
 * a *hand-built* `ParsedDeepLink`. That leaves a gap: `App.tsx` never builds a
 * `ParsedDeepLink` by hand — at runtime the `deepLinkManager` feeds it the
 * output of `parseDeepLink(url)`. A drift in EITHER stage would silently break
 * document deep links while keeping the isolated unit test green, e.g.:
 *   - the `documents/:id` route table entry changing its param name away from
 *     `id` (parse stage), or
 *   - `resolveDeepLinkTarget` renaming the `documentId` payload key (resolve
 *     stage) — the key `App.tsx`'s `DocumentDetail` case reads as `documentId`.
 *
 * This test pins the WHOLE chain a tapped link travels:
 *   raw URL  ->  parseDeepLink  ->  resolveDeepLinkTarget  ->  { screen, params }
 * for both the cold-start custom-scheme link and the HTTPS universal link, so
 * the payload that `handleNavigate` ultimately hands to `DocumentDetailScreen`
 * is locked down on both sides of the boundary.
 *
 * Pure-logic test (no `@testing-library/react-native` render), matching the
 * `deepLinkRouting.test.ts` / `backgroundNotifications.test.ts` style.
 */

import { parseDeepLink } from '../qrcode';
import { resolveDeepLinkTarget } from './deepLinkRouting';

/** Run the exact runtime chain: parse the URL, then resolve to a nav target. */
function navigateTargetFor(url: string) {
  return resolveDeepLinkTarget(parseDeepLink(url));
}

describe('deep-link -> DocumentDetail payload pipeline (PR #992 churn guard)', () => {
  it('routes a custom-scheme documents/:id link to DocumentDetail + documentId', () => {
    expect(navigateTargetFor('ppt://documents/doc-123')).toEqual({
      screen: 'DocumentDetail',
      params: { documentId: 'doc-123' },
    });
  });

  it('routes the alternate custom scheme (ppt-management) the same way', () => {
    expect(navigateTargetFor('ppt-management://documents/doc-123')).toEqual({
      screen: 'DocumentDetail',
      params: { documentId: 'doc-123' },
    });
  });

  it('routes an HTTPS universal-link documents/:id to DocumentDetail + documentId', () => {
    // app.ppt.example.com is the DEFAULT_APP_LINK_HOST used in tests.
    expect(navigateTargetFor('https://app.ppt.example.com/documents/doc-123')).toEqual({
      screen: 'DocumentDetail',
      params: { documentId: 'doc-123' },
    });
  });

  it('hands DocumentDetailScreen exactly `documentId` (not `id`) so its query is enabled', () => {
    const target = navigateTargetFor('ppt://documents/doc-123');

    // Guards against the payload key drifting back to `id`, which would leave
    // DocumentDetailScreen with `documentId === undefined` and its detail query
    // disabled — a silent dead deep link.
    expect(target?.params).toHaveProperty('documentId', 'doc-123');
    expect(target?.params).not.toHaveProperty('id');
  });

  it('routes a documents link without an id to the Documents list (no detail nav)', () => {
    expect(navigateTargetFor('ppt://documents')).toEqual({ screen: 'Documents' });
  });

  it('yields no navigation target for a URL this app does not own', () => {
    // parseDeepLink fails -> resolveDeepLinkTarget returns null -> App does not navigate.
    expect(navigateTargetFor('https://evil.example.com/documents/doc-123')).toBeNull();
    expect(navigateTargetFor('not-a-url')).toBeNull();
  });
});
