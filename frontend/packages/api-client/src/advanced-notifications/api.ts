/**
 * Advanced Notifications API (Epic 40)
 *
 * The backend never shipped dedicated `/categories`, `/quiet-hours`, `/digest`,
 * `/grouping` or `/advanced` endpoints under `/users/me/notification-preferences`
 * (a GET to them fell through to `PATCH /{channel}` and returned 405). The data
 * these screens edit already lives in the granular notification API (Epic 8B,
 * `/users/me/notification-preferences/granular`):
 *
 * - categories  -> `GET /granular/events` rolled up per category; a category
 *                  channel toggle fans out to `PUT /granular/events/{eventType}`
 *                  for every event type in that category.
 * - quiet hours -> `GET|PUT /granular/schedule` (`quietHours*` + weekend window).
 * - digest      -> `GET|PUT /granular/schedule` (`digest*`).
 * - grouping    -> purely a client-side display preference; persisted in
 *                  browser storage, no network call.
 *
 * This module adapts those payloads to the Epic 40 shapes the UI consumes.
 */

import { getEventTriggers, updateEventTrigger } from '../granular-notifications/api';
import type {
  EventTriggerPreference,
  EventTriggersResponse,
  NotificationEventCategory,
  UpdateTriggerRequest,
} from '../granular-notifications/types';
import { authenticatedFetchJson } from '../lib/fetch';
import type {
  AdvancedPreferencesResponse,
  CategoryPreference,
  CategoryPreferencesResponse,
  DayOfWeek,
  DigestConfig,
  DigestResponse,
  GroupingConfig,
  GroupingResponse,
  NotificationCategory,
  QuietHoursConfig,
  QuietHoursResponse,
  UpdateCategoryPreferenceRequest,
  UpdateDigestRequest,
  UpdateGroupingRequest,
  UpdateQuietHoursRequest,
} from './types';

const SCHEDULE_PATH = '/api/v1/users/me/notification-preferences/granular/schedule';

// ============================================================================
// Category mapping (backend event category <-> UI category)
// ============================================================================

const BACKEND_TO_UI_CATEGORY: Record<NotificationEventCategory, NotificationCategory> = {
  fault: 'faults',
  vote: 'voting',
  announcement: 'announcements',
  document: 'documents',
  message: 'messages',
  finance: 'financial',
  facility: 'maintenance',
  critical: 'system',
};

/** UI categories in display order that have a backend counterpart. */
const UI_CATEGORY_ORDER: NotificationCategory[] = [
  'faults',
  'voting',
  'announcements',
  'documents',
  'messages',
  'financial',
  'maintenance',
  'system',
];

function rollUpCategories(response: EventTriggersResponse): CategoryPreferencesResponse {
  const byCategory = new Map<NotificationCategory, EventTriggerPreference[]>();
  for (const pref of response.preferences) {
    const uiCategory = BACKEND_TO_UI_CATEGORY[pref.category];
    if (!uiCategory) continue;
    const list = byCategory.get(uiCategory) ?? [];
    list.push(pref);
    byCategory.set(uiCategory, list);
  }

  const categories: CategoryPreference[] = [];
  for (const category of UI_CATEGORY_ORDER) {
    const prefs = byCategory.get(category);
    if (!prefs || prefs.length === 0) continue;
    const updatedAt = prefs
      .map((p) => p.updatedAt)
      .filter((v): v is string => Boolean(v))
      .sort()
      .pop();
    categories.push({
      category,
      // A channel counts as "on" for the category when any event in it uses it.
      channels: {
        push: prefs.some((p) => p.pushEnabled),
        email: prefs.some((p) => p.emailEnabled),
        in_app: prefs.some((p) => p.inAppEnabled),
      },
      updatedAt: updatedAt ?? '',
    });
  }
  return { categories };
}

// ============================================================================
// Story 40.1: Category Preferences API
// ============================================================================

/**
 * Fetch category-based notification preferences (rolled up from per-event prefs).
 */
export async function getCategoryPreferences(): Promise<CategoryPreferencesResponse> {
  return rollUpCategories(await getEventTriggers());
}

/**
 * Update preferences for a specific category by applying the channel change to
 * every event type in that category.
 */
export async function updateCategoryPreference(
  category: NotificationCategory,
  request: UpdateCategoryPreferenceRequest
): Promise<CategoryPreferencesResponse> {
  const update: UpdateTriggerRequest = {};
  if (request.channels.push !== undefined) update.pushEnabled = request.channels.push;
  if (request.channels.email !== undefined) update.emailEnabled = request.channels.email;
  if (request.channels.in_app !== undefined) update.inAppEnabled = request.channels.in_app;

  const current = await getEventTriggers();
  const targets = current.preferences.filter(
    (p) => BACKEND_TO_UI_CATEGORY[p.category] === category
  );
  if (Object.keys(update).length > 0) {
    await Promise.all(targets.map((p) => updateEventTrigger(p.eventType, update)));
  }
  return getCategoryPreferences();
}

