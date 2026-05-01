/**
 * MeterReadingScreen - Offline-capable meter reading submission (Epic 123 - Story 123.3)
 *
 * Allows residents to submit meter readings even when offline.
 * Readings are queued locally and synced when connectivity returns.
 */
import * as ImagePicker from 'expo-image-picker';
import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  Alert,
  Image,
  KeyboardAvoidingView,
  Platform,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  View,
} from 'react-native';
import { PendingSyncIndicator, type SyncStatus } from '../../components/sync';
import { useOfflineSupport } from '../../hooks';
import { colors } from '../shared/screenStyles';

export interface MeterReadingScreenProps {
  onSuccess?: () => void;
  onCancel?: () => void;
}

type MeterType = 'water' | 'electricity' | 'gas' | 'heating';

interface MeterReading {
  id: string;
  meterType: MeterType;
  reading: string;
  unit: string;
  photoUri?: string;
  timestamp: number;
  syncStatus: SyncStatus;
}

const METER_TYPES: { type: MeterType; icon: string; label: string; unit: string }[] = [
  { type: 'water', icon: '💧', label: 'meters.water', unit: 'm³' },
  { type: 'electricity', icon: '⚡', label: 'meters.electricity', unit: 'kWh' },
  { type: 'gas', icon: '🔥', label: 'meters.gas', unit: 'm³' },
  { type: 'heating', icon: '🌡️', label: 'meters.heating', unit: 'GJ' },
];

