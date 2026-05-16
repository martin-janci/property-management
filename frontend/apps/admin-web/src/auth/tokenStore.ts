import type { TokenStore } from '../api/client';

const STORAGE_KEY = 'ppt.admin.access_token';

export const sessionTokenStore: TokenStore = {
  get: () => sessionStorage.getItem(STORAGE_KEY),
  set: (token) => sessionStorage.setItem(STORAGE_KEY, token),
  clear: () => sessionStorage.removeItem(STORAGE_KEY),
};
