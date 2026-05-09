/**
 * Bridge between React Native and native widget implementations.
 *
 * Epic 49 - Story 49.1: Home Screen Widgets
 *
 * This module provides the interface for communicating between the React Native
 * app and native home screen widgets (iOS WidgetKit / Android App Widgets).
 */
import { NativeModules, Platform } from 'react-native';
import type { WidgetConfig, WidgetData, WidgetDeepLink, WidgetType } from './types';
import { WidgetDataProvider } from './WidgetDataProvider';

// Native module interface (implemented in native code)
interface NativeWidgetModule {
  refreshWidget(widgetId: string, data: string): Promise<void>;
  refreshAllWidgets(): Promise<void>;
  registerForUpdates(widgetId: string, intervalMinutes: number): Promise<void>;
  unregisterFromUpdates(widgetId: string): Promise<void>;
  getInstalledWidgets(): Promise<string[]>;
  isWidgetSupported(): Promise<boolean>;
}

// Get native module or provide mock for development
const NativeWidget: NativeWidgetModule | null = NativeModules.PPTWidget ?? null;

/**
 * Bridge for widget communication between React Native and native platforms.
 */
export class WidgetBridge {
  private dataProvider: WidgetDataProvider;
  private isSupported: boolean | null = null;

  constructor(apiBaseUrl: string) {
    this.dataProvider = new WidgetDataProvider(apiBaseUrl);
  }

  /**
   * Initialize the widget bridge with auth token.
   */
  setAuthToken(token: string | null): void {
    this.dataProvider.setAuthToken(token);
  }

  /**
   * Check if widgets are supported on this platform.
   */
  async isWidgetSupported(): Promise<boolean> {
    if (this.isSupported !== null) {
      return this.isSupported;
    }

    // Widgets require iOS 14+ or Android
    if (Platform.OS === 'ios') {
      // Platform.Version on iOS is a string like "14.0" or "15.2.1"
      const versionString = String(Platform.Version);
      const majorVersion = Number.parseInt(versionString.split('.')[0], 10);
      this.isSupported = !Number.isNaN(majorVersion) && majorVersion >= 14;
    } else if (Platform.OS === 'android') {
      this.isSupported = true;
    } else {
      this.isSupported = false;
    }

    // Also check native module availability
    if (this.isSupported && NativeWidget) {
      try {
        this.isSupported = await NativeWidget.isWidgetSupported();
      } catch {
        this.isSupported = false;
      }
    }

    return this.isSupported;
  }

  /**
   * Configure a new widget.
   */
  async configureWidget(config: WidgetConfig): Promise<void> {
    // Save configuration
    await this.dataProvider.saveWidgetConfig(config);

    // Fetch and cache initial data
    const data = await this.dataProvider.fetchWidgetData(config);
    await this.dataProvider.cacheWidgetData(config.id, data);

    // Register for background updates
    if (NativeWidget) {
      await NativeWidget.registerForUpdates(config.id, config.refreshInterval);
      await NativeWidget.refreshWidget(config.id, JSON.stringify(data));
    }
  }

  /**
   * Remove a widget configuration.
   */
  async removeWidget(widgetId: string): Promise<void> {
    await this.dataProvider.removeWidgetConfig(widgetId);

    if (NativeWidget) {
      await NativeWidget.unregisterFromUpdates(widgetId);
    }
  }

  /**
   * Update a specific widget with fresh data.
   */
  async updateWidget(widgetId: string): Promise<WidgetData | null> {
    const configs = await this.dataProvider.getAllWidgetConfigs();
    const config = configs.find((c) => c.id === widgetId);

    if (!config) {
      return null;
    }

    try {
      const data = await this.dataProvider.fetchWidgetData(config);
      await this.dataProvider.cacheWidgetData(widgetId, data);

      if (NativeWidget) {
        await NativeWidget.refreshWidget(widgetId, JSON.stringify(data));
      }

      return data;
    } catch {
      // Return cached data on failure
      return this.dataProvider.getCachedWidgetData(widgetId);
    }
  }

