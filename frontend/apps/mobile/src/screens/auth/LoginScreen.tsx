import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  ActivityIndicator,
  Alert,
  KeyboardAvoidingView,
  Platform,
  Pressable,
  StyleSheet,
  Text,
  TextInput,
  View,
} from 'react-native';
import { useAuth } from '../../contexts/AuthContext';
import { colors } from '../shared/screenStyles';

interface LoginScreenProps {
  onRegisterPress?: () => void;
  onForgotPasswordPress?: () => void;
}

export function LoginScreen({ onRegisterPress, onForgotPasswordPress }: LoginScreenProps = {}) {
  const { t } = useTranslation();
  const { login, authenticateWithBiometric, biometricAvailable, biometricEnabled, isLoading } =
    useAuth();
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [errors, setErrors] = useState<{ email?: string; password?: string }>({});

  const validateForm = (): boolean => {
    const newErrors: { email?: string; password?: string } = {};

    if (!email.trim()) {
      newErrors.email = t('auth.emailRequired');
    } else if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) {
      newErrors.email = t('auth.invalidEmailFormat');
    }

    if (!password) {
      newErrors.password = t('auth.passwordRequired');
    } else if (password.length < 8) {
      newErrors.password = t('auth.passwordMinLength');
    }

    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleLogin = async () => {
    if (!validateForm()) {
      return;
    }

    try {
      await login(email, password);
    } catch (error) {
      Alert.alert(
        t('auth.loginFailed'),
        error instanceof Error ? error.message : t('auth.checkCredentials')
      );
    }
  };

  const handleBiometricLogin = async () => {
    try {
      const success = await authenticateWithBiometric();
      if (!success) {
        Alert.alert(t('auth.authenticationFailed'), t('auth.biometricNotSuccessful'));
      }
    } catch (_error) {
      Alert.alert(t('common.error'), t('auth.biometricFailed'));
    }
  };

  return (
    <KeyboardAvoidingView
      style={styles.container}
      behavior={Platform.OS === 'ios' ? 'padding' : 'height'}
    >
      <View style={styles.content}>
        <View style={styles.header}>
          <Text style={styles.logo}>PPT</Text>
          <Text style={styles.title}>{t('auth.propertyManagement')}</Text>
          <Text style={styles.subtitle}>{t('auth.signInPrompt')}</Text>
        </View>

        <View style={styles.form}>
          <View style={styles.inputGroup}>
            <Text style={styles.label}>{t('auth.email')}</Text>
            <TextInput
              style={[styles.input, errors.email ? styles.inputError : undefined]}
              placeholder={t('auth.emailPlaceholder')}
              value={email}
              onChangeText={setEmail}
              keyboardType="email-address"
              autoCapitalize="none"
              autoComplete="email"
              editable={!isLoading}
            />
            {errors.email && <Text style={styles.errorText}>{errors.email}</Text>}
          </View>

          <View style={styles.inputGroup}>
            <Text style={styles.label}>{t('auth.password')}</Text>
            <View style={styles.passwordContainer}>
              <TextInput
                style={[
                  styles.input,
                  styles.passwordInput,
                  errors.password ? styles.inputError : undefined,
                ]}
                placeholder={t('auth.passwordPlaceholder')}
                value={password}
                onChangeText={setPassword}
                secureTextEntry={!showPassword}
                autoComplete="password"
                editable={!isLoading}
              />
              <Pressable
                style={styles.showPasswordButton}
                onPress={() => setShowPassword(!showPassword)}
              >
                <Text style={styles.showPasswordText}>
                  {showPassword ? t('auth.hide') : t('auth.show')}
                </Text>
              </Pressable>
            </View>
            {errors.password && <Text style={styles.errorText}>{errors.password}</Text>}
          </View>

          <Pressable style={styles.forgotPassword} onPress={onForgotPasswordPress}>
            <Text style={styles.forgotPasswordText}>{t('auth.forgotPassword')}</Text>
          </Pressable>

          <Pressable
            style={[styles.loginButton, isLoading && styles.loginButtonDisabled]}
            onPress={handleLogin}
            disabled={isLoading}
          >
            {isLoading ? (
              <ActivityIndicator color={colors.surface} />
            ) : (
              <Text style={styles.loginButtonText}>{t('auth.signIn')}</Text>
            )}
          </Pressable>

          {biometricAvailable && biometricEnabled && (
            <Pressable
              style={styles.biometricButton}
              onPress={handleBiometricLogin}
              disabled={isLoading}
            >
              <Text style={styles.biometricIcon}>{Platform.OS === 'ios' ? '👤' : '🔒'}</Text>
              <Text style={styles.biometricText}>
                {t('auth.signInWith', {
                  provider: Platform.OS === 'ios' ? t('auth.faceIdTouchId') : t('auth.biometrics'),
                })}
              </Text>
            </Pressable>
          )}
        </View>

        <View style={styles.footer}>
          <Text style={styles.footerText}>{t('auth.dontHaveAccount')}</Text>
          <Pressable onPress={onRegisterPress}>
            <Text style={styles.registerLink}>
              {onRegisterPress ? t('auth.signUp', 'Sign up') : t('auth.contactManager')}
            </Text>
          </Pressable>
        </View>
      </View>
    </KeyboardAvoidingView>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: colors.background,
  },
  content: {
    flex: 1,
    padding: 24,
    justifyContent: 'center',
  },
  header: {
    alignItems: 'center',
    marginBottom: 40,
  },
  logo: {
    fontSize: 48,
    fontWeight: 'bold',
    color: colors.accent,
    marginBottom: 8,
  },
  title: {
    fontSize: 24,
    fontWeight: '600',
    color: colors.text,
    marginBottom: 4,
  },
  subtitle: {
    fontSize: 16,
    color: colors.textMuted,
  },
  form: {
    gap: 16,
  },
  inputGroup: {
    gap: 6,
  },
  label: {
    fontSize: 14,
    fontWeight: '500',
    color: colors.textSecondary,
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
  },
  passwordContainer: {
    position: 'relative',
  },
  passwordInput: {
    paddingRight: 60,
  },
  showPasswordButton: {
    position: 'absolute',
    right: 12,
    top: 12,
    padding: 4,
  },
  showPasswordText: {
    color: colors.accent,
    fontSize: 14,
    fontWeight: '500',
  },
  forgotPassword: {
    alignSelf: 'flex-end',
  },
  forgotPasswordText: {
    color: colors.accent,
    fontSize: 14,
  },
  loginButton: {
    backgroundColor: colors.accent,
    borderRadius: 8,
    padding: 16,
    alignItems: 'center',
    marginTop: 8,
  },
  loginButtonDisabled: {
    backgroundColor: colors.accentDisabled,
  },
  loginButtonText: {
    color: colors.surface,
    fontSize: 16,
    fontWeight: '600',
  },
  biometricButton: {
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'center',
    padding: 16,
    gap: 8,
    backgroundColor: colors.surface,
    borderRadius: 8,
    borderWidth: 1,
    borderColor: colors.borderStrong,
  },
  biometricIcon: {
    fontSize: 20,
  },
  biometricText: {
    fontSize: 16,
    color: colors.textSecondary,
  },
  footer: {
    flexDirection: 'row',
    justifyContent: 'center',
    alignItems: 'center',
    gap: 4,
    marginTop: 32,
  },
  footerText: {
    color: colors.textMuted,
    fontSize: 14,
  },
  registerLink: {
    color: colors.accent,
    fontSize: 14,
    fontWeight: '500',
  },
});
