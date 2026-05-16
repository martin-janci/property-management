/**
 * sessionStorage-backed access-token store for admin-web.
 *
 * `sessionStorage` (not `localStorage`) is intentional: closing the tab
 * logs the admin out. Namespaced key prevents collisions with other apps
 * that happen to share an origin.
 */

export interface TokenStore {
  get(): string | null;
  set(token: string): void;
  clear(): void;
}

const STORAGE_KEY = 'ppt.admin.access_token';

export const sessionTokenStore: TokenStore = {
  get: () => sessionStorage.getItem(STORAGE_KEY),
  set: (token) => sessionStorage.setItem(STORAGE_KEY, token),
  clear: () => sessionStorage.removeItem(STORAGE_KEY),
};
