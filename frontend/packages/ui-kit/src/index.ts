/**
 * @ppt/ui-kit - Design system for Property Management apps.
 *
 * Shared components for ppt-web, reality-web, and mobile applications.
 * Import the tokens.css file in your app's entry point to enable CSS custom properties.
 */

// ============================================
// CSS Tokens (import in app entry point)
// ============================================
// import '@ppt/ui-kit/src/tokens.css';

// ============================================
// Version
// ============================================
export const VERSION = '0.2.356';

// ============================================
// Accessibility
// ============================================
export { SkipNavigation } from './SkipNavigation';
export type { SkipNavigationProps } from './SkipNavigation';
export { AccessibilityProvider, useAccessibilityContext } from './AccessibilityProvider';
export type { AccessibilityProviderProps } from './AccessibilityProvider';

// ============================================
// Core Components
// ============================================

// Button
export { Button } from './Button';
export type { ButtonProps, ButtonVariant, ButtonSize } from './Button';

// Input
export { Input } from './Input';
export type { InputProps, InputType, InputSize } from './Input';

// Modal
export { Modal } from './Modal';
export type { ModalProps, ModalSize } from './Modal';

// Card
export { Card } from './Card';
export type { CardProps, CardHeaderProps, CardContentProps, CardFooterProps } from './Card';

// Table
export { Table } from './Table';
export type {
  TableProps,
  TableHeadProps,
  TableBodyProps,
  TableRowProps,
  TableCellProps,
  TableHeaderCellProps,
} from './Table';

// Badge
export { Badge } from './Badge';
export type { BadgeProps, BadgeVariant, BadgeSize } from './Badge';

// Alert
export { Alert } from './Alert';
export type { AlertProps, AlertVariant } from './Alert';

// Spinner
export { Spinner, SpinnerOverlay } from './Spinner';
export type { SpinnerProps, SpinnerSize, SpinnerOverlayProps } from './Spinner';

// Pagination
export { Pagination } from './Pagination';
export type { PaginationProps } from './Pagination';

// ============================================
// Design Tokens (JavaScript)
// ============================================

/**
 * Color tokens for programmatic access.
 * For CSS usage, import tokens.css and use CSS custom properties.
 */
export const colors = {
  primary: '#3B82F6',
  primaryHover: '#2563EB',
  primaryActive: '#1D4ED8',
  primaryLight: '#DBEAFE',
  primaryDark: '#1E40AF',

  secondary: '#6B7280',
  secondaryHover: '#4B5563',
  secondaryLight: '#F3F4F6',

  success: '#10B981',
  successHover: '#059669',
  successLight: '#D1FAE5',
  successDark: '#047857',

  warning: '#F59E0B',
  warningHover: '#D97706',
  warningLight: '#FEF3C7',
  warningDark: '#B45309',

  danger: '#EF4444',
  dangerHover: '#DC2626',
  dangerLight: '#FEE2E2',
  dangerDark: '#B91C1C',

  info: '#3B82F6',
  infoLight: '#DBEAFE',

  background: '#FFFFFF',
  surface: '#FFFFFF',
  surfaceHover: '#F9FAFB',
  border: '#E5E7EB',
  borderHover: '#D1D5DB',

  text: '#1F2937',
  textSecondary: '#6B7280',
  textMuted: '#9CA3AF',
  textInverse: '#FFFFFF',
} as const;

/**
 * Spacing tokens in pixels.
 */
export const spacing = {
  0: 0,
  1: 4,
  2: 8,
  3: 12,
  4: 16,
  5: 20,
  6: 24,
  8: 32,
  10: 40,
  12: 48,
} as const;

/**
 * Font size tokens in rem.
 */
export const fontSize = {
  xs: '0.75rem',
  sm: '0.875rem',
  base: '1rem',
  lg: '1.125rem',
  xl: '1.25rem',
  '2xl': '1.5rem',
} as const;

/**
 * Border radius tokens in rem.
 */
export const borderRadius = {
  none: '0',
  sm: '0.125rem',
  md: '0.375rem',
  lg: '0.5rem',
  xl: '0.75rem',
  '2xl': '1rem',
  full: '9999px',
} as const;

/**
 * Shadow tokens.
 */
export const shadows = {
  sm: '0 1px 2px 0 rgb(0 0 0 / 0.05)',
  md: '0 4px 6px -1px rgb(0 0 0 / 0.1), 0 2px 4px -2px rgb(0 0 0 / 0.1)',
  lg: '0 10px 15px -3px rgb(0 0 0 / 0.1), 0 4px 6px -4px rgb(0 0 0 / 0.1)',
  xl: '0 20px 25px -5px rgb(0 0 0 / 0.1), 0 8px 10px -6px rgb(0 0 0 / 0.1)',
} as const;
