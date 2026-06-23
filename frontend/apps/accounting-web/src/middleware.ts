import createMiddleware from 'next-intl/middleware';
import { routing } from './i18n/routing';

// Plain next-intl locale routing. Unlike reality-web there is no per-tenant
// kill-switch here — the public marketing/signup surface is single-tenant.
export default createMiddleware(routing);

export const config = {
  // Run on everything except API routes, Next internals, and static files.
  matcher: ['/((?!api|_next|_vercel|.*\\..*).*)'],
};
