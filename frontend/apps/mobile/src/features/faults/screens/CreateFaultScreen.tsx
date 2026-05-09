/**
 * CreateFaultScreen - screen for reporting a new fault.
 * Epic 4: Fault Reporting & Resolution (UC-03.1)
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  ActivityIndicator,
  KeyboardAvoidingView,
  Platform,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  TouchableOpacity,
  View,
} from 'react-native';
import { colors } from '../../../screens/shared/screenStyles';
import type { FaultCategory } from '../components/FaultCard';

interface CreateFaultScreenProps {
  buildings: Array<{ id: string; name: string }>;
  isSubmitting?: boolean;
  onSubmit: (data: {
    buildingId: string;
    title: string;
    description: string;
    category: FaultCategory;
    locationDescription?: string;
  }) => void;
  onCancel: () => void;
}

const categoryKeys: FaultCategory[] = [
  'plumbing',
  'electrical',
  'heating',
  'structural',
  'exterior',
  'elevator',
  'common_area',
  'security',
  'cleaning',
  'other',
];

export function CreateFaultScreen({
  buildings,
  isSubmitting,
  onSubmit,
  onCancel,
}: CreateFaultScreenProps) {
  const { t } = useTranslation();
  const [buildingId, setBuildingId] = useState('');
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [category, setCategory] = useState<FaultCategory>('other');
  const [locationDescription, setLocationDescription] = useState('');
  const [errors, setErrors] = useState<{ [key: string]: string }>({});

  const validate = () => {
    const newErrors: { [key: string]: string } = {};
    if (!buildingId) newErrors.buildingId = t('faults.selectBuilding');
    if (!title.trim()) newErrors.title = t('faults.titleRequired');
    if (!description.trim()) newErrors.description = t('faults.descriptionRequired');
    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSubmit = () => {
    if (validate()) {
      onSubmit({
        buildingId,
        title: title.trim(),
        description: description.trim(),
        category,
        locationDescription: locationDescription.trim() || undefined,
      });
    }
  };

  return (
    <KeyboardAvoidingView
      style={styles.container}
      behavior={Platform.OS === 'ios' ? 'padding' : 'height'}
    >
      <ScrollView style={styles.scrollView} contentContainerStyle={styles.content}>
        <Text style={styles.label}>{t('faults.buildingLabel')} *</Text>
        <View style={styles.pickerContainer}>
          {buildings.map((b) => (
            <TouchableOpacity
              key={b.id}
              style={[styles.pickerOption, buildingId === b.id && styles.pickerOptionSelected]}
              onPress={() => setBuildingId(b.id)}
            >
              <Text
                style={[
                  styles.pickerOptionText,
                  buildingId === b.id && styles.pickerOptionTextSelected,
                ]}
              >
                {b.name}
              </Text>
            </TouchableOpacity>
          ))}
        </View>
        {errors.buildingId && <Text style={styles.error}>{errors.buildingId}</Text>}

        <Text style={styles.label}>{t('faults.titleLabel')} *</Text>
        <TextInput
          style={[styles.input, errors.title ? styles.inputError : undefined]}
          value={title}
          onChangeText={setTitle}
          placeholder={t('faults.titlePlaceholder')}
          maxLength={255}
        />
        {errors.title && <Text style={styles.error}>{errors.title}</Text>}

        <Text style={styles.label}>{t('faults.descriptionLabel')} *</Text>
        <TextInput
          style={[styles.textArea, errors.description ? styles.inputError : undefined]}
          value={description}
          onChangeText={setDescription}
          placeholder={t('faults.descriptionPlaceholder')}
          multiline
          numberOfLines={4}
          textAlignVertical="top"
        />
        {errors.description && <Text style={styles.error}>{errors.description}</Text>}

        <Text style={styles.label}>{t('faults.locationOptional')}</Text>
        <TextInput
          style={styles.input}
          value={locationDescription}
          onChangeText={setLocationDescription}
          placeholder={t('faults.locationPlaceholder')}
        />

        <Text style={styles.label}>{t('faults.categoryLabel')}</Text>
        <ScrollView horizontal showsHorizontalScrollIndicator={false} style={styles.categoryScroll}>
          {categoryKeys.map((cat) => (
            <TouchableOpacity
              key={cat}
              style={[styles.categoryChip, category === cat && styles.categoryChipSelected]}
              onPress={() => setCategory(cat)}
            >
              <Text
                style={[
                  styles.categoryChipText,
                  category === cat && styles.categoryChipTextSelected,
                ]}
              >
                {t(`faults.category.${cat}`)}
              </Text>
            </TouchableOpacity>
          ))}
        </ScrollView>
      </ScrollView>

      <View style={styles.footer}>
        <TouchableOpacity style={styles.cancelButton} onPress={onCancel} disabled={isSubmitting}>
          <Text style={styles.cancelButtonText}>{t('common.cancel')}</Text>
        </TouchableOpacity>
        <TouchableOpacity
          style={[styles.submitButton, isSubmitting && styles.submitButtonDisabled]}
          onPress={handleSubmit}
          disabled={isSubmitting}
        >
          {isSubmitting ? (
            <ActivityIndicator size="small" color={colors.white} />
          ) : (
            <Text style={styles.submitButtonText}>{t('common.submit')}</Text>
          )}
        </TouchableOpacity>
      </View>
    </KeyboardAvoidingView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.white,
  },
  scrollView: {
    flex: 1,
  },
  content: {
    padding: 16,
    paddingBottom: 100,
  },
  label: {
    fontSize: 14,
    fontWeight: '500',
    color: colors.textSecondary,
    marginTop: 16,
    marginBottom: 8,
  },
  input: {
    borderWidth: 1,
    borderColor: colors.borderStrong,
    borderRadius: 8,
    padding: 12,
    fontSize: 16,
    color: colors.text,
    backgroundColor: colors.white,
  },
  textArea: {
    borderWidth: 1,
    borderColor: colors.borderStrong,
    borderRadius: 8,
    padding: 12,
    fontSize: 16,
    color: colors.text,
    backgroundColor: colors.white,
    minHeight: 100,
  },
  inputError: {
    borderColor: colors.danger,
  },
  error: {
    fontSize: 12,
    color: colors.danger,
    marginTop: 4,
  },
  pickerContainer: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 8,
  },
  pickerOption: {
    paddingHorizontal: 12,
    paddingVertical: 8,
    borderRadius: 8,
    borderWidth: 1,
    borderColor: colors.borderStrong,
    backgroundColor: colors.white,
  },
  pickerOptionSelected: {
    backgroundColor: colors.accent,
    borderColor: colors.accent,
  },
  pickerOptionText: {
    fontSize: 14,
    color: colors.textSecondary,
  },
  pickerOptionTextSelected: {
    color: colors.white,
  },
  categoryScroll: {
    marginTop: 4,
  },
  categoryChip: {
    paddingHorizontal: 16,
    paddingVertical: 8,
    borderRadius: 20,
    backgroundColor: colors.surfaceMuted,
    marginRight: 8,
  },
  categoryChipSelected: {
    backgroundColor: colors.accent,
  },
  categoryChipText: {
    fontSize: 14,
    color: colors.textSecondary,
  },
  categoryChipTextSelected: {
    color: colors.white,
  },
  footer: {
    position: 'absolute',
    bottom: 0,
    left: 0,
    right: 0,
    flexDirection: 'row',
    padding: 16,
    backgroundColor: colors.white,
    borderTopWidth: 1,
    borderTopColor: colors.border,
    gap: 12,
  },
  cancelButton: {
    flex: 1,
    paddingVertical: 14,
    borderRadius: 8,
    borderWidth: 1,
    borderColor: colors.borderStrong,
    alignItems: 'center',
  },
  cancelButtonText: {
    fontSize: 16,
    fontWeight: '500',
    color: colors.textSecondary,
  },
  submitButton: {
    flex: 1,
    paddingVertical: 14,
    borderRadius: 8,
    backgroundColor: colors.accent,
    alignItems: 'center',
  },
  submitButtonDisabled: {
    opacity: 0.5,
  },
  submitButtonText: {
    fontSize: 16,
    fontWeight: '500',
    color: colors.white,
  },
});
