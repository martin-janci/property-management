/**
 * Tailwind CSS preset for @ppt/ui-kit — tokens/tailwind-preset.ts
 *
 * Extends Tailwind's theme with design-system tokens sourced from tokens.ts.
 * Add this to your tailwind.config.ts `presets` array to apply design tokens:
 *
 * @example
 * import { pptPreset } from '@ppt/ui-kit/src/tokens';
 * export default { presets: [pptPreset], content: [...] };
 *
 * Note: values reference CSS custom properties so they work with dark-mode
 * overrides in [data-theme="dark"] without Tailwind's dark variant.
 */

import { duration, fontSize, radius, shadow } from './tokens';

/** Tailwind preset that maps @ppt design tokens to Tailwind theme extensions. */
export const pptPreset = {
  theme: {
    extend: {
      colors: {
        brand: {
          50: 'var(--brand-50)',
          100: 'var(--brand-100)',
          200: 'var(--brand-200)',
          300: 'var(--brand-300)',
          400: 'var(--brand-400)',
          500: 'var(--brand-500)',
          600: 'var(--brand-600)',
          700: 'var(--brand-700)',
          800: 'var(--brand-800)',
          900: 'var(--brand-900)',
        },
        accent: 'var(--accent)',
        surface: 'var(--bg-surface)',
        app: 'var(--bg-app)',
        elevated: 'var(--bg-elevated)',
        subtle: 'var(--bg-subtle)',
        success: {
          50: 'var(--success-50)',
          100: 'var(--success-100)',
          500: 'var(--success-500)',
          600: 'var(--success-600)',
          800: 'var(--success-800)',
        },
        warning: {
          50: 'var(--warning-50)',
          100: 'var(--warning-100)',
          500: 'var(--warning-500)',
          800: 'var(--warning-800)',
        },
        danger: {
          50: 'var(--danger-50)',
          100: 'var(--danger-100)',
          500: 'var(--danger-500)',
          600: 'var(--danger-600)',
          800: 'var(--danger-800)',
        },
        featured: {
          500: 'var(--featured-500)',
          ink: 'var(--featured-ink)',
        },
      },
      fontSize: {
        display: [fontSize.display, { lineHeight: '1.1', fontWeight: '800' }],
        h1: [fontSize.h1, { lineHeight: '1.35', fontWeight: '700' }],
        h2: [fontSize.h2, { lineHeight: '1.35', fontWeight: '600' }],
        h3: [fontSize.h3, { lineHeight: '1.35', fontWeight: '600' }],
        price: [fontSize.price, { fontWeight: '700' }],
        'body-lg': [fontSize.bodyLg, {}],
        body: [fontSize.body, {}],
        'body-sm': [fontSize.bodySm, {}],
        caption: [fontSize.caption, {}],
        tiny: [fontSize.tiny, {}],
      },
      borderRadius: {
        sm: radius.sm,
        md: radius.md,
        lg: radius.lg,
        xl: radius.xl,
        '2xl': radius['2xl'],
        pill: radius.pill,
        circle: radius.circle,
      },
      boxShadow: {
        flat: shadow.flat,
        card: shadow.card,
        'card-hover': shadow.cardHover,
        popover: shadow.popover,
        modal: shadow.modal,
        hero: shadow.hero,
      },
      transitionDuration: {
        fast: duration.fast,
        base: duration.base,
        slow: duration.slow,
      },
    },
  },
} as const;

export default pptPreset;
