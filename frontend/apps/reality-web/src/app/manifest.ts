import type { MetadataRoute } from 'next';

/**
 * Web App Manifest (Next 14 file convention).
 *
 * Lightweight start: name + colours + icons. No service worker yet — that's
 * a bigger conversation (offline strategy, push notifications, install
 * prompt UX). This manifest alone unlocks Add-to-Home-Screen on Android +
 * a "Reality Portal" tile name instead of "wt-sutherland.dev.rlt.sk" when
 * a user pins the site.
 */
export default function manifest(): MetadataRoute.Manifest {
  return {
    name: 'Reality Portal',
    short_name: 'Reality',
    description: 'Find your perfect property across Slovakia, Czech Republic, and beyond.',
    start_url: '/',
    display: 'standalone',
    background_color: '#ffffff',
    theme_color: '#1f4cd1',
    icons: [
      {
        src: '/icon.svg',
        sizes: 'any',
        type: 'image/svg+xml',
        purpose: 'any',
      },
    ],
  };
}
