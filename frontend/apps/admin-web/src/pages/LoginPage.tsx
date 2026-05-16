import { FormEvent, useState } from 'react';
import { useNavigate } from 'react-router-dom';

import { useAdminAuth } from '../auth/AdminAuthContext';

export interface LoginResponse {
  access_token: string;
}

export interface LoginPageProps {
  /** Override for tests; default issues real fetch against /api/v1/auth/login. */
  loginFn?: (creds: { email: string; password: string }) => Promise<LoginResponse>;
}

const defaultLoginFn = async (creds: { email: string; password: string }) => {
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
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const onSubmit = async (e: FormEvent) => {
    e.preventDefault();
    setError(null);
    setBusy(true);
    try {
      const { access_token } = await loginFn({ email, password });
      auth.setToken(access_token);
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
        <input type="email" required value={email} onChange={(e) => setEmail(e.target.value)} />
      </label>
      <label>
        Password
        <input type="password" required value={password} onChange={(e) => setPassword(e.target.value)} />
      </label>
      {error && <p role="alert">{error}</p>}
      <button type="submit" disabled={busy}>
        {busy ? 'Signing in…' : 'Sign in'}
      </button>
    </form>
  );
}
