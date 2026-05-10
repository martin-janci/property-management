/**
 * @ppt/ui-kit tokens barrel export.
 *
 * Exposed via package.json `exports["./tokens"]`. Use these import paths:
 *   CSS:      import '@ppt/ui-kit/tokens/tokens.css';
 *   JS/TS:    import { tokens } from '@ppt/ui-kit/tokens';
 *   Tailwind: import { pptPreset } from '@ppt/ui-kit/tokens';
 */

export { default as tailwindPreset, pptPreset } from './tailwind-preset';
export * from './tokens';
