import { type FormEvent, useState } from 'react';
import { useNavigate } from 'react-router-dom';

import { useAdminAuth } from '../auth/AdminAuthContext';
import { trackSignupLoggedIn } from '../features/onboarding/analytics';

export interface LoginResponse {
  /** Empty string when `mfaRequired: true` (server defers the token until MFA passes). */
  accessToken: string;
  mfaRequired?: boolean;
}

export interface LoginPageProps {
  /** Override for tests; default issues real fetch against /api/v1/auth/login. */
  loginFn?: (creds: {
    email: string;
    password: string;
    twoFactorCode?: string;
  }) => Promise<LoginResponse>;
}

const defaultLoginFn = async (creds: {
  email: string;
  password: string;
  twoFactorCode?: string;
}) => {
  const resp = await fetch('/api/v1/auth/login', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'include',
    body: JSON.stringify(creds),
  });
  if (!resp.ok) throw new Error(`login failed: ${resp.status}`);
  return (await resp.json()) as LoginResponse;
};

export function LoginPage({ loginFn = defaultLoginFn }: LoginPageProps) {
  const auth = useAdminAuth();
  const navigate = useNavigate();
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  // `mfaRequired` flips after the first POST returns `mfaRequired: true`.
  // The form then renders the TOTP code input and a second submit replays
  // the login with `twoFactorCode` populated. Until the server returns a
  // non-empty accessToken we never call `setToken` — otherwise we'd land
  // on the dashboard with an empty Bearer header and every API call would
  // 401-loop.
  const [mfaRequired, setMfaRequired] = useState(false);
  const [code, setCode] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    setBusy(true);
    try {
      const resp = await loginFn({
        email,
        password,
        twoFactorCode: mfaRequired ? code : undefined,
      });
      if (resp.mfaRequired || !resp.accessToken) {
        setMfaRequired(true);
        setCode('');
        if (!resp.accessToken && !resp.mfaRequired) {
          // Server returned 200 but no token and no MFA flag — refuse to navigate.
          setError('Login response missing access token');
        }
        return;
      }
      auth.setToken(resp.accessToken);
      // Signup-funnel: first login is the leg between email verification and
      // the onboarding tour (issue #2530). Fired only after a real,
      // non-MFA-deferred token is issued.
      trackSignupLoggedIn('email_password');
      navigate('/', { replace: true });
    } catch (err) {
      setError(err instanceof Error ? err.message : 'login failed');
    } finally {
      setBusy(false);
    }
  };

  return (
    <form onSubmit={onSubmit} aria-label="admin login">
      <h1>PPT Admin</h1>
      <label>
        Email
        <input
          type="email"
          required
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          disabled={mfaRequired}
        />
      </label>
      <label>
        Password
        <input
          type="password"
          required
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          disabled={mfaRequired}
        />
      </label>
      {mfaRequired && (
        <label>
          Authenticator code
          <input
            type="text"
            inputMode="numeric"
            autoComplete="one-time-code"
            pattern="\d{6}"
            required
            value={code}
            onChange={(e) => setCode(e.target.value)}
          />
        </label>
      )}
      {error && <p role="alert">{error}</p>}
      <button type="submit" disabled={busy}>
        {busy ? 'Signing in…' : mfaRequired ? 'Verify code' : 'Sign in'}
      </button>
    </form>
  );
}
