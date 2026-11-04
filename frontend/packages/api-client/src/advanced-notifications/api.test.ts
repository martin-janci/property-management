import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../lib/fetch', () => ({ authenticatedFetchJson: vi.fn() }));
vi.mock('../granular-notifications/api', () => ({
  getEventTriggers: vi.fn(),
  updateEventTrigger: vi.fn(),
}));

import { getEventTriggers, updateEventTrigger } from '../granular-notifications/api';
import { authenticatedFetchJson } from '../lib/fetch';
import {
  getCategoryPreferences,
  getDigestPreferences,
  getQuietHours,
  updateCategoryPreference,
  updateDigestPreferences,
  updateQuietHours,
} from './api';

const fetchJson = vi.mocked(authenticatedFetchJson);
const events = vi.mocked(getEventTriggers);
const updateEvent = vi.mocked(updateEventTrigger);

const SCHEDULE_URL = '/api/v1/users/me/notification-preferences/granular/schedule';

const schedule = (overrides: Record<string, unknown> = {}) => ({
  schedule: {
    quietHoursEnabled: true,
    quietHoursStart: '22:30:00',
    quietHoursEnd: '06:00:00',
    timezone: 'Europe/Bratislava',
    weekendQuietHoursEnabled: false,
    digestEnabled: true,
    digestFrequency: 'weekly',
    digestTime: '09:15:00',
    digestDayOfWeek: 1,
    updatedAt: '2026-11-01T00:00:00Z',
    ...overrides,
  },
  isCurrentlyQuiet: false,
});

const trigger = (eventType: string, category: string, push: boolean) => ({
  eventType,
  category,
  displayName: eventType,
  isPriority: false,
  pushEnabled: push,
  emailEnabled: false,
  inAppEnabled: true,
});

beforeEach(() => {
  vi.clearAllMocks();
});

describe('advanced notifications adapter', () => {
  it('only calls the granular endpoints (never the removed Epic 40 paths)', async () => {
    fetchJson.mockResolvedValue(schedule());
    await getQuietHours();
    await getDigestPreferences();
    for (const [url] of fetchJson.mock.calls) {
      expect(url).toBe(SCHEDULE_URL);
    }
  });

  it('rolls per-event preferences up into UI categories', async () => {
    events.mockResolvedValue({
      preferences: [
        trigger('fault.created', 'fault', false),
        trigger('fault.resolved', 'fault', true),
        trigger('vote.opened', 'vote', false),
      ],
      categories: [],
    } as never);

    const { categories } = await getCategoryPreferences();
    expect(categories.map((c) => c.category)).toEqual(['faults', 'voting']);
    expect(categories[0].channels).toEqual({ push: true, email: false, in_app: true });
    expect(categories[1].channels.push).toBe(false);
  });

  it('fans a category toggle out to every event type in it', async () => {
    events.mockResolvedValue({
      preferences: [
        trigger('fault.created', 'fault', false),
        trigger('fault.resolved', 'fault', false),
        trigger('vote.opened', 'vote', false),
      ],
      categories: [],
    } as never);
    updateEvent.mockResolvedValue({} as never);

    await updateCategoryPreference('faults', { channels: { push: true } });
    expect(updateEvent).toHaveBeenCalledTimes(2);
    expect(updateEvent).toHaveBeenCalledWith('fault.created', { pushEnabled: true });
    expect(updateEvent).toHaveBeenCalledWith('fault.resolved', { pushEnabled: true });
  });

  it('maps the schedule into quiet hours and digest configs', async () => {
    fetchJson.mockResolvedValue(schedule());
    const { quietHours } = await getQuietHours();
    expect(quietHours).toMatchObject({
      enabled: true,
      startTime: '22:30',
      endTime: '06:00',
      timezone: 'Europe/Bratislava',
    });
    expect(quietHours.daysOfWeek).toContain('saturday');

    const { digest } = await getDigestPreferences();
    expect(digest).toMatchObject({
      enabled: true,
      frequency: 'weekly',
      deliveryTime: '09:15',
      deliveryDay: 'monday',
    });
  });

  it('excluding the weekend writes a degenerate weekend window', async () => {
    fetchJson.mockResolvedValueOnce(schedule()).mockResolvedValueOnce(
      schedule({
        weekendQuietHoursEnabled: true,
        weekendQuietHoursStart: '00:00:00',
        weekendQuietHoursEnd: '00:00:00',
      })
    );
    const { quietHours } = await updateQuietHours({
      daysOfWeek: ['monday', 'tuesday', 'wednesday', 'thursday', 'friday'],
    });

    const [url, init] = fetchJson.mock.calls[1];
    expect(url).toBe(SCHEDULE_URL);
    expect(init?.method).toBe('PUT');
    expect(JSON.parse(String(init?.body))).toMatchObject({
      quietHoursStart: '22:30',
      quietHoursEnd: '06:00',
      weekendQuietHoursEnabled: true,
      weekendQuietHoursStart: '00:00',
      weekendQuietHoursEnd: '00:00',
    });
    expect(quietHours.daysOfWeek).not.toContain('saturday');
  });

  it('disabling the digest sends digestEnabled=false only', async () => {
    fetchJson.mockResolvedValueOnce(schedule({ digestEnabled: false }));
    const { digest } = await updateDigestPreferences({ frequency: 'disabled' });
    const [, init] = fetchJson.mock.calls[0];
    expect(JSON.parse(String(init?.body))).toEqual({ digestEnabled: false });
    expect(digest.frequency).toBe('disabled');
  });
});
