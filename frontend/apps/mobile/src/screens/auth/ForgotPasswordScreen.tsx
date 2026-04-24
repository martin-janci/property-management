/**
 * ForgotPasswordScreen (UC-14.4).
 *
 * Requests a password reset email by POSTing to
 * `/api/v1/auth/password-reset/request`.
 */

import Constants from 'expo-constants';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  ActivityIndicator,
  KeyboardAvoidingView,
  Platform,
  Pressable,
  StyleSheet,
  Text,
  TextInput,
  View,
} from 'react-native';

const API_BASE_URL = (Constants.expoConfig?.extra?.apiUrl as string) || 'http://localhost:8080';
const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

interface ForgotPasswordScreenProps {
  onSignInPress?: () => void;
}

export function ForgotPasswordScreen({ onSignInPress }: ForgotPasswordScreenProps) {
  const { t } = useTranslation();
  const [email, setEmail] = useState('');
  const [emailError, setEmailError] = useState<string>();
  const [generalError, setGeneralError] = useState<string>();
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [submitted, setSubmitted] = useState(false);

  const handleSubmit = async () => {
    setEmailError(undefined);
    setGeneralError(undefined);
    const trimmed = email.trim();
    if (!trimmed) {
      setEmailError(t('auth.emailRequired', 'Email is required'));
      return;
    }
    if (!EMAIL_RE.test(trimmed)) {
      setEmailError(t('auth.invalidEmailFormat', 'Enter a valid email address'));
      return;
    }
    setIsSubmitting(true);
    try {
      const response = await fetch(`${API_BASE_URL}/api/v1/auth/password-reset/request`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ email: trimmed }),
      });
      // Don't leak account existence: any non-network success or failure
      // results in the same confirmation screen.
      if (response.status >= 500) {
        throw new Error(t('auth.serverError', 'Server error. Please try again later.'));
      }
      setSubmitted(true);
    } catch (error) {
      setGeneralError(
        error instanceof Error ? error.message : t('auth.unexpectedError', 'An error occurred')
      );
    } finally {
      setIsSubmitting(false);
    }
  };

  if (submitted) {
    return (
      <View style={styles.container}>
        <View style={styles.contentCentered}>
          <Text style={styles.title}>{t('auth.checkInbox', 'Check your inbox')}</Text>
          <Text style={styles.subtitle}>
            {t(
              'auth.resetEmailSent',
              "If an account exists for {{email}}, we've sent instructions to reset your password.",
              { email }
            )}
          </Text>
          {onSignInPress && (
            <Pressable style={styles.primaryButton} onPress={onSignInPress}>
              <Text style={styles.primaryButtonText}>
                {t('auth.backToSignIn', 'Back to sign in')}
              </Text>
            </Pressable>
          )}
        </View>
      </View>
    );
  }

  return (
    <KeyboardAvoidingView
      style={styles.container}
      behavior={Platform.OS === 'ios' ? 'padding' : 'height'}
    >
      <View style={styles.content}>
        <View style={styles.header}>
          <Text style={styles.title}>{t('auth.resetYourPassword', 'Reset your password')}</Text>
          <Text style={styles.subtitle}>
            {t('auth.resetPasswordSubtitle', "Enter your email and we'll send you a reset link.")}
          </Text>
        </View>

        <View style={styles.form}>
          {generalError && (
            <View style={styles.alert}>
              <Text style={styles.alertText}>{generalError}</Text>
            </View>
          )}

          <View style={styles.inputGroup}>
            <Text style={styles.label}>{t('auth.email', 'Email')}</Text>
            <TextInput
              style={[styles.input, emailError ? styles.inputError : undefined]}
              value={email}
              onChangeText={setEmail}
              keyboardType="email-address"
              autoComplete="email"
              autoCapitalize="none"
              editable={!isSubmitting}
            />
            {emailError && <Text style={styles.errorText}>{emailError}</Text>}
          </View>

          <Pressable
            style={[styles.primaryButton, isSubmitting && styles.primaryButtonDisabled]}
            onPress={handleSubmit}
            disabled={isSubmitting}
          >
            {isSubmitting ? (
              <ActivityIndicator color="#fff" />
            ) : (
              <Text style={styles.primaryButtonText}>
                {t('auth.sendResetLink', 'Send reset link')}
              </Text>
            )}
          </Pressable>

          {onSignInPress && (
            <Pressable style={styles.secondary} onPress={onSignInPress}>
              <Text style={styles.link}>{t('auth.backToSignIn', 'Back to sign in')}</Text>
            </Pressable>
          )}
        </View>
      </View>
    </KeyboardAvoidingView>
  );
}

const styles = StyleSheet.create({
  container: { flex: 1, backgroundColor: '#f5f5f5' },
  content: { flex: 1, padding: 24, justifyContent: 'center' },
  contentCentered: { flex: 1, padding: 24, justifyContent: 'center', alignItems: 'center' },
  header: { alignItems: 'center', marginBottom: 24 },
  title: { fontSize: 24, fontWeight: '600', color: '#1f2937', marginBottom: 4 },
  subtitle: { fontSize: 16, color: '#6b7280', textAlign: 'center', marginBottom: 16 },
  form: { gap: 16 },
  alert: { padding: 12, backgroundColor: '#fef2f2', borderRadius: 8 },
  alertText: { color: '#b91c1c', fontSize: 14 },
  inputGroup: { gap: 6 },
  label: { fontSize: 14, fontWeight: '500', color: '#374151' },
  input: {
    backgroundColor: '#fff',
    borderRadius: 8,
    borderWidth: 1,
    borderColor: '#d1d5db',
    padding: 12,
    fontSize: 16,
  },
  inputError: { borderColor: '#ef4444' },
  errorText: { color: '#ef4444', fontSize: 12 },
  primaryButton: {
    backgroundColor: '#2563eb',
    borderRadius: 8,
    padding: 16,
    alignItems: 'center',
    marginTop: 8,
  },
  primaryButtonDisabled: { backgroundColor: '#93c5fd' },
  primaryButtonText: { color: '#fff', fontSize: 16, fontWeight: '600' },
  secondary: { alignItems: 'center', padding: 8 },
  link: { color: '#2563eb', fontSize: 14, fontWeight: '500' },
});
