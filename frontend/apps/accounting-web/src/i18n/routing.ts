import { createNavigation } from 'next-intl/navigation';
import { defineRouting } from 'next-intl/routing';
import { defaultLocale, locales } from './config';

export const routing = defineRouting({
  locales,
  defaultLocale,
  localePrefix: 'as-needed',
});

// Locale-aware navigation utilities (Link/redirect/hooks honour the prefix).
export const { Link, redirect, usePathname, useRouter, getPathname } = createNavigation(routing);
