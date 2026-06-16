/// <reference types="vitest/globals" />
/**
 * Vitest test setup file (Epic 80, Story 80.3)
 *
 * Configures testing environment:
 * - Jest DOM matchers for DOM assertions
 * - Axe accessibility matchers for a11y testing
 * - Cleanup after each test
 * - Mock implementations for browser APIs
 * - i18n mock for translation testing
 */

import '@testing-library/jest-dom';
import { cleanup } from '@testing-library/react';
import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import * as matchers from 'vitest-axe/matchers';
import en from '../../messages/en.json';

// Import type declarations for vitest-axe matchers
import './vitest-axe.d';

// Extend Vitest expect with axe accessibility matchers
expect.extend(matchers);

// Initialize i18n for tests with the full English translation bundle so
// components that read real keys (auth pages, status indicators, …) render
// their English copy under test instead of bare keys.
i18n.use(initReactI18next).init({
  lng: 'en',
  fallbackLng: 'en',
  resources: {
    en: { translation: en },
  },
  interpolation: {
    escapeValue: false,
  },
});

// jsdom under vitest 4 exposes `window.sessionStorage` but not
// `window.localStorage`, so token-storage helpers in `@ppt/shared`
// (exercised by `shared-auth.test.ts`) read/write a missing API and fall
// back to their no-op SSR path. Provide a minimal in-memory localStorage
// shim when the environment lacks one.
if (typeof window !== 'undefined' && !window.localStorage) {
  const store = new Map<string, string>();
  const localStorageShim: Storage = {
    getItem: (key: string) => (store.has(key) ? (store.get(key) as string) : null),
    setItem: (key: string, value: string) => {
      store.set(key, String(value));
    },
    removeItem: (key: string) => {
      store.delete(key);
    },
    clear: () => {
      store.clear();
    },
    key: (index: number) => Array.from(store.keys())[index] ?? null,
    get length() {
      return store.size;
    },
  };
  Object.defineProperty(window, 'localStorage', {
    value: localStorageShim,
    configurable: true,
    writable: true,
  });
}

// Cleanup after each test to prevent memory leaks.
// TEST-203: also clear localStorage/sessionStorage so token-storage tests
// (`shared-auth.test.ts`) can't leak state into the next test.
afterEach(() => {
  cleanup();
  try {
    window.localStorage.clear();
    window.sessionStorage.clear();
  } catch {
    // jsdom may not have storage in some environments; ignore.
  }
});

// Mock window.matchMedia for components using media queries
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  }),
});

// Mock ResizeObserver for components using it
(globalThis as Record<string, unknown>).ResizeObserver = class ResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
};

// Mock IntersectionObserver for lazy loading components
(globalThis as Record<string, unknown>).IntersectionObserver = class IntersectionObserver {
  root = null;
  rootMargin = '';
  thresholds: number[] = [];

  observe() {}
  unobserve() {}
  disconnect() {}
  takeRecords(): IntersectionObserverEntry[] {
    return [];
  }
};
