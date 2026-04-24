/**
 * ProfileScreen (UC-14.6).
 *
 * Authenticated user views and edits their profile, manages biometric login
 * and signs out. Edits are persisted to SecureStore via AuthContext storage
 * keys until the backend `PATCH /me` endpoint is wired in.
 */

import * as SecureStore from 'expo-secure-store';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  Alert,
  KeyboardAvoidingView,
  Platform,
  Pressable,
  ScrollView,
  StyleSheet,
  Switch,
  Text,
  TextInput,
  View,
} from 'react-native';
import { type User, useAuth } from '../../contexts/AuthContext';

const USER_KEY = 'ppt_user';

interface ProfileScreenProps {
  onChangePasswordPress?: () => void;
  onTwoFactorPress?: () => void;
}

export function ProfileScreen({ onChangePasswordPress, onTwoFactorPress }: ProfileScreenProps) {
  const { t } = useTranslation();
  const { user, logout, biometricAvailable, biometricEnabled, enableBiometric, disableBiometric } =
    useAuth();

  const [firstName, setFirstName] = useState(user?.firstName ?? '');
  const [lastName, setLastName] = useState(user?.lastName ?? '');
  const [isSaving, setIsSaving] = useState(false);
  const [savedAt, setSavedAt] = useState<number>();

  const handleSave = async () => {
    if (!user) return;
    if (!firstName.trim() || !lastName.trim()) {
      Alert.alert(
        t('common.error', 'Error'),
        t('auth.nameRequired', 'First and last name are required.')
      );
      return;
    }
    setIsSaving(true);
    try {
      const updated: User = {
        ...user,
        firstName: firstName.trim(),
        lastName: lastName.trim(),
      };
      await SecureStore.setItemAsync(USER_KEY, JSON.stringify(updated));
      setSavedAt(Date.now());
    } catch (error) {
      Alert.alert(
        t('common.error', 'Error'),
        error instanceof Error
          ? error.message
          : t('auth.saveProfileFailed', 'Could not save profile')
      );
    } finally {
      setIsSaving(false);
    }
  };

  const handleToggleBiometric = async (next: boolean) => {
    if (next) {
      const ok = await enableBiometric();
      if (!ok) {
        Alert.alert(
          t('auth.biometricFailed', 'Biometric setup failed'),
          t('auth.biometricEnableHint', 'Please try again.')
        );
      }
    } else {
      await disableBiometric();
    }
  };

  const handleSignOut = () => {
    Alert.alert(
      t('auth.signOut', 'Sign out'),
      t('auth.signOutConfirm', 'Are you sure you want to sign out?'),
      [
        { text: t('common.cancel', 'Cancel'), style: 'cancel' },
        {
          text: t('auth.signOut', 'Sign out'),
          style: 'destructive',
          onPress: () => {
            void logout();
          },
        },
      ]
    );
  };

  if (!user) return null;

  return (
    <KeyboardAvoidingView
      style={styles.container}
      behavior={Platform.OS === 'ios' ? 'padding' : 'height'}
    >
      <ScrollView contentContainerStyle={styles.content}>
        <View style={styles.header}>
          <View style={styles.avatar}>
            <Text style={styles.avatarText}>
              {(firstName.charAt(0) + lastName.charAt(0)).toUpperCase() || '?'}
            </Text>
          </View>
          <Text style={styles.email}>{user.email}</Text>
          <Text style={styles.role}>{user.role}</Text>
        </View>

        <Text style={styles.sectionTitle}>{t('auth.profile', 'Profile')}</Text>
        {savedAt && (
          <View style={styles.success}>
            <Text style={styles.successText}>{t('auth.profileSaved', 'Profile updated.')}</Text>
          </View>
        )}
        <View style={styles.inputGroup}>
          <Text style={styles.label}>{t('auth.firstName', 'First name')}</Text>
          <TextInput
            style={styles.input}
            value={firstName}
            onChangeText={setFirstName}
            autoComplete="given-name"
          />
        </View>
        <View style={styles.inputGroup}>
          <Text style={styles.label}>{t('auth.lastName', 'Last name')}</Text>
          <TextInput
            style={styles.input}
            value={lastName}
            onChangeText={setLastName}
            autoComplete="family-name"
          />
        </View>
        <Pressable
          style={[styles.primaryButton, isSaving && styles.primaryButtonDisabled]}
          onPress={handleSave}
          disabled={isSaving}
        >
          <Text style={styles.primaryButtonText}>
            {isSaving ? t('common.saving', 'Saving…') : t('common.saveChanges', 'Save changes')}
          </Text>
        </Pressable>

        <Text style={styles.sectionTitle}>{t('auth.security', 'Security')}</Text>

        {onChangePasswordPress && (
          <Pressable style={styles.row} onPress={onChangePasswordPress}>
            <Text style={styles.rowText}>{t('auth.changePassword', 'Change password')}</Text>
            <Text style={styles.rowChevron}>›</Text>
          </Pressable>
        )}
        {onTwoFactorPress && (
          <Pressable style={styles.row} onPress={onTwoFactorPress}>
            <Text style={styles.rowText}>{t('auth.twoFactor', 'Two-factor authentication')}</Text>
            <Text style={styles.rowChevron}>›</Text>
          </Pressable>
        )}

        {biometricAvailable && (
          <View style={styles.row}>
            <Text style={styles.rowText}>{t('auth.biometricLogin', 'Biometric login')}</Text>
            <Switch value={biometricEnabled} onValueChange={handleToggleBiometric} />
          </View>
        )}

        <Pressable style={styles.signOutButton} onPress={handleSignOut}>
          <Text style={styles.signOutText}>{t('auth.signOut', 'Sign out')}</Text>
        </Pressable>
      </ScrollView>
    </KeyboardAvoidingView>
  );
}

