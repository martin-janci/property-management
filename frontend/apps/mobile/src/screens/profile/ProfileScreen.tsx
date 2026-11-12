/**
 * ProfileScreen (UC-14.6).
 *
 * Authenticated user views and edits their profile, manages biometric login
 * and signs out. Edits are persisted server-side via `PATCH /api/v1/auth/me`
 * (the api-server `updateMe` endpoint, body `{ displayName }`); on success the
 * cached session user is updated through `AuthContext.updateUser`, which keeps
 * both the in-memory context and the SecureStore `ppt_user` key in sync so the
 * change survives a cold start / biometric unlock.
 */

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
import { useAuth } from '../../contexts/AuthContext';
import { useApiMutation } from '../../hooks/useApi';
import { colors } from '../shared/screenStyles';

interface ProfileScreenProps {
  onChangePasswordPress?: () => void;
  onTwoFactorPress?: () => void;
}

/**
 * Body accepted by `PATCH /api/v1/auth/me`. The api-server stores a single
 * `displayName`; the app edits first/last separately, so they are joined on
 * save (and split back into first/last on login via `buildSessionUser`).
 */
interface UpdateProfileBody {
  displayName: string;
}

export function ProfileScreen({ onChangePasswordPress, onTwoFactorPress }: ProfileScreenProps) {
  const { t } = useTranslation();
  const {
    user,
    logout,
    updateUser,
    biometricAvailable,
    biometricEnabled,
    enableBiometric,
    disableBiometric,
  } = useAuth();

  const [firstName, setFirstName] = useState(user?.firstName ?? '');
  const [lastName, setLastName] = useState(user?.lastName ?? '');
  const [savedAt, setSavedAt] = useState<number>();

  const updateProfile = useApiMutation<unknown, UpdateProfileBody>('/api/v1/auth/me', 'PATCH');

  const handleSave = () => {
    if (!user || updateProfile.isPending) return;
    const trimmedFirst = firstName.trim();
    const trimmedLast = lastName.trim();
    if (!trimmedFirst || !trimmedLast) {
      Alert.alert(
        t('common.error', 'Error'),
        t('auth.nameRequired', 'First and last name are required.')
      );
      return;
    }
    // Clear any stale success notice while the request is in flight.
    setSavedAt(undefined);
    updateProfile.mutate(
      { displayName: `${trimmedFirst} ${trimmedLast}` },
      {
        onSuccess: async () => {
          // Persist the confirmed values into the cached session so the rest of
          // the app (and the next cold start) reflects them.
          await updateUser({ firstName: trimmedFirst, lastName: trimmedLast });
          setSavedAt(Date.now());
        },
        onError: (error) => {
          Alert.alert(
            t('common.error', 'Error'),
            error instanceof Error
              ? error.message
              : t('auth.saveProfileFailed', 'Could not save profile')
          );
        },
      }
    );
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
          style={[styles.primaryButton, updateProfile.isPending && styles.primaryButtonDisabled]}
          onPress={handleSave}
          disabled={updateProfile.isPending}
        >
          <Text style={styles.primaryButtonText}>
            {updateProfile.isPending
              ? t('common.saving', 'Saving…')
              : t('common.saveChanges', 'Save changes')}
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
  container: { flex: 1, backgroundColor: colors.background },
  content: { padding: 24, gap: 12 },
  header: { alignItems: 'center', marginBottom: 16, gap: 4 },
  avatar: {
    width: 80,
    height: 80,
    borderRadius: 40,
    backgroundColor: colors.accent,
    alignItems: 'center',
    justifyContent: 'center',
    marginBottom: 8,
  },
  avatarText: { color: colors.surface, fontSize: 28, fontWeight: '600' },
  email: { fontSize: 16, color: colors.text, fontWeight: '500' },
  role: { fontSize: 14, color: colors.textMuted },
  sectionTitle: {
    fontSize: 12,
    fontWeight: '600',
    color: colors.textMuted,
    textTransform: 'uppercase',
    marginTop: 16,
    marginBottom: 4,
  },
  success: { backgroundColor: colors.successBg, padding: 12, borderRadius: 8 },
  successText: { color: colors.successDark, fontSize: 14 },
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
  primaryButton: {
    backgroundColor: colors.accent,
    borderRadius: 8,
    padding: 14,
    alignItems: 'center',
    marginTop: 8,
  },
  primaryButtonDisabled: { backgroundColor: colors.accentDisabled },
  primaryButtonText: { color: colors.surface, fontSize: 16, fontWeight: '600' },
  row: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
    backgroundColor: colors.surface,
    paddingVertical: 14,
    paddingHorizontal: 16,
    borderRadius: 8,
    borderWidth: 1,
    borderColor: colors.border,
  },
  rowText: { fontSize: 16, color: colors.text },
  rowChevron: { fontSize: 20, color: colors.textSubtle },
  signOutButton: {
    marginTop: 24,
    padding: 14,
    alignItems: 'center',
    borderRadius: 8,
    borderWidth: 1,
    borderColor: colors.dangerBg,
  },
  signOutText: { color: colors.dangerDark, fontSize: 16, fontWeight: '600' },
});
