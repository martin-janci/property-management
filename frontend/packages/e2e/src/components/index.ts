/**
 * Shared UI region components.
 *
 * Composition over inheritance: pages own component instances rather than
 * extending a god-page. Each component is a thin, resilient wrapper over a
 * single UI region and is reused across every app's page objects.
 */

export { ConnectionStatus } from './ConnectionStatus';
export { LanguageSwitcher } from './LanguageSwitcher';
export { Nav } from './Nav';
export type { Component } from './types';
