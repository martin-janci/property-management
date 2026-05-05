/**
 * AuthFlow (UC-14).
 *
 * Lightweight state switcher that lets the unauthenticated user move between
 * Login, Register and ForgotPassword without pulling in react-navigation.
 * Adopt react-navigation later if richer routing is needed.
 */

import { useState } from 'react';
import { ForgotPasswordScreen } from './ForgotPasswordScreen';
import { LoginScreen } from './LoginScreen';
import { RegisterScreen } from './RegisterScreen';

type Step = 'login' | 'register' | 'forgot';

export function AuthFlow() {
  const [step, setStep] = useState<Step>('login');

  if (step === 'register') {
    return <RegisterScreen onSignInPress={() => setStep('login')} />;
  }
  if (step === 'forgot') {
    return <ForgotPasswordScreen onSignInPress={() => setStep('login')} />;
  }
  return (
    <LoginScreen
      onRegisterPress={() => setStep('register')}
      onForgotPasswordPress={() => setStep('forgot')}
    />
  );
}
