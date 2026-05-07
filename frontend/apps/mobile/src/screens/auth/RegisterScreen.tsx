/**
 * RegisterScreen (UC-14.1).
 *
 * New account registration. POSTs directly to `/api/v1/auth/register` on the
 * Property Management API server (matches the pattern used by AuthContext
 * for login).
 */

import Constants from 'expo-constants';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  ActivityIndicator,
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
import { colors } from '../shared/screenStyles';

const API_BASE_URL = (Constants.expoConfig?.extra?.apiUrl as string) || 'http://localhost:8080';
const EMAIL_RE = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
const MIN_PASSWORD = 8;

interface RegisterScreenProps {
  onSignInPress?: () => void;
  onRegistered?: (email: string) => void;
}

export function RegisterScreen({ onSignInPress, onRegistered }: RegisterScreenProps) {
  const { t } = useTranslation();
  const [firstName, setFirstName] = useState('');
  const [lastName, setLastName] = useState('');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [submittedEmail, setSubmittedEmail] = useState<string>();

  const validate = (): boolean => {
    const next: Record<string, string> = {};
    if (!firstName.trim()) next.firstName = t('auth.firstNameRequired', 'First name is required');
    if (!lastName.trim()) next.lastName = t('auth.lastNameRequired', 'Last name is required');
    if (!email.trim()) next.email = t('auth.emailRequired', 'Email is required');
    else if (!EMAIL_RE.test(email.trim()))
      next.email = t('auth.invalidEmailFormat', 'Enter a valid email address');
    if (!password) next.password = t('auth.passwordRequired', 'Password is required');
    else if (password.length < MIN_PASSWORD)
      next.password = t('auth.passwordMinLength', 'Password must be at least 8 characters');
    if (confirmPassword !== password)
      next.confirmPassword = t('auth.passwordsDoNotMatch', 'Passwords do not match');
    setErrors(next);
    return Object.keys(next).length === 0;
  };

  const handleSubmit = async () => {
    if (!validate()) return;
    setIsSubmitting(true);
    try {
      const response = await fetch(`${API_BASE_URL}/api/v1/auth/register`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          email: email.trim(),
          password,
          firstName: firstName.trim(),
          lastName: lastName.trim(),
        }),
      });
      if (!response.ok) {
        const data = await response.json().catch(() => ({}));
        if (response.status === 409) {
          setErrors({
            email: t('auth.emailAlreadyExists', 'An account with this email already exists.'),
          });
          return;
        }
        throw new Error(data.message || t('auth.registrationFailed', 'Registration failed'));
      }
      setSubmittedEmail(email.trim());
      onRegistered?.(email.trim());
    } catch (error) {
      Alert.alert(
        t('auth.registrationFailed', 'Registration failed'),
        error instanceof Error ? error.message : t('auth.unexpectedError', 'An error occurred')
      );
    } finally {
      setIsSubmitting(false);
    }
  };

  if (submittedEmail) {
    return (
      <View style={styles.container}>
        <ScrollView contentContainerStyle={styles.contentCentered}>
          <Text style={styles.title}>{t('auth.checkInbox', 'Check your inbox')}</Text>
          <Text style={styles.subtitle}>
            {t('auth.verificationSent', 'We sent a verification link to {{email}}.', {
              email: submittedEmail,
            })}
          </Text>
          {onSignInPress && (
            <Pressable style={styles.primaryButton} onPress={onSignInPress}>
              <Text style={styles.primaryButtonText}>
                {t('auth.backToSignIn', 'Back to sign in')}
              </Text>
            </Pressable>
          )}
        </ScrollView>
      </View>
    );
  }

  return (
    <KeyboardAvoidingView
      style={styles.container}
      behavior={Platform.OS === 'ios' ? 'padding' : 'height'}
    >
      <ScrollView contentContainerStyle={styles.content}>
        <View style={styles.header}>
          <Text style={styles.title}>{t('auth.createAccount', 'Create account')}</Text>
          <Text style={styles.subtitle}>
            {t('auth.createAccountSubtitle', 'Join Property Management')}
          </Text>
        </View>

        <View style={styles.form}>
          <Field
            label={t('auth.firstName', 'First name')}
            value={firstName}
            onChangeText={setFirstName}
            error={errors.firstName}
            autoComplete="given-name"
            editable={!isSubmitting}
          />
          <Field
            label={t('auth.lastName', 'Last name')}
            value={lastName}
            onChangeText={setLastName}
            error={errors.lastName}
            autoComplete="family-name"
            editable={!isSubmitting}
          />
          <Field
            label={t('auth.email', 'Email')}
            value={email}
            onChangeText={setEmail}
            error={errors.email}
            autoComplete="email"
            keyboardType="email-address"
            autoCapitalize="none"
            editable={!isSubmitting}
          />
          <Field
            label={t('auth.password', 'Password')}
            value={password}
            onChangeText={setPassword}
            error={errors.password}
            secureTextEntry
            autoComplete="password-new"
            editable={!isSubmitting}
          />
          <Field
            label={t('auth.confirmPassword', 'Confirm password')}
            value={confirmPassword}
            onChangeText={setConfirmPassword}
            error={errors.confirmPassword}
            secureTextEntry
            autoComplete="password-new"
            editable={!isSubmitting}
          />

          <Pressable
            style={[styles.primaryButton, isSubmitting && styles.primaryButtonDisabled]}
            onPress={handleSubmit}
            disabled={isSubmitting}
          >
            {isSubmitting ? (
              <ActivityIndicator color={colors.surface} />
            ) : (
              <Text style={styles.primaryButtonText}>
                {t('auth.createAccount', 'Create account')}
              </Text>
            )}
          </Pressable>
        </View>

        {onSignInPress && (
          <View style={styles.footer}>
            <Text style={styles.footerText}>
              {t('auth.alreadyHaveAccount', 'Already have an account?')}
            </Text>
            <Pressable onPress={onSignInPress}>
              <Text style={styles.link}>{t('auth.signIn', 'Sign in')}</Text>
            </Pressable>
          </View>
        )}
      </ScrollView>
    </KeyboardAvoidingView>
  );
}