  /**
   * Update all configured widgets.
   */
  async updateAllWidgets(): Promise<void> {
    const results = await this.dataProvider.updateAllWidgets();

    if (NativeWidget) {
      for (const [widgetId, data] of results) {
        await NativeWidget.refreshWidget(widgetId, JSON.stringify(data));
      }
    }
  }

  /**
   * Get list of installed widget IDs.
   */
  async getInstalledWidgets(): Promise<string[]> {
    if (NativeWidget) {
      return NativeWidget.getInstalledWidgets();
    }
    return [];
  }

  /**
   * Get all widget configurations.
   */
  async getAllConfigs(): Promise<WidgetConfig[]> {
    return this.dataProvider.getAllWidgetConfigs();
  }

  /**
   * Get cached data for a widget.
   */
  async getCachedData(widgetId: string): Promise<WidgetData | null> {
    return this.dataProvider.getCachedWidgetData(widgetId);
  }

  /**
   * Handle deep link from widget tap.
   */
  parseWidgetDeepLink(url: string): WidgetDeepLink | null {
    try {
      const parsed = new URL(url);

      if (parsed.protocol !== 'ppt:') {
        return null;
      }

      const path = parsed.hostname + parsed.pathname;

      // Extract query parameters
      const queryParams: Record<string, string> = {};
      parsed.searchParams.forEach((value, key) => {
        queryParams[key] = value;
      });

      // Extract ID from path if present (e.g., "faults/123" -> "123")
      const pathSegments = path.split('/');
      const baseRoute = pathSegments[0];
      const entityId = pathSegments[1];

      switch (baseRoute) {
        case 'dashboard':
          return { screen: 'Dashboard', query: queryParams };
        case 'faults':
          return { screen: 'Faults', faultId: entityId, query: queryParams };
        case 'fault':
          if (pathSegments[1] === 'report') {
            return { screen: 'ReportFault', query: queryParams };
          }
          return null;
        case 'announcements':
          return { screen: 'Announcements', announcementId: entityId, query: queryParams };
        case 'voting':
          return { screen: 'Voting', voteId: entityId, query: queryParams };
        case 'documents':
          return { screen: 'Documents', query: queryParams };
        default:
          return null;
      }
    } catch {
      // Invalid URL format
      return null;
    }
  }

  /**
   * Create deep link URL for a screen.
   */
  createDeepLink(target: WidgetDeepLink): string {
    switch (target.screen) {
      case 'Dashboard':
        return 'ppt://dashboard';
      case 'Faults':
        return target.faultId ? `ppt://faults/${target.faultId}` : 'ppt://faults';
      case 'ReportFault':
        return 'ppt://fault/report';
      case 'Announcements':
        return target.announcementId
          ? `ppt://announcements/${target.announcementId}`
          : 'ppt://announcements';
      case 'Voting':
        return target.voteId ? `ppt://voting/${target.voteId}` : 'ppt://voting';
      case 'Documents':
        return 'ppt://documents';
    }
  }

  /**
   * Get available widget types with metadata.
   */
  getAvailableWidgetTypes(): Array<{
    type: WidgetType;
    name: string;
    description: string;
    sizes: Array<'small' | 'medium' | 'large'>;
  }> {
    return [
      {
        type: 'notifications_count',
        name: 'Notifications',
        description: 'Shows unread notification count',
        sizes: ['small'],
      },
      {
        type: 'latest_announcement',
        name: 'Latest Announcement',
        description: 'Shows the most recent announcement',
        sizes: ['medium', 'large'],
      },
      {
        type: 'pending_votes',
        name: 'Pending Votes',
        description: 'Shows votes awaiting your input',
        sizes: ['small', 'medium'],
      },
      {
        type: 'active_faults',
        name: 'Active Faults',
        description: 'Shows recently reported faults',
        sizes: ['small', 'medium', 'large'],
      },
      {
        type: 'upcoming_meetings',
        name: 'Upcoming Meetings',
        description: 'Shows scheduled building meetings',
        sizes: ['medium', 'large'],
      },
      {
        type: 'quick_actions',
        name: 'Quick Actions',
        description: 'Quick access to common actions',
        sizes: ['small', 'medium'],
      },
    ];
  }
}
