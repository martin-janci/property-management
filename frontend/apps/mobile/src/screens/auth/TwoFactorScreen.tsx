/**
 * TwoFactorScreen (UC-14.10).
 *
 * UI scaffold for setting up TOTP-based two-factor authentication. The
 * backend endpoints for enrollment aren't exposed through our mobile auth
 * layer yet, so this screen wires up local state and renders the setup flow.
 */

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  KeyboardAvoidingView,
  Platform,
  Pressable,
  StyleSheet,
  Text,
  TextInput,
  View,
} from 'react-native';

type Step = 'idle' | 'verify' | 'enabled';

interface TwoFactorScreenProps {
  onDonePress?: () => void;
}

export function TwoFactorScreen({ onDonePress }: TwoFactorScreenProps) {
  const { t } = useTranslation();
  const [step, setStep] = useState<Step>('idle');
  const [code, setCode] = useState('');
  const [error, setError] = useState<string>();

  const handleStart = () => {
    setStep('verify');
    setCode('');
    setError(undefined);
  };

  const handleVerify = () => {
    if (!/^\d{6}$/.test(code.trim())) {
      setError(t('auth.enterSixDigitCode', 'Enter the 6-digit code from your authenticator app.'));
      return;
    }
    setError(undefined);
    setStep('enabled');
  };

  return (
    <KeyboardAvoidingView
      style={styles.container}
      behavior={Platform.OS === 'ios' ? 'padding' : 'height'}
    >
      <View style={styles.content}>
        <Text style={styles.title}>{t('auth.twoFactor', 'Two-factor authentication')}</Text>
        <Text style={styles.subtitle}>
          {t(
            'auth.twoFactorSubtitle',
            'Add an extra layer of security by requiring a code from your authenticator app.'
          )}
        </Text>

        {step === 'idle' && (
          <Pressable style={styles.primaryButton} onPress={handleStart}>
            <Text style={styles.primaryButtonText}>
              {t('auth.setUpTwoFactor', 'Set up two-factor authentication')}
            </Text>
          </Pressable>
        )}

        {step === 'verify' && (
          <View style={styles.form}>
            <Text style={styles.help}>
              {t(
                'auth.scanQrHelp',
                'Scan the QR code in your authenticator app, then enter the 6-digit code below.'
              )}
            </Text>
            <View style={styles.inputGroup}>
              <Text style={styles.label}>{t('auth.verificationCode', 'Verification code')}</Text>
              <TextInput
                style={[styles.input, error ? styles.inputError : undefined]}
                value={code}
                onChangeText={(text) => setCode(text.replace(/\D/g, '').slice(0, 6))}
                keyboardType="number-pad"
                autoComplete="one-time-code"
                maxLength={6}
              />
              {error && <Text style={styles.errorText}>{error}</Text>}
            </View>
            <Pressable style={styles.primaryButton} onPress={handleVerify}>
              <Text style={styles.primaryButtonText}>
                {t('auth.verifyAndEnable', 'Verify and enable')}
              </Text>
            </Pressable>
          </View>
        )}

        {step === 'enabled' && (
          <View style={styles.success}>
            <Text style={styles.successText}>
              {t('auth.twoFactorEnabled', 'Two-factor authentication is enabled.')}
            </Text>
            {onDonePress && (
              <Pressable style={styles.primaryButton} onPress={onDonePress}>
                <Text style={styles.primaryButtonText}>{t('common.done', 'Done')}</Text>
              </Pressable>
            )}
          </View>
        )}
      </View>
    </KeyboardAvoidingView>
  );
}

const styles = StyleSheet.create({
  container: { flex: 1, backgroundColor: '#f5f5f5' },
  content: { flex: 1, padding: 24, justifyContent: 'center' },
  title: {
    fontSize: 24,
    fontWeight: '600',
    color: '#1f2937',
    marginBottom: 8,
    textAlign: 'center',
  },
  subtitle: { fontSize: 16, color: '#6b7280', textAlign: 'center', marginBottom: 24 },
  form: { gap: 16 },
  help: { fontSize: 14, color: '#6b7280', textAlign: 'center' },
  inputGroup: { gap: 6 },
  label: { fontSize: 14, fontWeight: '500', color: '#374151' },
  input: {
    backgroundColor: '#fff',
    borderRadius: 8,
    borderWidth: 1,
    borderColor: '#d1d5db',
    padding: 12,
    fontSize: 20,
    textAlign: 'center',
    letterSpacing: 6,
  },
  inputError: { borderColor: '#ef4444' },
  errorText: { color: '#ef4444', fontSize: 12, textAlign: 'center' },
  primaryButton: {
    backgroundColor: '#2563eb',
    borderRadius: 8,
    padding: 16,
    alignItems: 'center',
    marginTop: 8,
  },
  primaryButtonText: { color: '#fff', fontSize: 16, fontWeight: '600' },
  success: { gap: 16, alignItems: 'center' },
  successText: {
    color: '#047857',
    fontSize: 16,
    backgroundColor: '#ecfdf5',
    paddingVertical: 12,
    paddingHorizontal: 16,
    borderRadius: 8,
    textAlign: 'center',
  },
});
