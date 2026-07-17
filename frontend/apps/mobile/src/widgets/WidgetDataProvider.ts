/**
 * Widget data provider for fetching and updating widget content.
 *
 * Epic 49 - Story 49.1: Home Screen Widgets
 */
import AsyncStorage from '@react-native-async-storage/async-storage';

import { WIDGET_CONFIG_KEY, WIDGET_DATA_KEY } from '../services/localCacheKeys';
import type {
  ActiveFaultsWidgetData,
  AnnouncementWidgetData,
  NotificationsWidgetData,
  PendingVotesWidgetData,
  QuickActionsWidgetData,
  UpcomingMeetingsWidgetData,
  WidgetConfig,
  WidgetData,
  WidgetType,
} from './types';

/**
 * Provides data for home screen widgets.
 */
export class WidgetDataProvider {
  private apiBaseUrl: string;
  private authToken: string | null = null;

  constructor(apiBaseUrl: string) {
    this.apiBaseUrl = apiBaseUrl;
  }

  /**
   * Set authentication token for API requests.
   */
  setAuthToken(token: string | null): void {
    this.authToken = token;
  }

  /**
   * Fetch widget data based on widget type.
   */
  async fetchWidgetData(config: WidgetConfig): Promise<WidgetData> {
    switch (config.type) {
      case 'notifications_count':
        return this.fetchNotificationsData(config.buildingId);
      case 'latest_announcement':
        return this.fetchAnnouncementData(config.buildingId);
      case 'pending_votes':
        return this.fetchPendingVotesData(config.buildingId);
      case 'active_faults':
        return this.fetchActiveFaultsData(config.buildingId);
      case 'upcoming_meetings':
        return this.fetchUpcomingMeetingsData(config.buildingId);
      case 'quick_actions':
        return this.fetchQuickActionsData();
    }
  }

  /**
   * Save widget configuration.
   */
  async saveWidgetConfig(config: WidgetConfig): Promise<void> {
    const configs = await this.getAllWidgetConfigs();
    const existingIndex = configs.findIndex((c) => c.id === config.id);

    if (existingIndex >= 0) {
      configs[existingIndex] = config;
    } else {
      configs.push(config);
    }

    await AsyncStorage.setItem(WIDGET_CONFIG_KEY, JSON.stringify(configs));
  }

  /**
   * Remove widget configuration.
   */
  async removeWidgetConfig(widgetId: string): Promise<void> {
    const configs = await this.getAllWidgetConfigs();
    const filtered = configs.filter((c) => c.id !== widgetId);
    await AsyncStorage.setItem(WIDGET_CONFIG_KEY, JSON.stringify(filtered));

    // Also remove cached data
    const allData = await this.getAllCachedData();
    delete allData[widgetId];
    await AsyncStorage.setItem(WIDGET_DATA_KEY, JSON.stringify(allData));
  }

  /**
   * Get all widget configurations.
   */
  async getAllWidgetConfigs(): Promise<WidgetConfig[]> {
    const stored = await AsyncStorage.getItem(WIDGET_CONFIG_KEY);
    return stored ? JSON.parse(stored) : [];
  }

  /**
   * Cache widget data for offline access.
   */
  async cacheWidgetData(widgetId: string, data: WidgetData): Promise<void> {
    const allData = await this.getAllCachedData();
    allData[widgetId] = {
      data,
      cachedAt: new Date().toISOString(),
    };
    await AsyncStorage.setItem(WIDGET_DATA_KEY, JSON.stringify(allData));
  }

  /**
   * Get cached widget data.
   */
  async getCachedWidgetData(widgetId: string, maxAgeMinutes?: number): Promise<WidgetData | null> {
    const allData = await this.getAllCachedData();
    const cached = allData[widgetId];

    if (!cached) {
      return null;
    }

    // Check cache age if maxAgeMinutes is specified
    if (maxAgeMinutes !== undefined) {
      const cachedTime = new Date(cached.cachedAt).getTime();
      const now = Date.now();
      const ageMinutes = (now - cachedTime) / (60 * 1000);

      if (ageMinutes > maxAgeMinutes) {
        return null; // Cache expired
      }
    }

    return cached.data;
  }

  /**
   * Update all widgets in the background.
   * Uses Promise.allSettled to fetch all widgets concurrently without race conditions.
   */
  async updateAllWidgets(): Promise<Map<string, WidgetData>> {
    const configs = await this.getAllWidgetConfigs();
    const results = new Map<string, WidgetData>();

    // Fetch all widgets concurrently
    const fetchPromises = configs.map(async (config) => {
      try {
        const data = await this.fetchWidgetData(config);
        return { config, data, success: true as const };
      } catch (error) {
        console.warn(`Failed to fetch widget ${config.id}:`, error);
        return { config, error, success: false as const };
      }
    });

    const settledResults = await Promise.allSettled(fetchPromises);

    // Process results sequentially to avoid AsyncStorage race conditions
    for (const settled of settledResults) {
      if (settled.status === 'fulfilled') {
        const result = settled.value;
        if (result.success) {
          await this.cacheWidgetData(result.config.id, result.data);
          results.set(result.config.id, result.data);
        } else {
          // Use cached data on failure
          const cached = await this.getCachedWidgetData(result.config.id);
          if (cached) {
            results.set(result.config.id, cached);
          }
        }
      }
    }

    return results;
  }

