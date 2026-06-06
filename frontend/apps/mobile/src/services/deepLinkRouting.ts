/**
 * Deep-link routing (pure logic).
 *
 * Extracted from `App.tsx` (gap-85-3) so the parsed-deep-link → in-app
 * screen+params mapping can be unit-tested without mounting the RN tree.
 * Mirrors the `backgroundNotifications.deepLinkForNotification` split: the
 * navigation layer (`App`) just calls this and hands the result to
 * `handleNavigate`.
 */

import type { ParsedDeepLink } from '../qrcode';

/**
 * In-app screen names understood by `handleNavigate`. Kept in sync with the
 * `Screen` union in `App.tsx`.
 */
export type Screen =
  | 'Dashboard'
  | 'Faults'
  | 'ReportFault'
  | 'Announcements'
  | 'AnnouncementDetail'
  | 'Voting'
  | 'VoteDetail'
  | 'Documents'
  | 'DocumentDetail'
  | 'DocumentPreview'
  | 'DocumentUpload'
  | 'DocumentPermissions'
  | 'Messages'
  | 'ThreadDetail'
  | 'Settings'
  | 'MeterReading'
  | 'Meters'
  | 'MeterDetail'
  | 'Profile'
  | 'TwoFactor'
  | 'Neighbors'
  | 'Notifications'
  | 'PersonMonths'
  | 'Outages'
  | 'News'
  | 'Forms'
  | 'Buildings'
  | 'Leases'
  | 'LeaseDetail'
  | 'LeaseSignature'
  | 'More';

/**
 * gap-85-3: translate a parsed deep link (custom scheme OR universal link)
 * into the in-app screen name + params understood by `handleNavigate`.
 *
 * The DeepLinkHandler route table keys detail params as `id`; the App screens
 * expect domain-specific param names (`announcementId`, `voteId`,
 * `documentId`). When an `:id` is present we route to the matching *Detail
 * screen; otherwise we land on the list screen.
 */
export function resolveDeepLinkTarget(
  link: ParsedDeepLink
): { screen: Screen; params?: Record<string, unknown> } | null {
  if (!link.success || !link.screen) {
    return null;
  }
  const id = link.params?.id;
  switch (link.screen) {
    case 'Announcements':
      return id
        ? { screen: 'AnnouncementDetail', params: { announcementId: id } }
        : { screen: 'Announcements' };
    case 'Voting':
      return id ? { screen: 'VoteDetail', params: { voteId: id } } : { screen: 'Voting' };
    case 'Documents':
      return id
        ? { screen: 'DocumentDetail', params: { documentId: id } }
        : { screen: 'Documents' };
    case 'Messages':
      return id ? { screen: 'ThreadDetail', params: { threadId: id } } : { screen: 'Messages' };
    case 'WidgetSettings':
      return { screen: 'Settings' };
    default:
      // Faults, ReportFault, Outages, Settings, Dashboard map 1:1. Pass any
      // remaining route params through verbatim.
      return { screen: link.screen as Screen, params: link.params };
  }
}
