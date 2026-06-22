import { useQueryClient } from '@tanstack/react-query';
import * as ImagePicker from 'expo-image-picker';
import * as Location from 'expo-location';
import { useCallback, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  ActivityIndicator,
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
import { PendingSyncIndicator } from '../../components/sync';
import { useOfflineSupport } from '../../hooks';
import { apiRequest, useApiQuery, useTenantId } from '../../hooks/useApi';
import { bytesToMb, compressImagesIfNeeded, generateIdempotencyKey } from '../../utils';
import { colors } from '../shared/screenStyles';
import type { FaultCategory, FaultPriority } from './FaultsListScreen';

interface ReportFaultScreenProps {
  onSuccess?: () => void;
  onCancel?: () => void;
}

/** Subset of the api-server `BuildingSummary` the building picker needs. */
interface ApiBuildingSummary {
  id: string;
  name?: string | null;
  street?: string | null;
  city?: string | null;
}

interface ApiBuildingsListResponse {
  buildings: ApiBuildingSummary[];
}

/** Request body for `POST /api/v1/faults` (mirrors `CreateFaultRequest`). */
interface CreateFaultVariables {
  building_id: string;
  title: string;
  description: string;
  category: string;
  priority: string;
  location_description?: string;
  // Story 2B.7 idempotency key: the backend create dedupes on this, so a
  // create that is queued offline and replayed (possibly more than once) never
  // produces a duplicate fault.
  idempotency_key: string;
}

/** Response from `POST /api/v1/faults` (mirrors `CreateFaultResponse`). */
interface CreateFaultResult {
  id: string;
  message: string;
}

/** A building's display label: prefer its name, fall back to the address. */
function buildingLabel(b: ApiBuildingSummary): string {
  if (b.name?.trim()) return b.name;
  const address = [b.street, b.city].filter(Boolean).join(', ');
  return address || b.id;
}

const categories: Array<{ value: FaultCategory; labelKey: string; icon: string }> = [
  { value: 'plumbing', labelKey: 'plumbing', icon: '🚿' },
  { value: 'electrical', labelKey: 'electrical', icon: '⚡' },
  { value: 'structural', labelKey: 'structural', icon: '🏗️' },
  { value: 'hvac', labelKey: 'hvac', icon: '❄️' },
  { value: 'elevator', labelKey: 'elevator', icon: '🛗' },
  { value: 'security', labelKey: 'security', icon: '🔒' },
  { value: 'other', labelKey: 'other', icon: '🔧' },
];

const priorities: Array<{ value: FaultPriority; labelKey: string; color: string }> = [
  { value: 'low', labelKey: 'priorityLow', color: colors.priorityLowInk },
  { value: 'medium', labelKey: 'priorityMedium', color: colors.priorityMediumInk },
  { value: 'high', labelKey: 'priorityHigh', color: colors.priorityHighInk },
  { value: 'urgent', labelKey: 'priorityUrgent', color: colors.priorityUrgentInk },
];

export function ReportFaultScreen({ onSuccess, onCancel }: ReportFaultScreenProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [buildingId, setBuildingId] = useState<string | null>(null);
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [category, setCategory] = useState<FaultCategory | null>(null);
  const [priority, setPriority] = useState<FaultPriority>('medium');
  const [location, setLocation] = useState('');
  const [photos, setPhotos] = useState<string[]>([]);
  const [isDetectingLocation, setIsDetectingLocation] = useState(false);
  const [isCompressing, setIsCompressing] = useState(false);
  const [compressionNotice, setCompressionNotice] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [queuedOffline, setQueuedOffline] = useState(false);
  const [errors, setErrors] = useState<Record<string, string>>({});

  // Connectivity + the persisted offline action queue (AC 4.1). When the
  // device is offline the create is enqueued and the global sync machinery
  // (badge in the tab bar + `processQueue` on reconnect) replays it.
  const { isConnected, isInternetReachable, addToQueue } = useOfflineSupport();
  const isOffline = !isConnected || isInternetReachable === false;

  // One idempotency key per report attempt, generated lazily and reused across
  // every retry/replay so the backend returns the existing fault instead of
  // inserting a duplicate. Kept in a ref (not state) so it survives re-renders
  // without triggering them.
  const idempotencyKeyRef = useRef<string | null>(null);

  // The api-server requires a `building_id` on every fault. Load the
  // buildings the user belongs to so they can pick which one the fault is for.
  //
  // `GET /api/v1/buildings` requires `organization_id` as a query param
  // (a required, non-defaulted Uuid that must equal the RLS tenant) — without
  // it the request 400s/403s and the picker never loads. The org id is the
  // JWT `tenant_id` claim, surfaced here via `useTenantId`. We gate the query
  // on it (`enabled`) and key the cache on it so a tenant switch refetches.
  const { tenantId, isLoading: isTenantLoading } = useTenantId();
  const buildingsQuery = useApiQuery<ApiBuildingsListResponse>(
    ['buildings', 'list', tenantId],
    `/api/v1/buildings?organization_id=${tenantId ?? ''}`,
    { staleTime: 60_000, enabled: Boolean(tenantId) }
  );
  const buildings = buildingsQuery.data?.buildings ?? [];
  // Show the loading state while we resolve the tenant id too, otherwise the
  // gated-off query would flash the empty/"no buildings" state first.
  const isBuildingsLoading = isTenantLoading || buildingsQuery.isLoading;

  const validateForm = (): boolean => {
    const newErrors: Record<string, string> = {};

    if (!buildingId) {
      newErrors.building = t('faults.selectBuilding');
    }

    if (!title.trim()) {
      newErrors.title = t('faults.titleRequired');
    } else if (title.length < 5) {
      newErrors.title = t('faults.titleMinLength');
    }

    if (!description.trim()) {
      newErrors.description = t('faults.descriptionRequired');
    } else if (description.length < 20) {
      newErrors.description = t('faults.descriptionMinLength');
    }

    if (!category) {
      newErrors.category = t('faults.selectCategory');
    }

    if (!location.trim()) {
      newErrors.location = t('faults.locationRequired');
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const pickImage = async (useCamera: boolean) => {
    try {
      if (useCamera) {
        const { status } = await ImagePicker.requestCameraPermissionsAsync();
        if (status !== 'granted') {
          Alert.alert(t('permissions.denied'), t('faults.cameraPermissionDenied'));
          return;
        }
      } else {
        const { status } = await ImagePicker.requestMediaLibraryPermissionsAsync();
        if (status !== 'granted') {
          Alert.alert(t('permissions.denied'), t('faults.photoLibraryPermissionDenied'));
          return;
        }
      }

      const result = await (useCamera
        ? ImagePicker.launchCameraAsync({
            mediaTypes: 'images',
            quality: 0.8,
            allowsEditing: true,
          })
        : ImagePicker.launchImageLibraryAsync({
            mediaTypes: 'images',
            quality: 0.8,
            allowsMultipleSelection: true,
            selectionLimit: 5 - photos.length,
          }));

      if (!result.canceled) {
        const newPhotos = result.assets.map((asset: { uri: string }) => asset.uri);
        const combined = [...photos, ...newPhotos].slice(0, 5);
        await applyPhotos(combined);
      }
    } catch (_error) {
      Alert.alert(t('common.error'), t('faults.failedToPickImage'));
    }
  };

  // Set the photo list, compressing first when the selection exceeds 10 MB
  // total (AC 4.1). Compression is lossy, so the user is warned once a pass
  // runs; under the limit the originals are kept untouched.
  const applyPhotos = async (next: string[]) => {
    setIsCompressing(true);
    try {
      const result = await compressImagesIfNeeded(next);
      setPhotos(result.uris);
      if (result.compressed) {
        const notice = t('faults.photosCompressedWarning', {
          original: bytesToMb(result.originalBytes),
          final: bytesToMb(result.finalBytes),
        });
        setCompressionNotice(notice);
        Alert.alert(t('faults.photosCompressedTitle'), notice);
      } else {
        setCompressionNotice(null);
      }
    } finally {
      setIsCompressing(false);
    }
  };

  const removePhoto = (index: number) => {
    setPhotos((prev) => {
      const next = prev.filter((_, i) => i !== index);
      if (next.length === 0) {
        setCompressionNotice(null);
      }
      return next;
    });
  };

  const detectLocation = useCallback(async () => {
    setIsDetectingLocation(true);
    try {
      const { status } = await Location.requestForegroundPermissionsAsync();
      if (status !== 'granted') {
        Alert.alert(t('permissions.denied'), t('permissions.locationRequired'));
        return;
      }

      const currentLocation = await Location.getCurrentPositionAsync({});
      const [address] = await Location.reverseGeocodeAsync({
        latitude: currentLocation.coords.latitude,
        longitude: currentLocation.coords.longitude,
      });

      if (address) {
        const locationText = [address.street, address.city].filter(Boolean).join(', ');
        setLocation(locationText || t('faults.locationDetected'));
      }
    } catch (_error) {
      Alert.alert(t('common.error'), t('faults.failedToDetectLocation'));
    } finally {
      setIsDetectingLocation(false);
    }
  }, [t]);

  // Persist the create to the offline queue for replay on reconnect.
  const queueForLater = async (payload: CreateFaultVariables, message: string) => {
    await addToQueue({
      type: 'CREATE',
      endpoint: '/api/v1/faults',
      method: 'POST',
      body: payload,
    });
    setQueuedOffline(true);
    Alert.alert(t('faults.queuedTitle'), message, [{ text: t('common.ok'), onPress: onSuccess }]);
  };

  const handleSubmit = async () => {
    if (!validateForm() || !buildingId) {
      return;
    }

    // Reuse the same key across every retry/replay so the idempotent backend
    // create never produces a duplicate, even if the request below half-fails
    // and is later replayed from the queue.
    if (!idempotencyKeyRef.current) {
      idempotencyKeyRef.current = generateIdempotencyKey();
    }

    // NOTE: `photos` holds local device URIs (compressed in-place when over
    // 10 MB). Uploading attachments needs the presigned-URL pipeline
    // (POST /faults/{id}/attachments), which has no shared mobile helper yet —
    // tracked as a follow-up. The fault itself is created/queued here.
    const payload: CreateFaultVariables = {
      building_id: buildingId,
      title: title.trim(),
      description: description.trim(),
      category: category ?? 'other',
      priority,
      location_description: location.trim() || undefined,
      idempotency_key: idempotencyKeyRef.current,
    };

    setIsSubmitting(true);
    try {
      // Offline: queue immediately rather than firing a request that will fail.
      if (isOffline) {
        await queueForLater(payload, t('faults.queuedMessage'));
        return;
      }

      await apiRequest<CreateFaultResult>('/api/v1/faults', { method: 'POST', body: payload });
      // Refresh the faults list so the new report shows on return.
      queryClient.invalidateQueries({ queryKey: ['faults', 'list'] });
      // The key has been consumed; a subsequent report gets a fresh one.
      idempotencyKeyRef.current = null;
      Alert.alert(t('common.done'), t('faults.successSubmit'), [
        { text: t('common.ok'), onPress: onSuccess },
      ]);
    } catch (error) {
      // The request failed mid-flight. We can't tell a dropped connection from
      // a server error here, so queue for replay: a transient/5xx error retries
      // on reconnect, and a genuine 4xx is dropped by the queue (it marks 4xx
      // permanent) instead of looping. The idempotency key guarantees that if
      // the fault was in fact created, the replay won't duplicate it.
      await queueForLater(payload, t('faults.queuedOnErrorMessage'));
      void error;
    } finally {
      setIsSubmitting(false);
    }
  };

  // Category label mapping (these would ideally come from translations)
  const getCategoryLabel = (labelKey: string): string => {
    const labels: Record<string, string> = {
      plumbing: 'Plumbing',
      electrical: 'Electrical',
      structural: 'Structural',
      hvac: 'HVAC',
      elevator: 'Elevator',
      security: 'Security',
      other: 'Other',
    };
    return labels[labelKey] || labelKey;
  };

  return (
    <KeyboardAvoidingView
      style={styles.container}
      behavior={Platform.OS === 'ios' ? 'padding' : 'height'}
    >
      {/* Header */}
      <View style={styles.header}>
        <Pressable style={styles.cancelButton} onPress={onCancel}>
          <Text style={styles.cancelText}>{t('common.cancel')}</Text>
        </Pressable>
        <Text style={styles.headerTitle}>{t('faults.reportFault')}</Text>
        <View style={styles.placeholder} />
      </View>

      <ScrollView style={styles.scrollView} keyboardShouldPersistTaps="handled">
        {/* Building */}
        <View style={styles.formGroup}>
          <Text style={styles.label}>{t('faults.buildingLabel')} *</Text>
          {isBuildingsLoading ? (
            <Text style={styles.helperText}>{t('faults.loadingBuildings')}</Text>
          ) : buildingsQuery.error ? (
            <Text style={styles.errorText}>{t('faults.buildingsLoadError')}</Text>
          ) : buildings.length === 0 ? (
            <Text style={styles.helperText}>{t('faults.noBuildings')}</Text>
          ) : (
            <View style={styles.categoryGrid}>
              {buildings.map((b) => (
                <Pressable
                  key={b.id}
                  style={[
                    styles.categoryButton,
                    buildingId === b.id && styles.categoryButtonSelected,
                  ]}
                  onPress={() => setBuildingId(b.id)}
                >
                  <Text
                    style={[
                      styles.categoryLabel,
                      buildingId === b.id && styles.categoryLabelSelected,
                    ]}
                  >
                    {buildingLabel(b)}
                  </Text>
                </Pressable>
              ))}
            </View>
          )}
          {errors.building && <Text style={styles.errorText}>{errors.building}</Text>}
        </View>

        {/* Title */}
        <View style={styles.formGroup}>
          <Text style={styles.label}>{t('faults.titleLabel')} *</Text>
          <TextInput
            style={[styles.input, errors.title ? styles.inputError : undefined]}
            placeholder={t('faults.titlePlaceholder')}
            value={title}
            onChangeText={setTitle}
            maxLength={100}
          />
          {errors.title && <Text style={styles.errorText}>{errors.title}</Text>}
        </View>

        {/* Category */}
        <View style={styles.formGroup}>
          <Text style={styles.label}>{t('faults.categoryLabel')} *</Text>
          <View style={styles.categoryGrid}>
            {categories.map((cat) => (
              <Pressable
                key={cat.value}
                style={[
                  styles.categoryButton,
                  category === cat.value && styles.categoryButtonSelected,
                ]}
                onPress={() => setCategory(cat.value)}
              >
                <Text style={styles.categoryIcon}>{cat.icon}</Text>
                <Text
                  style={[
                    styles.categoryLabel,
                    category === cat.value && styles.categoryLabelSelected,
                  ]}
                >
                  {getCategoryLabel(cat.labelKey)}
                </Text>
              </Pressable>
            ))}
          </View>
          {errors.category && <Text style={styles.errorText}>{errors.category}</Text>}
        </View>

        {/* Priority */}
        <View style={styles.formGroup}>
          <Text style={styles.label}>{t('faults.priorityLabel')}</Text>
          <View style={styles.priorityRow}>
            {priorities.map((p) => (
              <Pressable
                key={p.value}
                style={[
                  styles.priorityButton,
                  priority === p.value && { backgroundColor: p.color, borderColor: p.color },
                ]}
                onPress={() => setPriority(p.value)}
              >
                <Text
                  style={[
                    styles.priorityLabel,
                    priority === p.value && styles.priorityLabelSelected,
                  ]}
                >
                  {t(`faults.${p.labelKey}`)}
                </Text>
              </Pressable>
            ))}
          </View>
        </View>

        {/* Location */}
        <View style={styles.formGroup}>
          <Text style={styles.label}>{t('faults.locationLabel')} *</Text>
          <View style={styles.locationRow}>
            <TextInput
              style={[
                styles.input,
                styles.locationInput,
                errors.location ? styles.inputError : undefined,
              ]}
              placeholder={t('faults.locationPlaceholder')}
              value={location}
              onChangeText={setLocation}
            />
            <Pressable
              style={styles.detectButton}
              onPress={detectLocation}
              disabled={isDetectingLocation}
            >
              {isDetectingLocation ? (
                <ActivityIndicator size="small" color={colors.accent} />
              ) : (
                <Text style={styles.detectButtonText}>📍</Text>
              )}
            </Pressable>
          </View>
          {errors.location && <Text style={styles.errorText}>{errors.location}</Text>}
        </View>

        {/* Description */}
        <View style={styles.formGroup}>
          <Text style={styles.label}>{t('faults.descriptionLabel')} *</Text>
          <TextInput
            style={[
              styles.input,
              styles.textArea,
              errors.description ? styles.inputError : undefined,
            ]}
            placeholder={t('faults.descriptionPlaceholder')}
            value={description}
            onChangeText={setDescription}
            multiline
            numberOfLines={4}
            textAlignVertical="top"
          />
          {errors.description && <Text style={styles.errorText}>{errors.description}</Text>}
        </View>

        {/* Photos */}
        <View style={styles.formGroup}>
          <Text style={styles.label}>{t('faults.photosLabel')}</Text>
          <View style={styles.photosContainer}>
            {photos.map((photo, index) => (
              <View key={photo} style={styles.photoWrapper}>
                <Image source={{ uri: photo }} style={styles.photoPreview} />
                <Pressable style={styles.removePhoto} onPress={() => removePhoto(index)}>
                  <Text style={styles.removePhotoText}>✕</Text>
                </Pressable>
              </View>
            ))}
            {photos.length < 5 && (
              <View style={styles.photoActions}>
                <Pressable style={styles.photoButton} onPress={() => pickImage(true)}>
                  <Text style={styles.photoButtonIcon}>📷</Text>
                  <Text style={styles.photoButtonText}>{t('faults.cameraButton')}</Text>
                </Pressable>
                <Pressable style={styles.photoButton} onPress={() => pickImage(false)}>
                  <Text style={styles.photoButtonIcon}>🖼️</Text>
                  <Text style={styles.photoButtonText}>{t('faults.galleryButton')}</Text>
                </Pressable>
              </View>
            )}
          </View>
          <Text style={styles.photoHint}>{t('faults.photosHint', { current: photos.length })}</Text>
          {isCompressing && (
            <View style={styles.compressingRow}>
              <ActivityIndicator size="small" color={colors.accent} />
              <Text style={styles.compressingText}>{t('faults.compressingPhotos')}</Text>
            </View>
          )}
          {compressionNotice && (
            <View style={styles.compressionNotice}>
              <Text style={styles.compressionNoticeIcon}>🗜️</Text>
              <Text style={styles.compressionNoticeText}>{compressionNotice}</Text>
            </View>
          )}
        </View>

        {/* Offline / pending-sync status (AC 4.1) */}
        {queuedOffline ? (
          <PendingSyncIndicator status="pending" />
        ) : isOffline ? (
          <View style={styles.offlineHint}>
            <Text style={styles.offlineHintIcon}>📡</Text>
            <Text style={styles.offlineHintText}>{t('faults.offlineSubmitHint')}</Text>
          </View>
        ) : null}

        {/* Submit Button */}
        <Pressable
          style={[
            styles.submitButton,
            (isSubmitting || isCompressing) && styles.submitButtonDisabled,
          ]}
          onPress={handleSubmit}
          disabled={isSubmitting || isCompressing}
        >
          {isSubmitting ? (
            <ActivityIndicator color={colors.surface} />
          ) : (
            <Text style={styles.submitButtonText}>
              {isOffline ? t('faults.submitButtonOffline') : t('faults.submitButton')}
            </Text>
          )}
        </Pressable>

        <View style={styles.bottomSpacer} />
      </ScrollView>
    </KeyboardAvoidingView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.background,
  },
  header: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    padding: 20,
    paddingTop: 60,
    backgroundColor: colors.surface,
    borderBottomWidth: 1,
    borderBottomColor: colors.border,
  },
  cancelButton: {
    padding: 4,
  },
  cancelText: {
    color: colors.textMuted,
    fontSize: 16,
  },
  headerTitle: {
    fontSize: 18,
    fontWeight: '600',
    color: colors.text,
  },
  placeholder: {
    width: 50,
  },
  scrollView: {
    flex: 1,
    padding: 16,
  },
  formGroup: {
    marginBottom: 20,
  },
  label: {
    fontSize: 14,
    fontWeight: '600',
    color: colors.textSecondary,
    marginBottom: 8,
  },
  input: {
    backgroundColor: colors.surface,
    borderRadius: 8,
    borderWidth: 1,
    borderColor: colors.borderStrong,
    padding: 12,
    fontSize: 16,
  },
  inputError: {
    borderColor: colors.danger,
  },
  errorText: {
    color: colors.danger,
    fontSize: 12,
    marginTop: 4,
  },
  helperText: {
    color: colors.textMuted,
    fontSize: 13,
  },
  textArea: {
    minHeight: 100,
  },
  categoryGrid: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 8,
  },
  categoryButton: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: 12,
    paddingVertical: 8,
    borderRadius: 8,
    backgroundColor: colors.surface,
    borderWidth: 1,
    borderColor: colors.borderStrong,
    gap: 6,
  },
  categoryButtonSelected: {
    backgroundColor: colors.accentSoft,
    borderColor: colors.accent,
  },
  categoryIcon: {
    fontSize: 18,
  },
  categoryLabel: {
    fontSize: 14,
    color: colors.textSecondary,
  },
  categoryLabelSelected: {
    color: colors.accent,
    fontWeight: '500',
  },
  priorityRow: {
    flexDirection: 'row',
    gap: 8,
  },
  priorityButton: {
    flex: 1,
    paddingVertical: 10,
    borderRadius: 8,
    backgroundColor: colors.surface,
    borderWidth: 1,
    borderColor: colors.borderStrong,
    alignItems: 'center',
  },
  priorityLabel: {
    fontSize: 14,
    color: colors.textSecondary,
    fontWeight: '500',
  },
  priorityLabelSelected: {
    color: colors.surface,
  },
  locationRow: {
    flexDirection: 'row',
    gap: 8,
  },
  locationInput: {
    flex: 1,
  },
  detectButton: {
    backgroundColor: colors.surface,
    borderRadius: 8,
    borderWidth: 1,
    borderColor: colors.borderStrong,
    width: 48,
    alignItems: 'center',
    justifyContent: 'center',
  },
  detectButtonText: {
    fontSize: 20,
  },
  photosContainer: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 8,
  },
  photoWrapper: {
    position: 'relative',
  },
  photoPreview: {
    width: 80,
    height: 80,
    borderRadius: 8,
  },
  removePhoto: {
    position: 'absolute',
    top: -6,
    right: -6,
    backgroundColor: colors.danger,
    width: 22,
    height: 22,
    borderRadius: 11,
    alignItems: 'center',
    justifyContent: 'center',
  },
  removePhotoText: {
    color: colors.surface,
    fontSize: 12,
    fontWeight: 'bold',
  },
  photoActions: {
    flexDirection: 'row',
    gap: 8,
  },
  photoButton: {
    width: 80,
    height: 80,
    backgroundColor: colors.surface,
    borderRadius: 8,
    borderWidth: 1,
    borderColor: colors.borderStrong,
    borderStyle: 'dashed',
    alignItems: 'center',
    justifyContent: 'center',
  },
  photoButtonIcon: {
    fontSize: 24,
    marginBottom: 4,
  },
  photoButtonText: {
    fontSize: 12,
    color: colors.textMuted,
  },
  photoHint: {
    fontSize: 12,
    color: colors.textSubtle,
    marginTop: 6,
  },
  compressingRow: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 8,
    marginTop: 8,
  },
  compressingText: {
    fontSize: 13,
    color: colors.textMuted,
  },
  compressionNotice: {
    flexDirection: 'row',
    alignItems: 'flex-start',
    gap: 8,
    marginTop: 8,
    padding: 10,
    borderRadius: 8,
    backgroundColor: colors.warningBg,
  },
  compressionNoticeIcon: {
    fontSize: 16,
  },
  compressionNoticeText: {
    flex: 1,
    fontSize: 13,
    color: colors.warningDark,
    lineHeight: 18,
  },
  offlineHint: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 8,
    padding: 10,
    borderRadius: 8,
    backgroundColor: colors.warningBg,
    marginBottom: 8,
  },
  offlineHintIcon: {
    fontSize: 16,
  },
  offlineHintText: {
    flex: 1,
    fontSize: 13,
    color: colors.warningDark,
    lineHeight: 18,
  },
  submitButton: {
    backgroundColor: colors.accent,
    borderRadius: 8,
    padding: 16,
    alignItems: 'center',
    marginTop: 8,
  },
  submitButtonDisabled: {
    backgroundColor: colors.accentDisabled,
  },
  submitButtonText: {
    color: colors.surface,
    fontSize: 16,
    fontWeight: '600',
  },
  bottomSpacer: {
    height: 40,
  },
});