  /**
   * Get default widget configurations.
   */
  getDefaultWidgetConfigs(): WidgetConfig[] {
    return [
      {
        id: 'widget-notifications-default',
        type: 'notifications_count' as WidgetType,
        size: 'small',
        refreshInterval: 15,
      },
      {
        id: 'widget-announcement-default',
        type: 'latest_announcement' as WidgetType,
        size: 'medium',
        refreshInterval: 30,
      },
      {
        id: 'widget-quick-actions-default',
        type: 'quick_actions' as WidgetType,
        size: 'small',
        refreshInterval: 60,
      },
    ];
  }

  // Private methods for fetching specific widget data

  private async fetchNotificationsData(buildingId?: string): Promise<NotificationsWidgetData> {
    // In production, fetch from API
    // For now, return mock data structure
    const params = buildingId ? `?building_id=${buildingId}` : '';
    const response = await this.apiRequest<{
      unread_count?: number;
      categories?: {
        announcements?: number;
        faults?: number;
        votes?: number;
        documents?: number;
        messages?: number;
      };
    }>(`/api/v1/notifications/summary${params}`);

    return {
      type: 'notifications_count',
      unreadCount: response.unread_count ?? 0,
      categories: {
        announcements: response.categories?.announcements ?? 0,
        faults: response.categories?.faults ?? 0,
        votes: response.categories?.votes ?? 0,
        documents: response.categories?.documents ?? 0,
        messages: response.categories?.messages ?? 0,
      },
    };
  }

  private async fetchAnnouncementData(buildingId?: string): Promise<AnnouncementWidgetData> {
    const params = buildingId ? `?building_id=${buildingId}` : '';
    const response = await this.apiRequest<{
      id?: string;
      title?: string;
      preview?: string;
      created_at?: string;
      is_urgent?: boolean;
      building_name?: string;
    }>(`/api/v1/announcements/latest${params}`);

    return {
      type: 'latest_announcement',
      id: response.id ?? '',
      title: response.title ?? 'No announcements',
      preview: response.preview ?? '',
      createdAt: response.created_at ?? new Date().toISOString(),
      isUrgent: response.is_urgent ?? false,
      buildingName: response.building_name ?? '',
    };
  }

  private async fetchPendingVotesData(buildingId?: string): Promise<PendingVotesWidgetData> {
    const params = buildingId ? `?building_id=${buildingId}` : '';
    const response = await this.apiRequest<{
      pending_count?: number;
      votes?: Array<{ id: string; title: string; end_date: string; building_name: string }>;
    }>(`/api/v1/votes/pending${params}`);

    return {
      type: 'pending_votes',
      pendingCount: response.pending_count ?? 0,
      votes:
        response.votes?.map((v) => ({
          id: v.id,
          title: v.title,
          endDate: v.end_date,
          buildingName: v.building_name,
        })) ?? [],
    };
  }

  private async fetchActiveFaultsData(buildingId?: string): Promise<ActiveFaultsWidgetData> {
    const params = buildingId ? `?building_id=${buildingId}` : '';
    const response = await this.apiRequest<{
      active_count?: number;
      faults?: Array<{ id: string; title: string; status: string; reported_at: string }>;
    }>(`/api/v1/faults/active${params}`);

    return {
      type: 'active_faults',
      activeCount: response.active_count ?? 0,
      recentFaults:
        response.faults?.map((f) => ({
          id: f.id,
          title: f.title,
          status: f.status as 'open' | 'in_progress' | 'awaiting_parts',
          reportedAt: f.reported_at,
        })) ?? [],
    };
  }

  private async fetchUpcomingMeetingsData(
    buildingId?: string
  ): Promise<UpcomingMeetingsWidgetData> {
    const params = buildingId ? `?building_id=${buildingId}` : '';
    const response = await this.apiRequest<{
      upcoming_count?: number;
      meetings?: Array<{ id: string; title: string; date: string; location: string }>;
    }>(`/api/v1/meetings/upcoming${params}`);

    return {
      type: 'upcoming_meetings',
      upcomingCount: response.upcoming_count ?? 0,
      meetings:
        response.meetings?.map((m) => ({
          id: m.id,
          title: m.title,
          date: m.date,
          location: m.location,
        })) ?? [],
    };
  }

  private async fetchQuickActionsData(): Promise<QuickActionsWidgetData> {
    return {
      type: 'quick_actions',
      actions: [
        {
          id: 'report-fault',
          label: 'Report Fault',
          icon: '🔧',
          deepLink: 'ppt://fault/report',
        },
        {
          id: 'view-announcements',
          label: 'Announcements',
          icon: '📢',
          deepLink: 'ppt://announcements',
        },
        {
          id: 'vote',
          label: 'Vote',
          icon: '🗳️',
          deepLink: 'ppt://voting',
        },
        {
          id: 'documents',
          label: 'Documents',
          icon: '📄',
          deepLink: 'ppt://documents',
        },
      ],
    };
  }

  private async getAllCachedData(): Promise<
    Record<string, { data: WidgetData; cachedAt: string }>
  > {
    const stored = await AsyncStorage.getItem(WIDGET_DATA_KEY);
    return stored ? JSON.parse(stored) : {};
  }

  private async apiRequest<T>(endpoint: string): Promise<T> {
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };

    if (this.authToken) {
      headers.Authorization = `Bearer ${this.authToken}`;
    }

    try {
      const response = await fetch(`${this.apiBaseUrl}${endpoint}`, {
        method: 'GET',
        headers,
      });

      if (!response.ok) {
        throw new Error(`API request failed: ${response.status}`);
      }

      return response.json();
    } catch (error) {
      // Log error for debugging; in production, integrate with error reporting
      if (__DEV__) {
        console.error(`Widget API request failed for ${endpoint}:`, error);
      }
      // Return empty object for mock data fallback
      return {} as T;
    }
  }
}
