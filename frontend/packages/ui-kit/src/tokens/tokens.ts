/**
 * Design tokens for @ppt/ui-kit — tokens/tokens.ts
 *
 * TypeScript const export of design tokens for consumers that cannot use CSS variables
 * (e.g. React Native, canvas, chart libraries, server-side PDF generation).
 *
 * Ported 1:1 from guest-registration-v2-design-system/colors_and_type.css.
 * For CSS usage, import tokens.css instead and use CSS custom properties.
 */

export const brand = {
  50:  '#eff6ff',
  100: '#dbeafe',
  200: '#bfdbfe',
  300: '#93c5fd',
  400: '#60a5fa',
  500: '#3b82f6',
  600: '#2563eb',
  700: '#1d4ed8',
  800: '#1e40af',
  900: '#1e3a8a',
} as const;

export const accent = {
  default:  brand[600],
  hover:    brand[700],
  softBg:   '#eff6ff',
  ink:      '#ffffff',
} as const;

export const bg = {
  app:      '#f9fafb',
  appSoft:  '#f5f5f5',
  surface:  '#ffffff',
  elevated: '#ffffff',
  overlay:  'rgba(17, 24, 39, 0.5)',
  input:    '#ffffff',
  subtle:   '#f3f4f6',
  hover:    '#f3f4f6',
} as const;

export const fg = {
  primary:   '#111827',
  secondary: '#374151',
  muted:     '#6b7280',
  subtle:    '#9ca3af',
  onAccent:  '#ffffff',
  link:      '#2563eb',
} as const;

export const border = {
  subtle:  '#f3f4f6',
  default: '#e5e7eb',
  strong:  '#d1d5db',
  focus:   '#2563eb',
} as const;

export const success = {
  50:  '#ecfdf5',
  100: '#d1fae5',
  500: '#10b981',
  600: '#059669',
  800: '#065f46',
} as const;

export const warning = {
  50:  '#fffbeb',
  100: '#fef3c7',
  500: '#f59e0b',
  800: '#92400e',
} as const;

export const danger = {
  50:  '#fef2f2',
  100: '#fee2e2',
  500: '#ef4444',
  600: '#dc2626',
  800: '#991b1b',
} as const;

export const featured = {
  500: '#fbbf24',
  ink: '#78350f',
} as const;

export const shadow = {
  flat:      'none',
  card:      '0 1px 3px rgba(0, 0, 0, 0.1)',
  cardHover: '0 4px 12px rgba(0, 0, 0, 0.15)',
  popover:   '0 4px 12px rgba(0, 0, 0, 0.1)',
  modal:     '0 10px 40px rgba(0, 0, 0, 0.15)',
  hero:      '0 10px 40px rgba(0, 0, 0, 0.15)',
} as const;

export const duration = {
  fast: '120ms',
  base: '200ms',
  slow: '320ms',
} as const;

export const easing = {
  standard: 'cubic-bezier(0.4, 0, 0.2, 1)',
  decel:    'cubic-bezier(0, 0, 0.2, 1)',
  accel:    'cubic-bezier(0.4, 0, 1, 1)',
} as const;

export const radius = {
  sm:     '4px',
  md:     '8px',
  lg:     '12px',
  xl:     '16px',
  '2xl':  '20px',
  pill:   '9999px',
  circle: '50%',
} as const;

export const fontSize = {
  display:  'clamp(2.25rem, 2vw + 1.75rem, 3rem)',
  h1:       '1.5rem',
  h2:       '1.125rem',
  h3:       '1rem',
  price:    '1.25rem',
  bodyLg:   '1rem',
  body:     '0.875rem',
  bodySm:   '0.8125rem',
  caption:  '0.75rem',
  tiny:     '0.6875rem',
} as const;

export const fontWeight = {
  regular: 400,
  medium:  500,
  semi:    600,
  bold:    700,
} as const;

/** Grouped export for consumers that want a single import */
export const tokens = {
  brand,
  accent,
  bg,
  fg,
  border,
  success,
  warning,
  danger,
  featured,
  shadow,
  duration,
  easing,
  radius,
  fontSize,
  fontWeight,
} as const;

export type Tokens = typeof tokens;