interface FieldProps {
  label: string;
  value: string;
  onChangeText: (text: string) => void;
  error?: string;
  autoComplete?: 'given-name' | 'family-name' | 'email' | 'password' | 'password-new' | 'name';
  keyboardType?: 'default' | 'email-address';
  autoCapitalize?: 'none' | 'sentences' | 'words' | 'characters';
  secureTextEntry?: boolean;
  editable?: boolean;
}

function Field({
  label,
  value,
  onChangeText,
  error,
  autoComplete,
  keyboardType,
  autoCapitalize,
  secureTextEntry,
  editable,
}: FieldProps) {
  return (
    <View style={styles.inputGroup}>
      <Text style={styles.label}>{label}</Text>
      <TextInput
        style={[styles.input, error ? styles.inputError : undefined]}
        value={value}
        onChangeText={onChangeText}
        autoComplete={autoComplete}
        keyboardType={keyboardType}
        autoCapitalize={autoCapitalize}
        secureTextEntry={secureTextEntry}
        editable={editable}
      />
      {error && <Text style={styles.errorText}>{error}</Text>}
    </View>
  );
}

const styles = StyleSheet.create({
  container: { flex: 1, backgroundColor: colors.background },
  content: { padding: 24, paddingTop: 48 },
  contentCentered: {
    padding: 24,
    flexGrow: 1,
    justifyContent: 'center',
    alignItems: 'center',
  },
  header: { alignItems: 'center', marginBottom: 24 },
  title: { fontSize: 24, fontWeight: '600', color: colors.text, marginBottom: 4 },
  subtitle: { fontSize: 16, color: colors.textMuted, textAlign: 'center', marginBottom: 24 },
  form: { gap: 16 },
  inputGroup: { gap: 6 },
  label: { fontSize: 14, fontWeight: '500', color: colors.textSecondary },
  input: {
    backgroundColor: colors.surface,
    borderRadius: 8,
    borderWidth: 1,
    borderColor: colors.borderStrong,
    padding: 12,
    fontSize: 16,
  },
  inputError: { borderColor: colors.danger },
  errorText: { color: colors.danger, fontSize: 12 },
  primaryButton: {
    backgroundColor: colors.accent,
    borderRadius: 8,
    padding: 16,
    alignItems: 'center',
    marginTop: 8,
  },
  primaryButtonDisabled: { backgroundColor: colors.accentDisabled },
  primaryButtonText: { color: colors.surface, fontSize: 16, fontWeight: '600' },
  footer: {
    flexDirection: 'row',
    justifyContent: 'center',
    alignItems: 'center',
    gap: 4,
    marginTop: 24,
  },
  footerText: { color: colors.textMuted, fontSize: 14 },
  link: { color: colors.accent, fontSize: 14, fontWeight: '500' },
});
