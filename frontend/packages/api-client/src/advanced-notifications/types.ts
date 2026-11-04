/**
 * Advanced Notification Types (Epic 40)
 *
 * Extends notification preferences with:
 * - Category-based granular preferences (Story 40.1)
 * - Quiet hours configuration (Story 40.2)
 * - Digest preferences (Story 40.3)
 * - Notification grouping settings (Story 40.4)
 */

import type { NotificationChannel } from '../notification-preferences/types';

// ============================================================================
// Story 40.1: Granular Category Preferences
// ============================================================================

/** Notification categories for granular control */
export type NotificationCategory =
  | 'faults'
  | 'voting'
  | 'announcements'
  | 'documents'
  | 'messages'
  | 'community'
  | 'financial'
  | 'maintenance'
  | 'system';

/** Category preference with channel-specific settings */
export interface CategoryPreference {
  category: NotificationCategory;
  channels: {
    push: boolean;
    email: boolean;
    in_app: boolean;
  };
  updatedAt: string;
}

/** Response from GET /users/me/notification-preferences/categories */
export interface CategoryPreferencesResponse {
  categories: CategoryPreference[];
}

/** Request to update category preferences */
export interface UpdateCategoryPreferenceRequest {
  channels: Partial<Record<NotificationChannel, boolean>>;
}

/** All notification categories in display order */
export const ALL_CATEGORIES: NotificationCategory[] = [
  'faults',
  'voting',
  'announcements',
  'documents',
  'messages',
  'community',
  'financial',
  'maintenance',
  'system',
];

/** Human-readable labels for categories */
export const CATEGORY_LABELS: Record<NotificationCategory, string> = {
  faults: 'Fault Reports',
  voting: 'Voting & Decisions',
  announcements: 'Announcements',
  documents: 'Documents',
  messages: 'Messages',
  community: 'Community',
  financial: 'Financial',
  maintenance: 'Maintenance',
  system: 'System Updates',
};

/** Descriptions for each category */
export const CATEGORY_DESCRIPTIONS: Record<NotificationCategory, string> = {
  faults: 'Updates on fault reports, repairs, and resolutions',
  voting: 'New votes, results, and voting reminders',
  announcements: 'Building announcements and news',
  documents: 'New documents, signatures, and updates',
  messages: 'Direct messages from neighbors and managers',
  community: 'Community posts, events, and marketplace activity',
  financial: 'Payment reminders, invoices, and financial updates',
  maintenance: 'Scheduled maintenance and work orders',
  system: 'App updates, security alerts, and system notifications',
};

// ============================================================================
// Story 40.2: Quiet Hours Configuration
// ============================================================================

/** Day of week for quiet hours schedule */
export type DayOfWeek =
  | 'monday'
  | 'tuesday'
  | 'wednesday'
  | 'thursday'
  | 'friday'
  | 'saturday'
  | 'sunday';

/** Quiet hours configuration */
export interface QuietHoursConfig {
  enabled: boolean;
  startTime: string; // HH:mm format (e.g., "22:00")
  endTime: string; // HH:mm format (e.g., "07:00")
  timezone: string; // IANA timezone (e.g., "Europe/Bratislava")
  daysOfWeek: DayOfWeek[]; // Which days quiet hours apply
  allowEmergency: boolean; // Allow emergency notifications during quiet hours
  updatedAt: string;
}

/** Response from GET /users/me/notification-preferences/quiet-hours */
export interface QuietHoursResponse {
  quietHours: QuietHoursConfig;
}

/** Request to update quiet hours */
export interface UpdateQuietHoursRequest {
  enabled?: boolean;
  startTime?: string;
  endTime?: string;
  timezone?: string;
  daysOfWeek?: DayOfWeek[];
  allowEmergency?: boolean;
}

/** All days of the week */
export const ALL_DAYS: DayOfWeek[] = [
  'monday',
  'tuesday',
  'wednesday',
  'thursday',
  'friday',
  'saturday',
  'sunday',
];

/** Human-readable day labels */
export const DAY_LABELS: Record<DayOfWeek, string> = {
  monday: 'Mon',
  tuesday: 'Tue',
  wednesday: 'Wed',
  thursday: 'Thu',
  friday: 'Fri',
  saturday: 'Sat',
  sunday: 'Sun',
};

