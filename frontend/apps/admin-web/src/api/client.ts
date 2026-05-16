import axios, { AxiosError, AxiosInstance } from 'axios';

export interface TokenStore {
  get(): string | null;
  set(token: string): void;
  clear(): void;
}

export interface ApiClientOptions {
  baseURL: string;
  tokenStore: TokenStore;
  onUnauthenticated: () => void;
  onMfaRequired?: () => Promise<void>;
}

export interface AdminApiClient {
  axios: AxiosInstance;
  handle401(error: unknown): Promise<never>;
}

export function createApiClient(opts: ApiClientOptions): AdminApiClient {
  const instance = axios.create({
    baseURL: opts.baseURL,
    withCredentials: true,
  });

  instance.interceptors.request.use((config) => {
    const token = opts.tokenStore.get();
    if (token) {
      config.headers.Authorization = `Bearer ${token}`;
    }
    return config;
  });

  const handle401 = async (error: unknown): Promise<never> => {
    const err = error as AxiosError<{ error?: string }>;
    if (err?.response?.status === 401 && err.response.data?.error === 'mfa_required' && opts.onMfaRequired) {
      await opts.onMfaRequired();
      // After MFA, the caller is expected to retry; we still reject so caller can decide.
    }
    opts.tokenStore.clear();
    opts.onUnauthenticated();
    throw err;
  };

  instance.interceptors.response.use(
    (resp) => resp,
    (error: AxiosError) => {
      if (error.response?.status === 401) {
        return handle401(error);
      }
      throw error;
    },
  );

  return { axios: instance, handle401 };
}