// ============================================================================
// Schedule (quiet hours + digest) mapping
// ============================================================================

/** Mirrors backend `NotificationSchedule` (camelCase on the wire). */
interface BackendSchedule {
  quietHoursEnabled: boolean;
  quietHoursStart?: string;
  quietHoursEnd?: string;
  timezone: string;
  weekendQuietHoursEnabled: boolean;
  weekendQuietHoursStart?: string;
  weekendQuietHoursEnd?: string;
  digestEnabled: boolean;
  digestFrequency?: string;
  digestTime?: string;
  /** 0 = Sunday … 6 = Saturday. */
  digestDayOfWeek?: number | null;
  updatedAt: string;
}

interface BackendScheduleResponse {
  schedule: BackendSchedule;
  isCurrentlyQuiet: boolean;
}

interface BackendScheduleUpdate {
  quietHoursEnabled?: boolean;
  quietHoursStart?: string;
  quietHoursEnd?: string;
  timezone?: string;
  weekendQuietHoursEnabled?: boolean;
  weekendQuietHoursStart?: string;
  weekendQuietHoursEnd?: string;
  digestEnabled?: boolean;
  digestFrequency?: 'daily' | 'weekly';
  digestTime?: string;
  digestDayOfWeek?: number;
}

const WEEKDAYS: DayOfWeek[] = ['monday', 'tuesday', 'wednesday', 'thursday', 'friday'];
const WEEKEND: DayOfWeek[] = ['saturday', 'sunday'];
/** Backend day index (0 = Sunday). */
const DAY_INDEX: DayOfWeek[] = [
  'sunday',
  'monday',
  'tuesday',
  'wednesday',
  'thursday',
  'friday',
  'saturday',
];

const DEFAULT_QUIET_START = '22:00';
const DEFAULT_QUIET_END = '07:00';
const DEFAULT_DIGEST_TIME = '08:00';
/** A weekend window with start == end is evaluated as "never quiet" by the backend. */
const WEEKEND_OFF_TIME = '00:00';

/** Backend times serialise as `HH:MM:SS`; the UI uses `HH:MM`. */
function hhmm(value: string | undefined | null, fallback: string): string {
  return value ? value.slice(0, 5) : fallback;
}

function weekendExcluded(s: BackendSchedule): boolean {
  return (
    s.weekendQuietHoursEnabled &&
    hhmm(s.weekendQuietHoursStart, '') === hhmm(s.weekendQuietHoursEnd, '')
  );
}

function toQuietHours(s: BackendSchedule): QuietHoursConfig {
  return {
    enabled: s.quietHoursEnabled,
    startTime: hhmm(s.quietHoursStart, DEFAULT_QUIET_START),
    endTime: hhmm(s.quietHoursEnd, DEFAULT_QUIET_END),
    timezone: s.timezone || 'UTC',
    // Backend models one daily window plus an optional weekend override, so
    // weekdays always share the window and the weekend is either in or out.
    daysOfWeek: weekendExcluded(s) ? [...WEEKDAYS] : [...WEEKDAYS, ...WEEKEND],
    // The backend holds every push during quiet hours; there is no bypass.
    allowEmergency: false,
    updatedAt: s.updatedAt,
  };
}

function toDigest(s: BackendSchedule): DigestConfig {
  const frequency = s.digestFrequency === 'weekly' ? 'weekly' : 'daily';
  const day = typeof s.digestDayOfWeek === 'number' ? DAY_INDEX[s.digestDayOfWeek] : undefined;
  return {
    enabled: s.digestEnabled,
    frequency: s.digestEnabled ? frequency : 'disabled',
    deliveryTime: hhmm(s.digestTime, DEFAULT_DIGEST_TIME),
    deliveryDay: day,
    timezone: s.timezone || 'UTC',
    includeCategories: [],
    updatedAt: s.updatedAt,
  };
}

async function getSchedule(): Promise<BackendSchedule> {
  const res = await authenticatedFetchJson<BackendScheduleResponse>(SCHEDULE_PATH);
  return res.schedule;
}

async function putSchedule(update: BackendScheduleUpdate): Promise<BackendSchedule> {
  const res = await authenticatedFetchJson<BackendScheduleResponse>(SCHEDULE_PATH, {
    method: 'PUT',
    body: JSON.stringify(update),
  });
  return res.schedule;
}

// ============================================================================
// Story 40.2: Quiet Hours API
// ============================================================================

/**
 * Fetch quiet hours configuration.
 */
export async function getQuietHours(): Promise<QuietHoursResponse> {
  return { quietHours: toQuietHours(await getSchedule()) };
}

/**
 * Update quiet hours configuration.
 */
