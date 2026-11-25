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
import { commodityIcon, parseMeters, toUiMeter } from './MetersScreen';
import { uploadMeterPhoto } from './meterPhotoUpload';
import { isValidReading, normalizeReadingInput } from './meterReadingInput';
import { useMyMeters } from './useMyMeters';

export interface MeterReadingScreenProps {
  /** Meter to preselect, passed when the screen is opened from a meter row. */
  meterId?: string;
  onSuccess?: () => void;
  onCancel?: () => void;
}

interface MeterReading {
  id: string;
  meterId: string;
  reading: string;
  unit: string;
  photoUri?: string;
  timestamp: number;
  syncStatus: SyncStatus;
}

export function MeterReadingScreen({ meterId, onSuccess, onCancel }: MeterReadingScreenProps) {
  const { t } = useTranslation();
  const { isConnected, addToQueue } = useOfflineSupport();
  // The resident's real meters: a reading is recorded against a meter id
  // (`SubmitReading.meter_id`), not against a commodity type.
  const metersQuery = useMyMeters();
  const meters = parseMeters(metersQuery.data?.meters ?? []).map(toUiMeter);
  const [pickedMeterId, setPickedMeterId] = useState<string | null>(null);
  // An explicit pick wins, then the meter the screen was opened for, then the
  // only/first meter of the unit. `meterId` alone is enough to submit, so a
  // reading opened from a meter row still works when the list can't load
  // (offline with a cold cache).
  const selectedMeterId = pickedMeterId ?? meterId ?? meters[0]?.id ?? null;
  const selectedMeter = meters.find((m) => m.id === selectedMeterId);
  const [reading, setReading] = useState('');
  const [photoUri, setPhotoUri] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [pendingReadings, setPendingReadings] = useState<MeterReading[]>([]);

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

    if (!selectedMeterId) {
      Alert.alert(t('common.error'), t('meters.meterRequired'));
      return;
    }

    // Guard against non-numeric / malformed values reaching the sync queue.
    // `keyboardType="decimal-pad"` is only a hint — hardware keyboards, paste
    // and autofill can still enter garbage, which was previously queued verbatim.
    if (!isValidReading(reading)) {
      Alert.alert(t('common.error'), t('meters.readingInvalid'));
      return;
    }

    setIsSubmitting(true);

    try {
      // Upload the captured photo to storage FIRST so the reading carries a real
      // storage reference rather than a local `file://` URI the api-server can't
      // resolve. The offline queue only replays a JSON body, so the image bytes
      // can only be uploaded while online; when offline (or when the upload
      // fails) we keep the reading but warn the user the photo wasn't included
      // instead of silently dropping it (mirrors ReportFaultScreen, issue #2999).
      let photoStorageKey: string | undefined;
      let photoDropped = false;
      if (photoUri) {
        if (isConnected) {
          try {
            photoStorageKey = await uploadMeterPhoto(photoUri);
          } catch (uploadError) {
            console.error('Failed to upload meter photo:', uploadError);
            photoDropped = true;
          }
        } else {
          photoDropped = true;
        }
      }

      const readingData: MeterReading = {
        id: `reading-${Date.now()}-${Math.random().toString(36).substring(2)}`,
        meterId: selectedMeterId,
        reading: reading.trim(),
        unit: selectedMeter?.unit ?? '',
        // Local URI kept only for the on-screen pending-list thumbnail; the
        // submitted body carries the uploaded storage key, not this URI.
        photoUri: photoUri || undefined,
        timestamp: Date.now(),
        syncStatus: isConnected ? 'syncing' : 'pending',
      };

      // Add to offline queue.
      // Endpoint must match the api-server route: POST /api/v1/meters/readings
      // (lib.rs nests "/api/v1/meters" + meters.rs "/readings"), which is the
      // same path @ppt/api-client's `submitReading` targets. The previous
      // "/api/v1/meter-readings" had no backing route and 404'd on sync.
      await addToQueue({
        type: 'CREATE',
        endpoint: '/api/v1/meters/readings',
        method: 'POST',
        body: {
          // `SubmitReading` requires the meter's id; the unit of measure is
          // the meter's own and is not part of the request.
          meter_id: readingData.meterId,
          reading: readingData.reading,
          // The uploaded object's storage key, under the api-server's `photo_url`
          // field (`SubmitReading.photo_url`). Omitted (undefined → dropped by
          // JSON.stringify) when no photo was uploaded, so a reading without a
          // photo, an offline reading, or a failed upload simply sends no photo.
          photo_url: photoStorageKey,
        },
      });

      // Add to local pending list for display
      setPendingReadings((prev) => [readingData, ...prev]);

      // Show the outcome. When a photo was captured but couldn't be uploaded
      // (offline, or the upload failed), tell the user it wasn't attached rather
      // than implying the full reading went through unchanged.
      if (photoDropped) {
        Alert.alert(t('meters.photoNotUploadedTitle'), t('meters.photoNotUploadedWarning'), [
          { text: t('common.ok'), onPress: isConnected ? onSuccess : undefined },
        ]);
      } else if (isConnected) {
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
  }, [reading, selectedMeterId, selectedMeter, photoUri, isConnected, addToQueue, t, onSuccess]);

  return (
    <KeyboardAvoidingView
      style={styles.container}
      behavior={Platform.OS === 'ios' ? 'padding' : 'height'}
    >
      {/*
       * keyboardShouldPersistTaps="handled" is load-bearing, not cosmetic: the
       * reading lives in a controlled, focusable TextInput. With the RN default
       * ("never"), the first tap on the Submit Pressable while that field is
       * focused is swallowed to dismiss the keyboard and never reaches onPress,
       * so the entered reading is silently NOT flushed to the submit/offline
       * queue on that tap. Mirrors ReportFaultScreen / DocumentUploadScreen.
       */}
      <ScrollView
        testID="meter-reading-scroll"
        style={styles.scrollView}
        contentContainerStyle={styles.scrollContent}
        keyboardShouldPersistTaps="handled"
      >
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
          {meters.length > 0 ? (
            <View style={styles.meterTypes}>
              {meters.map((meter) => (
                <Pressable
                  key={meter.id}
                  testID={`meter-option-${meter.id}`}
                  style={[
                    styles.meterTypeButton,
                    selectedMeterId === meter.id && styles.meterTypeSelected,
                  ]}
                  onPress={() => setPickedMeterId(meter.id)}
                >
                  <Text style={styles.meterTypeIcon}>{commodityIcon(meter.commodity)}</Text>
                  <Text
                    style={[
                      styles.meterTypeLabel,
                      selectedMeterId === meter.id && styles.meterTypeLabelSelected,
                    ]}
                  >
                    {meter.label}
                  </Text>
                </Pressable>
              ))}
            </View>
          ) : metersQuery.isLoading ? (
            <Text style={styles.meterListNote}>{t('common.loading')}</Text>
          ) : selectedMeterId ? null : (
            <Text style={styles.meterListNote}>
              {metersQuery.error ? t('meters.loadError') : t('meters.emptyText')}
            </Text>
          )}
        </View>

        {/* Reading input */}
        <View style={styles.section}>
          <Text style={styles.sectionTitle}>{t('meters.enterReading')}</Text>
          <View style={styles.readingInputContainer}>
            <TextInput
              style={styles.readingInput}
              value={reading}
              onChangeText={(text) => setReading(normalizeReadingInput(text))}
              keyboardType="decimal-pad"
              placeholder={t('meters.readingPlaceholder')}
              placeholderTextColor={colors.textSubtle}
            />
            <Text style={styles.unitLabel}>{selectedMeter?.unit ?? ''}</Text>
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
                    {commodityIcon(
                      meters.find((m) => m.id === item.meterId)?.commodity ?? 'electricity'
                    )}
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
  meterListNote: {
    fontSize: 14,
    color: colors.textSubtle,
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
