import { useQueryClient } from '@tanstack/react-query';
import * as ImagePicker from 'expo-image-picker';
import * as Location from 'expo-location';
import { useCallback, useState } from 'react';
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
import { useApiMutation, useApiQuery } from '../../hooks/useApi';
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
  const [errors, setErrors] = useState<Record<string, string>>({});

  // The api-server requires a `building_id` on every fault. Load the
  // buildings the user belongs to so they can pick which one the fault is for.
  const buildingsQuery = useApiQuery<ApiBuildingsListResponse>(
    ['buildings', 'list'],
    '/api/v1/buildings',
    { staleTime: 60_000 }
  );
  const buildings = buildingsQuery.data?.buildings ?? [];

  // POST the fault to the api-server. `isPending` drives the submit button's
  // spinner/disabled state (replacing the old local `isSubmitting` flag).
  const createMutation = useApiMutation<CreateFaultResult, CreateFaultVariables>(
    '/api/v1/faults',
    'POST'
  );
  const isSubmitting = createMutation.isPending;

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
        setPhotos((prev) => [...prev, ...newPhotos].slice(0, 5));
      }
    } catch (_error) {
      Alert.alert(t('common.error'), t('faults.failedToPickImage'));
    }
  };

  const removePhoto = (index: number) => {
    setPhotos((prev) => prev.filter((_, i) => i !== index));
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

  const handleSubmit = () => {
    if (!validateForm() || !buildingId) {
      return;
    }

    // NOTE: `photos` holds local device URIs. Uploading attachments needs the
    // presigned-URL pipeline (POST /faults/{id}/attachments), which has no
    // shared mobile helper yet — tracked as a follow-up. The fault itself is
    // created here so reports stop silently disappearing.
    createMutation.mutate(
      {
        building_id: buildingId,
        title: title.trim(),
        description: description.trim(),
        category: category ?? 'other',
        priority,
        location_description: location.trim() || undefined,
      },
      {
        onSuccess: () => {
          // Refresh the faults list so the new report shows on return.
          queryClient.invalidateQueries({ queryKey: ['faults', 'list'] });
          Alert.alert(t('common.done'), t('faults.successSubmit'), [
            { text: t('common.ok'), onPress: onSuccess },
          ]);
        },
        onError: (error) => {
          Alert.alert(t('common.error'), error.message || t('faults.failedSubmit'));
        },
      }
    );
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
          {buildingsQuery.isLoading ? (
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
        </View>

        {/* Submit Button */}
        <Pressable
          style={[styles.submitButton, isSubmitting && styles.submitButtonDisabled]}
          onPress={handleSubmit}
          disabled={isSubmitting}
        >
          {isSubmitting ? (
            <ActivityIndicator color={colors.surface} />
          ) : (
            <Text style={styles.submitButtonText}>{t('faults.submitButton')}</Text>
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