export async function updateQuietHours(
  request: UpdateQuietHoursRequest
): Promise<QuietHoursResponse> {
  const current = toQuietHours(await getSchedule());
  const update: BackendScheduleUpdate = {
    // Always send a full window so enabling never leaves start/end NULL
    // (a NULL window is treated as "no quiet hours" by the backend).
    quietHoursStart: request.startTime ?? current.startTime,
    quietHoursEnd: request.endTime ?? current.endTime,
  };

  if (request.enabled !== undefined) update.quietHoursEnabled = request.enabled;
  if (request.timezone !== undefined) update.timezone = request.timezone;

  if (request.daysOfWeek !== undefined) {
    const includeWeekend = request.daysOfWeek.some((d) => WEEKEND.includes(d));
    if (includeWeekend) {
      update.weekendQuietHoursEnabled = false;
    } else {
      update.weekendQuietHoursEnabled = true;
      update.weekendQuietHoursStart = WEEKEND_OFF_TIME;
      update.weekendQuietHoursEnd = WEEKEND_OFF_TIME;
    }
  }

  return { quietHours: toQuietHours(await putSchedule(update)) };
}

// ============================================================================
// Story 40.3: Digest Preferences API
// ============================================================================

/**
 * Fetch digest configuration.
 */
export async function getDigestPreferences(): Promise<DigestResponse> {
  return { digest: toDigest(await getSchedule()) };
}

/**
 * Update digest preferences.
 */
export async function updateDigestPreferences(
  request: UpdateDigestRequest
): Promise<DigestResponse> {
  const update: BackendScheduleUpdate = {};

  if (request.enabled !== undefined) update.digestEnabled = request.enabled;
  if (request.frequency !== undefined) {
    if (request.frequency === 'disabled') {
      update.digestEnabled = false;
    } else {
      update.digestEnabled = true;
      // Backend supports daily and weekly digests only.
      update.digestFrequency = request.frequency === 'weekly' ? 'weekly' : 'daily';
    }
  }
  if (request.deliveryTime !== undefined) update.digestTime = request.deliveryTime;
  if (request.deliveryDay !== undefined) {
    update.digestDayOfWeek = DAY_INDEX.indexOf(request.deliveryDay);
  }
  if (request.timezone !== undefined) update.timezone = request.timezone;

  if (update.digestEnabled) {
    // Make sure an enabled digest always has a frequency and delivery time.
    const current = await getSchedule();
    if (update.digestFrequency === undefined) {
      update.digestFrequency = current.digestFrequency === 'weekly' ? 'weekly' : 'daily';
    }
    if (update.digestTime === undefined) {
      update.digestTime = hhmm(current.digestTime, DEFAULT_DIGEST_TIME);
    }
  }

  return { digest: toDigest(await putSchedule(update)) };
}

// ============================================================================
// Story 40.4: Grouping Preferences (client-side only)
// ============================================================================

const GROUPING_STORAGE_KEY = 'ppt.notification-grouping';

const DEFAULT_GROUPING: GroupingConfig = {
  enabled: true,
  groupByCategory: true,
  groupBySource: true,
  maxGroupSize: 5,
  autoExpandThreshold: 2,
  updatedAt: '',
};

function readGrouping(): GroupingConfig {
  try {
    const raw = globalThis.localStorage?.getItem(GROUPING_STORAGE_KEY);
    if (raw) return { ...DEFAULT_GROUPING, ...(JSON.parse(raw) as Partial<GroupingConfig>) };
  } catch {
    // Storage unavailable or corrupted: fall back to defaults.
  }
  return { ...DEFAULT_GROUPING };
}

/**
 * Fetch notification grouping configuration (stored per browser).
 */
export async function getGroupingPreferences(): Promise<GroupingResponse> {
  return { grouping: readGrouping() };
}

/**
 * Update notification grouping preferences (stored per browser).
 */
export async function updateGroupingPreferences(
  request: UpdateGroupingRequest
): Promise<GroupingResponse> {
  const next: GroupingConfig = {
    ...readGrouping(),
    ...request,
    updatedAt: new Date().toISOString(),
  };
  try {
    globalThis.localStorage?.setItem(GROUPING_STORAGE_KEY, JSON.stringify(next));
  } catch {
    // Ignore storage failures; the in-memory value is still returned.
  }
  return { grouping: next };
}

// ============================================================================
// Combined API
// ============================================================================

/**
 * Fetch all advanced notification preferences at once.
 */
export async function getAdvancedPreferences(): Promise<AdvancedPreferencesResponse> {
  const [events, schedule] = await Promise.all([getEventTriggers(), getSchedule()]);
  return {
    preferences: {
      categories: rollUpCategories(events).categories,
      quietHours: toQuietHours(schedule),
      digest: toDigest(schedule),
      grouping: readGrouping(),
    },
  };
}
