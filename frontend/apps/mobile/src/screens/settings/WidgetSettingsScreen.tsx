/**
 * Widget settings screen for configuring home screen widgets.
 *
 * Epic 49 - Story 49.1: Home Screen Widgets
 */
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  Alert,
  Platform,
  Pressable,
  ScrollView,
  StyleSheet,
  Switch,
  Text,
  View,
} from 'react-native';

import { getApiBaseUrl } from '../../config/api';
import { WidgetBridge } from '../../widgets';
import type { WidgetConfig, WidgetType } from '../../widgets/types';
import { colors } from '../shared/screenStyles';

interface WidgetSettingsScreenProps {
  onNavigate: (screen: string) => void;
}

export function WidgetSettingsScreen({ onNavigate }: WidgetSettingsScreenProps) {
  const { t } = useTranslation();
  const [isSupported, setIsSupported] = useState<boolean | null>(null);
  const [configs, setConfigs] = useState<WidgetConfig[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  const bridge = useMemo(() => new WidgetBridge(getApiBaseUrl()), []);
  const availableTypes = useMemo(() => bridge.getAvailableWidgetTypes(), [bridge]);

  useEffect(() => {
    async function checkSupport() {
      const supported = await bridge.isWidgetSupported();
      setIsSupported(supported);
      if (supported) {
        const savedConfigs = await bridge.getAllConfigs();
        setConfigs(savedConfigs);
      }
      setIsLoading(false);
    }
    checkSupport();
  }, [bridge]);

  const handleToggleWidget = useCallback(
    async (type: WidgetType, enabled: boolean) => {
      const existingConfig = configs.find((c) => c.type === type);

      if (enabled && !existingConfig) {
        const widgetMeta = availableTypes.find((t) => t.type === type);
        const newConfig: WidgetConfig = {
          id: `widget-${type}-${Date.now()}`,
          type,
          size: widgetMeta?.sizes[0] ?? 'small',
          refreshInterval: 15,
        };

        await bridge.configureWidget(newConfig);
        setConfigs((prev) => [...prev, newConfig]);

        Alert.alert('Widget Added', `${widgetMeta?.name ?? type} widget has been configured.`);
      } else if (!enabled && existingConfig) {
        await bridge.removeWidget(existingConfig.id);
        setConfigs((prev) => prev.filter((c) => c.id !== existingConfig.id));
      }
    },
    [configs, availableTypes, bridge]
  );

  const handleRefreshAll = useCallback(async () => {
    setIsLoading(true);
    await bridge.updateAllWidgets();
    setIsLoading(false);
    Alert.alert('Widgets Updated', 'All widgets have been refreshed.');
  }, [bridge]);

  if (isLoading) {
    return (
      <View style={styles.container}>
        <View style={styles.header}>
          <Pressable onPress={() => onNavigate('Dashboard')} style={styles.backButton}>
            <Text style={styles.backButtonText}>← Back</Text>
          </Pressable>
          <Text style={styles.title}>Widget Settings</Text>
        </View>
        <View style={styles.loadingContainer}>
          <Text style={styles.loadingText}>{t('common.loading')}</Text>
        </View>
      </View>
    );
  }

  if (!isSupported) {
    return (
      <View style={styles.container}>
        <View style={styles.header}>
          <Pressable onPress={() => onNavigate('Dashboard')} style={styles.backButton}>
            <Text style={styles.backButtonText}>← Back</Text>
          </Pressable>
          <Text style={styles.title}>Widget Settings</Text>
        </View>
        <View style={styles.unsupportedContainer}>
          <Text style={styles.unsupportedIcon}>📱</Text>
          <Text style={styles.unsupportedTitle}>Widgets Not Available</Text>
          <Text style={styles.unsupportedText}>
            {Platform.OS === 'ios'
              ? 'Home screen widgets require iOS 14 or later.'
              : 'Widgets are not supported on this device.'}
          </Text>
        </View>
      </View>
    );
  }

  return (
    <View style={styles.container}>
      <View style={styles.header}>
        <Pressable onPress={() => onNavigate('Dashboard')} style={styles.backButton}>
          <Text style={styles.backButtonText}>← Back</Text>
        </Pressable>
        <Text style={styles.title}>Widget Settings</Text>
      </View>

      <ScrollView style={styles.content}>
        <View style={styles.section}>
          <Text style={styles.sectionTitle}>Available Widgets</Text>
          <Text style={styles.sectionDescription}>
            Configure which widgets appear on your home screen.
          </Text>

          {availableTypes.map((widgetType) => {
            const isEnabled = configs.some((c) => c.type === widgetType.type);
            return (
              <View key={widgetType.type} style={styles.widgetItem}>
                <View style={styles.widgetInfo}>
                  <Text style={styles.widgetName}>{widgetType.name}</Text>
                  <Text style={styles.widgetDescription}>{widgetType.description}</Text>
                  <View style={styles.sizeChips}>
                    {widgetType.sizes.map((size) => (
                      <View key={size} style={styles.sizeChip}>
                        <Text style={styles.sizeChipText}>{size}</Text>
                      </View>
                    ))}
                  </View>
                </View>
                <Switch
                  value={isEnabled}
                  onValueChange={(value) => handleToggleWidget(widgetType.type, value)}
                  trackColor={{ false: colors.border, true: colors.accentDisabled }}
                  thumbColor={isEnabled ? colors.accent : colors.surfaceMuted}
                />
              </View>
            );
          })}
        </View>

        <View style={styles.section}>
          <Text style={styles.sectionTitle}>Active Widgets</Text>
          {configs.length === 0 ? (
            <Text style={styles.emptyText}>{t('widgets.emptyText')}</Text>
          ) : (
            configs.map((config) => {
              const meta = availableTypes.find((wt) => wt.type === config.type);
              return (
                <View key={config.id} style={styles.activeWidgetItem}>
                  <View style={styles.widgetInfo}>
                    <Text style={styles.widgetName}>{meta?.name ?? config.type}</Text>
                    <Text style={styles.widgetMeta}>
                      Size: {config.size} • Refresh: {config.refreshInterval}min
                    </Text>
                  </View>
                  <Pressable
                    style={styles.removeButton}
                    onPress={() => handleToggleWidget(config.type, false)}
                  >
                    <Text style={styles.removeButtonText}>Remove</Text>
                  </Pressable>
                </View>
              );
            })
          )}
        </View>

        <Pressable style={styles.refreshButton} onPress={handleRefreshAll}>
          <Text style={styles.refreshButtonText}>🔄 Refresh All Widgets</Text>
        </Pressable>

        <View style={styles.instructionsSection}>
          <Text style={styles.instructionsTitle}>How to Add Widgets</Text>
          {Platform.OS === 'ios' ? (
            <Text style={styles.instructionsText}>
              1. Long press on your home screen{'\n'}
              2. Tap the + button in the top left{'\n'}
              3. Search for "PPT"{'\n'}
              4. Choose a widget size and tap "Add Widget"
            </Text>
          ) : (
            <Text style={styles.instructionsText}>
              1. Long press on your home screen{'\n'}
              2. Tap "Widgets"{'\n'}
              3. Find "PPT Property Management"{'\n'}
              4. Drag a widget to your home screen
            </Text>
          )}
        </View>
      </ScrollView>
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.background,
  },
  header: {
    backgroundColor: colors.surface,
    paddingTop: 60,
    paddingHorizontal: 16,
    paddingBottom: 16,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
  },
  backButton: {
    marginBottom: 8,
  },
  backButtonText: {
    color: colors.accent,
    fontSize: 16,
  },
  title: {
    fontSize: 24,
    fontWeight: 'bold',
    color: colors.text,
  },
  loadingContainer: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
  },
  loadingText: {
    fontSize: 16,
    color: colors.textMuted,
  },
  unsupportedContainer: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
    padding: 32,
  },
  unsupportedIcon: {
    fontSize: 48,
    marginBottom: 16,
  },
  unsupportedTitle: {
    fontSize: 20,
    fontWeight: 'bold',
    color: colors.text,
    marginBottom: 8,
  },
  unsupportedText: {
    fontSize: 16,
    color: colors.textMuted,
    textAlign: 'center',
  },
  content: {
    flex: 1,
  },
  section: {
    backgroundColor: colors.surface,
    marginTop: 16,
    paddingHorizontal: 16,
    paddingVertical: 20,
  },
  sectionTitle: {
    fontSize: 18,
    fontWeight: '600',
    color: colors.text,
    marginBottom: 4,
  },
  sectionDescription: {
    fontSize: 14,
    color: colors.textMuted,
    marginBottom: 16,
  },
  widgetItem: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingVertical: 16,
    borderBottomWidth: 1,
    borderBottomColor: colors.surfaceMuted,
  },
  widgetInfo: {
    flex: 1,
  },
  widgetName: {
    fontSize: 16,
    fontWeight: '500',
    color: colors.text,
  },
  widgetDescription: {
    fontSize: 14,
    color: colors.textMuted,
    marginTop: 2,
  },
  widgetMeta: {
    fontSize: 13,
    color: colors.textSubtle,
    marginTop: 2,
  },
  sizeChips: {
    flexDirection: 'row',
    marginTop: 8,
    // Using margin on children for RN < 0.71 compatibility (gap requires 0.71+)
  },
  sizeChip: {
    backgroundColor: colors.surfaceMuted,
    paddingHorizontal: 8,
    paddingVertical: 4,
    borderRadius: 4,
    marginRight: 6,
  },
  sizeChipText: {
    fontSize: 12,
    color: colors.textMuted,
    textTransform: 'capitalize',
  },
  emptyText: {
    fontSize: 14,
    color: colors.textSubtle,
    fontStyle: 'italic',
  },
  activeWidgetItem: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingVertical: 12,
    borderBottomWidth: 1,
    borderBottomColor: colors.surfaceMuted,
  },
  removeButton: {
    backgroundColor: colors.dangerBg,
    paddingHorizontal: 12,
    paddingVertical: 6,
    borderRadius: 6,
  },
  removeButtonText: {
    color: colors.dangerDark,
    fontSize: 14,
    fontWeight: '500',
  },
  refreshButton: {
    backgroundColor: colors.accent,
    marginHorizontal: 16,
    marginTop: 24,
    paddingVertical: 14,
    borderRadius: 8,
    alignItems: 'center',
  },
  refreshButtonText: {
    color: colors.surface,
    fontSize: 16,
    fontWeight: '600',
  },
  instructionsSection: {
    marginHorizontal: 16,
    marginTop: 24,
    marginBottom: 32,
    backgroundColor: colors.accentSoft,
    padding: 16,
    borderRadius: 8,
  },
  instructionsTitle: {
    fontSize: 16,
    fontWeight: '600',
    color: colors.accentDark,
    marginBottom: 8,
  },
  instructionsText: {
    fontSize: 14,
    color: colors.accentDark,
    lineHeight: 22,
  },
});