/** Full day names */
export const DAY_FULL_LABELS: Record<DayOfWeek, string> = {
  monday: 'Monday',
  tuesday: 'Tuesday',
  wednesday: 'Wednesday',
  thursday: 'Thursday',
  friday: 'Friday',
  saturday: 'Saturday',
  sunday: 'Sunday',
};

// ============================================================================
// Story 40.3: Digest Preferences
// ============================================================================

/** Digest frequency options */
export type DigestFrequency = 'hourly' | 'daily' | 'weekly' | 'disabled';

/**
 * Digest configuration
 *
 * Note: The `enabled` field and `frequency` field work together:
 * - When `enabled` is false, digests are disabled (frequency should be 'disabled')
 * - When `enabled` is true, frequency should be 'hourly', 'daily', or 'weekly'
 *
 * The UI ensures these stay in sync by setting frequency='disabled' when
 * toggling enabled to false.
 */
export interface DigestConfig {
  enabled: boolean;
  frequency: DigestFrequency;
  deliveryTime: string; // HH:mm format for daily/weekly
  deliveryDay?: DayOfWeek; // For weekly digests
  timezone: string;
  includeCategories: NotificationCategory[]; // Empty array means include all categories
  updatedAt: string;
}

/** Response from GET /users/me/notification-preferences/digest */
export interface DigestResponse {
  digest: DigestConfig;
}

/** Request to update digest preferences */
export interface UpdateDigestRequest {
  enabled?: boolean;
  frequency?: DigestFrequency;
  deliveryTime?: string;
  deliveryDay?: DayOfWeek;
  timezone?: string;
  includeCategories?: NotificationCategory[];
}

/**
 * Digest options (including disabled) offered in the UI, in display order.
 * The backend schedule supports daily and weekly digests only.
 */
export const DIGEST_OPTIONS: DigestFrequency[] = ['disabled', 'daily', 'weekly'];

/**
 * @deprecated Use DIGEST_OPTIONS instead. This constant includes 'disabled',
 * which is not a frequency but an option to turn off digests.
 */
export const ALL_FREQUENCIES: DigestFrequency[] = DIGEST_OPTIONS;

/** Human-readable frequency labels */
export const FREQUENCY_LABELS: Record<DigestFrequency, string> = {
  hourly: 'Every Hour',
  daily: 'Daily',
  weekly: 'Weekly',
  disabled: 'Disabled',
};

/** Frequency descriptions */
export const FREQUENCY_DESCRIPTIONS: Record<DigestFrequency, string> = {
  hourly: 'Receive a summary every hour with all new notifications',
  daily: 'Receive a daily summary at your preferred time',
  weekly: 'Receive a weekly summary on your chosen day',
  disabled: 'Receive notifications immediately instead of in digests',
};

// ============================================================================
// Story 40.4: Smart Notification Grouping
// ============================================================================

/** Grouping preferences */
export interface GroupingConfig {
  enabled: boolean;
  groupByCategory: boolean;
  groupBySource: boolean; // e.g., same building, same fault
  maxGroupSize: number; // Maximum notifications before showing "and X more"
  autoExpandThreshold: number; // Auto-expand if fewer than this many
  updatedAt: string;
}

/** Response from GET /users/me/notification-preferences/grouping */
export interface GroupingResponse {
  grouping: GroupingConfig;
}

/** Request to update grouping preferences */
export interface UpdateGroupingRequest {
  enabled?: boolean;
  groupByCategory?: boolean;
  groupBySource?: boolean;
  maxGroupSize?: number;
  autoExpandThreshold?: number;
}

// ============================================================================
// Combined Response Types
// ============================================================================

/** Full advanced notification preferences */
export interface AdvancedNotificationPreferences {
  categories: CategoryPreference[];
  quietHours: QuietHoursConfig;
  digest: DigestConfig;
  grouping: GroupingConfig;
}

/** Response from GET /users/me/notification-preferences/advanced */
export interface AdvancedPreferencesResponse {
  preferences: AdvancedNotificationPreferences;
}
