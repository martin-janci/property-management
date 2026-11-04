/**
 * Facility type values (Epic 56: Facility Booking).
 *
 * Labels are localized via `facilities.types.<value>` in the message bundles.
 */

import type { FacilityType } from '@ppt/api-client';

export const FACILITY_TYPES: readonly FacilityType[] = [
  'gym',
  'laundry',
  'meeting_room',
  'party_room',
  'sauna',
  'pool',
  'playground',
  'parking',
  'storage',
  'garden',
  'bbq',
  'bike_storage',
  'other',
];
