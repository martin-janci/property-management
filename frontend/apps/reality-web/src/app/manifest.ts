import type { MetadataRoute } from 'next';

/**
 * Web App Manifest (Next 14 file convention).
 *
 * Lightweight start: name + colours + icons. No service worker yet — that's
 * a bigger conversation (offline strategy, push notifications, install
 * prompt UX). This manifest alone unlocks Add-to-Home-Screen on Android +
 * a "Reality Portal" tile name instead of "wt-sutherland.dev.rlt.sk" when
 * a user pins the site.
 *
 * Icons: a single SVG with `sizes: 'any'` covers Chrome/Android A2HS in
 * 2023+ browsers — they raster the SVG at whatever size the launcher
 * needs. We deliberately don't ship 192px and 512px PNGs at this stage:
 *   - The brand mark is already a tiny SVG (~200 bytes); committing two
 *     pre-rasterized PNG copies would be ~10× larger for the same visual.
 *   - Maskable variants matter mostly for Android adaptive icons; the
 *     `purpose: 'maskable any'` hint below tells launchers they may
 *     safely apply the OS mask to the existing SVG.
 *   - iOS still ignores manifest icons entirely (it uses
 *     `<link rel="apple-touch-icon">` — to be added when iOS install
 *     experience is on the roadmap).
 * When a brand redesign produces a more complex mark that doesn't
 * rasterize cleanly, swap in `app/icon.png` + `app/icon-512.png` (Next
 * file convention) and reference them here.
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
    // Two entries with the same SVG src: one as the standard launcher
    // icon, one as the maskable variant. Next's MetadataRoute.Manifest
    // type only accepts a single `purpose` per icon entry, so we split
    // them rather than space-joining.
    icons: [
      {
        src: '/icon.svg',
        sizes: 'any',
        type: 'image/svg+xml',
        purpose: 'any',
      },
      {
        src: '/icon.svg',
        sizes: 'any',
        type: 'image/svg+xml',
        purpose: 'maskable',
      },
    ],
  };
}
