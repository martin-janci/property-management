import { afterEach, describe, expect, it } from 'vitest';

import { sessionTokenStore } from './tokenStore';

afterEach(() => sessionStorage.clear());

describe('sessionTokenStore', () => {
  it('returns null when no token stored', () => {
    expect(sessionTokenStore.get()).toBeNull();
  });
  it('stores and retrieves token', () => {
    sessionTokenStore.set('abc');
    expect(sessionTokenStore.get()).toBe('abc');
  });
  it('clears token', () => {
    sessionTokenStore.set('abc');
    sessionTokenStore.clear();
    expect(sessionTokenStore.get()).toBeNull();
  });
});
