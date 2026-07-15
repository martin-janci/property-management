/**
 * Vitest test setup file for Reality Web
 *
 * Configures testing environment:
 * - Jest DOM matchers for DOM assertions
 * - Cleanup after each test
 * - Mock implementations for browser APIs
 * - next-intl mock for translation testing
 */

import '@testing-library/jest-dom';
import { cleanup } from '@testing-library/react';
import { afterEach, vi } from 'vitest';

// Cleanup after each test to prevent memory leaks
afterEach(() => {
  cleanup();
});

// Mock next-intl useTranslations hook
vi.mock('next-intl', () => ({
  useTranslations: () => (key: string) => key,
  useLocale: () => 'en',
  useMessages: () => ({}),
  NextIntlClientProvider: ({ children }: { children: React.ReactNode }) => children,
}));

// Mock next-intl/navigation (used by src/i18n/routing.ts `createNavigation`).
// Returns a plain <a> Link so components using the locale-aware Link render in
// jsdom without a NextIntlClientProvider.
vi.mock('next-intl/navigation', () => ({
  createNavigation: () => ({
    Link: ({ children, href }: { children: React.ReactNode; href: string }) => (
      <a href={href}>{children}</a>
    ),
    redirect: vi.fn(),
    usePathname: () => '/',
    useRouter: () => ({ push: vi.fn(), replace: vi.fn() }),
    getPathname: () => '/',
  }),
}));

// Mock next/navigation
vi.mock('next/navigation', () => ({
  useRouter: () => ({
    push: vi.fn(),
    replace: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
    refresh: vi.fn(),
    prefetch: vi.fn(),
  }),
  usePathname: () => '/en',
  useSearchParams: () => new URLSearchParams(),
  useParams: () => ({ locale: 'en' }),
}));

// Mock next/link
vi.mock('next/link', () => ({
  default: ({ children, href }: { children: React.ReactNode; href: string }) => {
    return <a href={href}>{children}</a>;
  },
}));

// Mock next/image
vi.mock('next/image', () => ({
  default: ({ src, alt, ...props }: { src: string; alt: string }) => {
    return <img src={src} alt={alt || ''} {...props} />;
  },
}));

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