const styles = StyleSheet.create({
  container: { flex: 1, backgroundColor: '#f5f5f5' },
  content: { padding: 24, gap: 12 },
  header: { alignItems: 'center', marginBottom: 16, gap: 4 },
  avatar: {
    width: 80,
    height: 80,
    borderRadius: 40,
    backgroundColor: '#2563eb',
    alignItems: 'center',
    justifyContent: 'center',
    marginBottom: 8,
  },
  avatarText: { color: '#fff', fontSize: 28, fontWeight: '600' },
  email: { fontSize: 16, color: '#1f2937', fontWeight: '500' },
  role: { fontSize: 14, color: '#6b7280' },
  sectionTitle: {
    fontSize: 12,
    fontWeight: '600',
    color: '#6b7280',
    textTransform: 'uppercase',
    marginTop: 16,
    marginBottom: 4,
  },
  success: { backgroundColor: '#ecfdf5', padding: 12, borderRadius: 8 },
  successText: { color: '#047857', fontSize: 14 },
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
  primaryButton: {
    backgroundColor: '#2563eb',
    borderRadius: 8,
    padding: 14,
    alignItems: 'center',
    marginTop: 8,
  },
  primaryButtonDisabled: { backgroundColor: '#93c5fd' },
  primaryButtonText: { color: '#fff', fontSize: 16, fontWeight: '600' },
  row: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    backgroundColor: '#fff',
    paddingVertical: 14,
    paddingHorizontal: 16,
    borderRadius: 8,
    borderWidth: 1,
    borderColor: '#e5e7eb',
  },
  rowText: { fontSize: 16, color: '#1f2937' },
  rowChevron: { fontSize: 20, color: '#9ca3af' },
  signOutButton: {
    marginTop: 24,
    padding: 14,
    alignItems: 'center',
    borderRadius: 8,
    borderWidth: 1,
    borderColor: '#fecaca',
  },
  signOutText: { color: '#b91c1c', fontSize: 16, fontWeight: '600' },
});
