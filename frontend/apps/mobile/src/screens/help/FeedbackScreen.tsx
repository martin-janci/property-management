/**
 * Feedback and bug report screen.
 *
 * Epic 50 - Story 50.4: Feedback & Bug Reports
 */
import { useCallback, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  Alert,
  KeyboardAvoidingView,
  Platform,
  Pressable,
  ScrollView,
  StyleSheet,
  Text,
  TextInput,
  View,
} from 'react-native';

import { feedbackManager as globalFeedbackManager } from '../../onboarding';
import type { FeedbackType } from '../../onboarding/types';
import { colors } from '../shared/screenStyles';

interface FeedbackTypeOption {
  type: FeedbackType;
  label: string;
  icon: string;
}

interface FeedbackScreenProps {
  onNavigate: (screen: string) => void;
}

// Email validation regex
const EMAIL_REGEX = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export function FeedbackScreen({ onNavigate }: FeedbackScreenProps) {
  const { t } = useTranslation();
  const [feedbackType, setFeedbackType] = useState<FeedbackType>('bug');
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [email, setEmail] = useState('');
  const [includeDeviceInfo, setIncludeDeviceInfo] = useState(true);
  const [isSubmitting, setIsSubmitting] = useState(false);

  // Use singleton pattern for FeedbackManager
  const feedbackManager = useMemo(() => {
    // Initialize the global singleton if needed
    globalFeedbackManager.updateContext({ screen: 'Feedback' });
    return globalFeedbackManager;
  }, []);

  const feedbackTypes = useMemo(() => feedbackManager.getFeedbackTypes(), [feedbackManager]);

  const deviceInfo = useMemo(() => feedbackManager.getDeviceInfo(), [feedbackManager]);

  const diagnosticReport = useMemo(
    () => feedbackManager.generateDiagnosticReport(),
    [feedbackManager]
  );

  const handleSubmit = useCallback(async () => {
    if (!title.trim()) {
      Alert.alert(t('common.error'), t('errors.pleaseEnterTitle'));
      return;
    }

    if (!description.trim()) {
      Alert.alert(t('common.error'), t('errors.pleaseEnterDescription'));
      return;
    }

    // Validate email format if provided
    if (email.trim() && !EMAIL_REGEX.test(email.trim())) {
      Alert.alert(t('common.error'), t('errors.invalidEmailFormat'));
      return;
    }

    setIsSubmitting(true);

    const feedback = feedbackManager.createFeedback(
      feedbackType,
      title.trim(),
      description.trim(),
      email.trim() || undefined,
      undefined, // screenshot would be captured here
      includeDeviceInfo // Pass the includeDeviceInfo flag
    );

    const result = await feedbackManager.submitFeedback(feedback);

    setIsSubmitting(false);

    if (result.success) {
      Alert.alert(t('feedback.successTitle'), t('feedback.successMessage'), [
        { text: t('common.ok'), onPress: () => onNavigate('HelpCenter') },
      ]);
    } else {
      Alert.alert(t('feedback.savedForLater'), t('feedback.savedForLaterMessage'), [
        { text: t('common.ok') },
      ]);
    }
  }, [feedbackManager, feedbackType, title, description, email, includeDeviceInfo, onNavigate, t]);

  const handleSaveDraft = useCallback(async () => {
    if (!title.trim() && !description.trim()) {
      Alert.alert(t('common.error'), t('errors.nothingToSave'));
      return;
    }

    await feedbackManager.saveDraft({
      type: feedbackType,
      title: title.trim(),
      description: description.trim(),
      email: email.trim() || undefined,
    });

    Alert.alert(t('common.done'), t('feedback.draftSaved'));
  }, [feedbackManager, feedbackType, title, description, email, t]);

  return (
    <KeyboardAvoidingView
      style={styles.container}
      behavior={Platform.OS === 'ios' ? 'padding' : 'height'}
    >
      <View style={styles.header}>
        <Pressable onPress={() => onNavigate('HelpCenter')} style={styles.backButton}>
          <Text style={styles.backButtonText}>← {t('common.back')}</Text>
        </Pressable>
        <Text style={styles.title}>{t('feedback.title')}</Text>
      </View>

      <ScrollView style={styles.content} keyboardShouldPersistTaps="handled">
        <View style={styles.section}>
          <Text style={styles.sectionTitle}>{t('feedback.typeLabel')}</Text>
          <View style={styles.typeGrid}>
            {feedbackTypes.map((type: FeedbackTypeOption) => (
              <Pressable
                key={type.type}
                style={[styles.typeButton, feedbackType === type.type && styles.typeButtonActive]}
                onPress={() => setFeedbackType(type.type)}
              >
                <Text style={styles.typeIcon}>{type.icon}</Text>
                <Text
                  style={[styles.typeLabel, feedbackType === type.type && styles.typeLabelActive]}
                >
                  {type.label}
                </Text>
              </Pressable>
            ))}
          </View>
        </View>

        <View style={styles.section}>
          <Text style={styles.sectionTitle}>{t('feedback.titleLabel')} *</Text>
          <TextInput
            style={styles.input}
            placeholder={t('feedback.titlePlaceholder')}
            value={title}
            onChangeText={setTitle}
            placeholderTextColor={colors.textSubtle}
          />
        </View>

        <View style={styles.section}>
          <Text style={styles.sectionTitle}>{t('feedback.descriptionLabel')} *</Text>
          <TextInput
            style={[styles.input, styles.textArea]}
            placeholder={t('feedback.descriptionPlaceholder')}
            value={description}
            onChangeText={setDescription}
            multiline
            numberOfLines={6}
            textAlignVertical="top"
            placeholderTextColor={colors.textSubtle}
          />
        </View>

        <View style={styles.section}>
          <Text style={styles.sectionTitle}>{t('feedback.emailLabel')}</Text>
          <TextInput
            style={styles.input}
            placeholder={t('feedback.emailPlaceholder')}
            value={email}
            onChangeText={setEmail}
            keyboardType="email-address"
            autoCapitalize="none"
            autoCorrect={false}
            placeholderTextColor={colors.textSubtle}
          />
          <Text style={styles.helperText}>{t('feedback.emailHelper')}</Text>
        </View>

        <View style={styles.section}>
          <Pressable
            style={styles.checkboxRow}
            onPress={() => setIncludeDeviceInfo(!includeDeviceInfo)}
          >
            <View style={[styles.checkbox, includeDeviceInfo && styles.checkboxChecked]}>
              {includeDeviceInfo && <Text style={styles.checkmark}>✓</Text>}
            </View>
            <Text style={styles.checkboxLabel}>{t('feedback.includeDeviceInfo')}</Text>
          </Pressable>

          {includeDeviceInfo && (
            <View style={styles.deviceInfoPreview}>
              <Text style={styles.deviceInfoTitle}>{t('feedback.deviceInformation')}</Text>
              <Text style={styles.deviceInfoText}>
                Platform: {deviceInfo.platform} {deviceInfo.osVersion}
              </Text>
              <Text style={styles.deviceInfoText}>
                App: v{deviceInfo.appVersion} (Build {deviceInfo.buildNumber})
              </Text>
              <Pressable
                style={styles.viewDiagnostics}
                onPress={() => Alert.alert('Diagnostic Report', diagnosticReport)}
              >
                <Text style={styles.viewDiagnosticsText}>{t('feedback.viewDiagnostics')}</Text>
              </Pressable>
            </View>
          )}
        </View>

        <View style={styles.buttonContainer}>
          <Pressable
            style={[styles.submitButton, isSubmitting && styles.submitButtonDisabled]}
            onPress={handleSubmit}
            disabled={isSubmitting}
          >
            <Text style={styles.submitButtonText}>
              {isSubmitting ? t('feedback.submitting') : t('feedback.submitButton')}
            </Text>
          </Pressable>

          <Pressable style={styles.draftButton} onPress={handleSaveDraft}>
            <Text style={styles.draftButtonText}>{t('feedback.saveDraft')}</Text>
          </Pressable>
        </View>

        <View style={styles.footer}>
          <Text style={styles.footerText}>{t('feedback.footerText')}</Text>
        </View>
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
  content: {
    flex: 1,
  },
  section: {
    backgroundColor: colors.surface,
    marginTop: 16,
    paddingHorizontal: 16,
    paddingVertical: 16,
  },
  sectionTitle: {
    fontSize: 16,
    fontWeight: '600',
    color: colors.text,
    marginBottom: 12,
  },
  typeGrid: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 8,
  },
  typeButton: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: 14,
    paddingVertical: 10,
    backgroundColor: colors.surfaceMuted,
    borderRadius: 8,
    borderWidth: 2,
    borderColor: 'transparent',
  },
  typeButtonActive: {
    backgroundColor: colors.accentSoft,
    borderColor: colors.accent,
  },
  typeIcon: {
    fontSize: 16,
    marginRight: 6,
  },
  typeLabel: {
    fontSize: 14,
    color: colors.textMuted,
  },
  typeLabelActive: {
    color: colors.accent,
    fontWeight: '500',
  },
  input: {
    backgroundColor: colors.surfaceMuted,
    borderRadius: 8,
    paddingHorizontal: 16,
    paddingVertical: 12,
    fontSize: 16,
    color: colors.text,
  },
  textArea: {
    minHeight: 120,
    paddingTop: 12,
  },
  helperText: {
    fontSize: 13,
    color: colors.textSubtle,
    marginTop: 8,
  },
  checkboxRow: {
    flexDirection: 'row',
    alignItems: 'center',
  },
  checkbox: {
    width: 24,
    height: 24,
    borderWidth: 2,
    borderColor: colors.borderStrong,
    borderRadius: 4,
    marginRight: 12,
    alignItems: 'center',
    justifyContent: 'center',
  },
  checkboxChecked: {
    backgroundColor: colors.accent,
    borderColor: colors.accent,
  },
  checkmark: {
    color: colors.surface,
    fontSize: 14,
    fontWeight: 'bold',
  },
  checkboxLabel: {
    fontSize: 16,
    color: colors.text,
  },
  deviceInfoPreview: {
    marginTop: 16,
    backgroundColor: colors.surfaceMuted,
    borderRadius: 8,
    padding: 12,
  },
  deviceInfoTitle: {
    fontSize: 14,
    fontWeight: '600',
    color: colors.textMuted,
    marginBottom: 8,
  },
  deviceInfoText: {
    fontSize: 13,
    color: colors.textMuted,
    marginBottom: 4,
  },
  viewDiagnostics: {
    marginTop: 8,
  },
  viewDiagnosticsText: {
    fontSize: 14,
    color: colors.accent,
  },
  buttonContainer: {
    padding: 16,
    gap: 12,
  },
  submitButton: {
    backgroundColor: colors.accent,
    paddingVertical: 16,
    borderRadius: 8,
    alignItems: 'center',
  },
  submitButtonDisabled: {
    backgroundColor: colors.accentDisabled,
  },
  submitButtonText: {
    color: colors.surface,
    fontSize: 16,
    fontWeight: '600',
  },
  draftButton: {
    paddingVertical: 14,
    borderRadius: 8,
    alignItems: 'center',
    borderWidth: 1,
    borderColor: colors.borderStrong,
  },
  draftButtonText: {
    color: colors.textMuted,
    fontSize: 16,
    fontWeight: '500',
  },
  footer: {
    padding: 16,
    paddingBottom: 40,
  },
  footerText: {
    fontSize: 14,
    color: colors.textSubtle,
    textAlign: 'center',
    lineHeight: 20,
  },
});
