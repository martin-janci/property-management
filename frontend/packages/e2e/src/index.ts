/**
 * @ppt/e2e — shared object-oriented Playwright E2E framework for the PPT
 * monorepo. Import everything from here.
 *
 *   import { defineE2EConfig, test, expect, SitemapPage } from '@ppt/e2e';
 */

export type { Component } from './components';
// Components
export { ConnectionStatus, LanguageSwitcher, Nav } from './components';
export type { DefineE2EConfigOptions } from './config';
// Config
export { defineE2EConfig } from './config';
export type { AppName, E2EEnv, ResolvedEnv, SitemapApp } from './env';
// Env
export {
  getApiURL,
  getBaseURL,
  getLocalPort,
  resolveEnv,
  resolveEnvFor,
  shouldStartWebServer,
} from './env';
export type {
  ApiResult,
  PptFixtures,
  PptWorkerOptions,
  TestRole,
  TestUserCreds,
} from './fixtures';
// Fixtures
export {
  ApiHelper,
  AppHarness,
  expect,
  test,
  testUsers,
} from './fixtures';
// Flow engine (centerpiece)
export { backendEnabled, FlowRunner, flowNeedsBackend } from './flow/FlowRunner';
export type {
  GotoOptions,
  LoginPageOptions,
  SitemapPageOptions,
} from './pages';
// Pages
export { BasePage, LoginPage, SitemapPage, sitemapPage } from './pages';
export type { ByRoleOptions } from './selectors';
// Selectors
export { Selectors, selectors, testId } from './selectors';
export type {
  RegisterA11yOptions,
  RegisterAuthOptions,
  RegisterFlowOptions,
  RegisterNavigationOptions,
  RegisterRouteHealthOptions,
} from './specs';
// Spec factories
export {
  contentPublicRoutes,
  isCallbackRoute,
  isSitemapApp,
  navigablePublicRoutes,
  registerA11ySpecs,
  registerAuthSpecs,
  registerFlowSpecs,
  registerNavigationSpecs,
  registerRouteHealthSpecs,
  routeUrl,
} from './specs';
export type { ElementType, LocateOptions, TestIdRegistry } from './testids';
// Testid strategy
export {
  byTestId,
  ELEMENT_TYPES,
  locate,
  testIds,
  tid,
  tidFilter,
  tidPagination,
} from './testids';