export function MeterReadingScreen({ onSuccess, onCancel }: MeterReadingScreenProps) {
  const { t } = useTranslation();
  const { isConnected, addToQueue } = useOfflineSupport();
  const [selectedMeter, setSelectedMeter] = useState<MeterType>('water');
  const [reading, setReading] = useState('');
  const [photoUri, setPhotoUri] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [pendingReadings, setPendingReadings] = useState<MeterReading[]>([]);

  const selectedMeterConfig = METER_TYPES.find((m) => m.type === selectedMeter)!;

  const handleTakePhoto = useCallback(async () => {
    const { status } = await ImagePicker.requestCameraPermissionsAsync();
    if (status !== 'granted') {
      Alert.alert(t('permissions.denied'), t('permissions.cameraRequired'));
      return;
    }

    const result = await ImagePicker.launchCameraAsync({
      mediaTypes: 'images',
      allowsEditing: true,
      aspect: [4, 3],
      quality: 0.8,
    });

    if (!result.canceled && result.assets[0]) {
      setPhotoUri(result.assets[0].uri);
    }
  }, [t]);

  const handlePickFromGallery = useCallback(async () => {
    const { status } = await ImagePicker.requestMediaLibraryPermissionsAsync();
    if (status !== 'granted') {
      Alert.alert(t('permissions.denied'), t('permissions.cameraRequired'));
      return;
    }

    const result = await ImagePicker.launchImageLibraryAsync({
      mediaTypes: 'images',
      allowsEditing: true,
      aspect: [4, 3],
      quality: 0.8,
    });

    if (!result.canceled && result.assets[0]) {
      setPhotoUri(result.assets[0].uri);
    }
  }, [t]);

  const handleSubmit = useCallback(async () => {
    if (!reading.trim()) {
      Alert.alert(t('common.error'), t('meters.readingRequired'));
      return;
    }

    setIsSubmitting(true);

    try {
      const readingData: MeterReading = {
        id: `reading-${Date.now()}-${Math.random().toString(36).substring(2)}`,
        meterType: selectedMeter,
        reading: reading.trim(),
        unit: selectedMeterConfig.unit,
        photoUri: photoUri || undefined,
        timestamp: Date.now(),
        syncStatus: isConnected ? 'syncing' : 'pending',
      };

      // Add to offline queue
      await addToQueue({
        type: 'CREATE',
        endpoint: '/api/v1/meter-readings',
        method: 'POST',
        body: {
          meterType: readingData.meterType,
          reading: readingData.reading,
          unit: readingData.unit,
          photoUri: readingData.photoUri,
        },
      });

      // Add to local pending list for display
      setPendingReadings((prev) => [readingData, ...prev]);

      // Show success message
      if (isConnected) {
        Alert.alert(t('common.done'), t('meters.submitSuccess'), [
          { text: t('common.ok'), onPress: onSuccess },
        ]);
      } else {
        Alert.alert(t('meters.savedOffline'), t('meters.willSyncWhenOnline'), [
          { text: t('common.ok') },
        ]);
      }

      // Reset form
      setReading('');
      setPhotoUri(null);
    } catch (error) {
      console.error('Failed to submit meter reading:', error);
      Alert.alert(t('common.error'), t('meters.submitFailed'));
    } finally {
      setIsSubmitting(false);
    }
  }, [
    reading,
    selectedMeter,
    selectedMeterConfig,
    photoUri,
    isConnected,
    addToQueue,
    t,
    onSuccess,
  ]);

  return (
    <KeyboardAvoidingView
      style={styles.container}
      behavior={Platform.OS === 'ios' ? 'padding' : 'height'}
    >
      <ScrollView style={styles.scrollView} contentContainerStyle={styles.scrollContent}>
        {/* Header */}
        <View style={styles.header}>
          <Text style={styles.title}>{t('meters.title')}</Text>
          {onCancel && (
            <Pressable onPress={onCancel} style={styles.cancelButton}>
              <Text style={styles.cancelText}>{t('common.cancel')}</Text>
            </Pressable>
          )}
        </View>

        {/* Offline indicator */}
        {!isConnected && (
          <View style={styles.offlineNotice}>
            <Text style={styles.offlineIcon}>📵</Text>
            <Text style={styles.offlineText}>{t('meters.offlineMode')}</Text>
          </View>
        )}

        {/* Meter type selector */}
        <View style={styles.section}>
          <Text style={styles.sectionTitle}>{t('meters.selectType')}</Text>
          <View style={styles.meterTypes}>
            {METER_TYPES.map((meter) => (
              <Pressable
                key={meter.type}
                style={[
                  styles.meterTypeButton,
                  selectedMeter === meter.type && styles.meterTypeSelected,
                ]}
                onPress={() => setSelectedMeter(meter.type)}
              >
                <Text style={styles.meterTypeIcon}>{meter.icon}</Text>
                <Text
                  style={[
                    styles.meterTypeLabel,
                    selectedMeter === meter.type && styles.meterTypeLabelSelected,
                  ]}
                >
                  {t(meter.label)}
                </Text>
              </Pressable>
            ))}
          </View>
        </View>

        {/* Reading input */}
        <View style={styles.section}>
          <Text style={styles.sectionTitle}>{t('meters.enterReading')}</Text>
          <View style={styles.readingInputContainer}>
            <TextInput
              style={styles.readingInput}
              value={reading}
              onChangeText={setReading}
              keyboardType="decimal-pad"
              placeholder={t('meters.readingPlaceholder')}
              placeholderTextColor={colors.textSubtle}
            />
            <Text style={styles.unitLabel}>{selectedMeterConfig.unit}</Text>
          </View>
        </View>

        {/* Photo capture */}
        <View style={styles.section}>
          <Text style={styles.sectionTitle}>{t('meters.photo')}</Text>
          {photoUri ? (
            <View style={styles.photoPreview}>
              <Image source={{ uri: photoUri }} style={styles.photo} />
              <Pressable style={styles.removePhotoButton} onPress={() => setPhotoUri(null)}>
                <Text style={styles.removePhotoText}>✕</Text>
              </Pressable>
            </View>
          ) : (
            <View style={styles.photoButtons}>
              <Pressable style={styles.photoButton} onPress={handleTakePhoto}>
                <Text style={styles.photoButtonIcon}>📷</Text>
                <Text style={styles.photoButtonLabel}>{t('faults.cameraButton')}</Text>
              </Pressable>
              <Pressable style={styles.photoButton} onPress={handlePickFromGallery}>
                <Text style={styles.photoButtonIcon}>🖼️</Text>
                <Text style={styles.photoButtonLabel}>{t('faults.galleryButton')}</Text>
              </Pressable>
            </View>
          )}
        </View>

        {/* Submit button */}
        <Pressable
          style={[styles.submitButton, isSubmitting && styles.submitButtonDisabled]}
          onPress={handleSubmit}
          disabled={isSubmitting}
        >
          <Text style={styles.submitButtonText}>
            {isSubmitting ? t('common.loading') : t('meters.submit')}
          </Text>
        </Pressable>

        {/* Pending readings list */}
        {pendingReadings.length > 0 && (
          <View style={styles.section}>
            <Text style={styles.sectionTitle}>{t('meters.pendingReadings')}</Text>
            {pendingReadings.map((item) => (
              <View key={item.id} style={styles.pendingItem}>
                <View style={styles.pendingItemHeader}>
                  <Text style={styles.pendingItemIcon}>
                    {METER_TYPES.find((m) => m.type === item.meterType)?.icon}
                  </Text>
                  <Text style={styles.pendingItemText}>
                    {item.reading} {item.unit}
                  </Text>
                </View>
                <PendingSyncIndicator
                  status={item.syncStatus}
                  createdAt={new Date(item.timestamp)}
                  compact
                />
              </View>
            ))}
          </View>
        )}
      </ScrollView>
    </KeyboardAvoidingView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.background,
  },
  scrollView: {
    flex: 1,
  },
  scrollContent: {
    padding: 16,
    paddingTop: 60,
  },
  header: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: 24,
  },
  title: {
    fontSize: 24,
    fontWeight: 'bold',
    color: colors.text,
  },
  cancelButton: {
    padding: 8,
  },
  cancelText: {
    fontSize: 16,
    color: colors.accent,
  },
  offlineNotice: {
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: colors.warningBg,
    padding: 12,
    borderRadius: 8,
    marginBottom: 16,
  },
  offlineIcon: {
    fontSize: 20,
    marginRight: 8,
  },
  offlineText: {
    fontSize: 14,
    color: colors.warningDark,
    flex: 1,
  },
  section: {
    marginBottom: 24,
  },
  sectionTitle: {
    fontSize: 16,
    fontWeight: '600',
    color: colors.textSecondary,
    marginBottom: 12,
  },
  meterTypes: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 12,
  },
  meterTypeButton: {
    flex: 1,
    minWidth: '45%',
    backgroundColor: colors.surface,
    borderWidth: 2,
    borderColor: colors.border,
    borderRadius: 12,
    padding: 16,
    alignItems: 'center',
  },
  meterTypeSelected: {
    borderColor: colors.accent,
    backgroundColor: colors.accentSoft,
  },
  meterTypeIcon: {
    fontSize: 32,
    marginBottom: 8,
  },
  meterTypeLabel: {
    fontSize: 14,
    color: colors.textMuted,
    fontWeight: '500',
  },
  meterTypeLabelSelected: {
    color: colors.accent,
  },
  readingInputContainer: {
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: colors.surface,
    borderWidth: 1,
    borderColor: colors.borderStrong,
    borderRadius: 8,
    overflow: 'hidden',
  },
  readingInput: {
    flex: 1,
    fontSize: 24,
    fontWeight: '600',
    padding: 16,
    color: colors.text,
  },
  unitLabel: {
    fontSize: 18,
    color: colors.textMuted,
    paddingRight: 16,
    fontWeight: '500',
  },
  photoButtons: {
    flexDirection: 'row',
    gap: 12,
  },
  photoButton: {
    flex: 1,
    backgroundColor: colors.surface,
    borderWidth: 1,
    borderColor: colors.borderStrong,
    borderRadius: 8,
    padding: 16,
    alignItems: 'center',
  },
  photoButtonIcon: {
    fontSize: 24,
    marginBottom: 8,
  },
  photoButtonLabel: {
    fontSize: 14,
    color: colors.textMuted,
  },
  photoPreview: {
    position: 'relative',
    borderRadius: 8,
    overflow: 'hidden',
  },
  photo: {
    width: '100%',
    height: 200,
    borderRadius: 8,
  },
  removePhotoButton: {
    position: 'absolute',
    top: 8,
    right: 8,
    backgroundColor: colors.bgOverlay,
    width: 32,
    height: 32,
    borderRadius: 16,
    alignItems: 'center',
    justifyContent: 'center',
  },
  removePhotoText: {
    color: colors.surface,
    fontSize: 16,
    fontWeight: 'bold',
  },
  submitButton: {
    backgroundColor: colors.accent,
    borderRadius: 8,
    padding: 16,
    alignItems: 'center',
    marginBottom: 24,
  },
  submitButtonDisabled: {
    backgroundColor: colors.accentDisabled,
  },
  submitButtonText: {
    color: colors.surface,
    fontSize: 16,
    fontWeight: '600',
  },
  pendingItem: {
    backgroundColor: colors.surface,
    borderRadius: 8,
    padding: 12,
    marginBottom: 8,
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
  pendingItemHeader: {
    flexDirection: 'row',
    alignItems: 'center',
  },
  pendingItemIcon: {
    fontSize: 20,
    marginRight: 8,
  },
  pendingItemText: {
    fontSize: 16,
    fontWeight: '500',
    color: colors.text,
  },
});
