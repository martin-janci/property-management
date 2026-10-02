// Pin the timezone before Jest forks its workers, so they inherit it via the
// environment and ICU initialises against UTC. Date formatters such as
// `toLocaleDateString` (used by AnnouncementsScreen's relative-date label)
// otherwise render in the host TZ, making absolute "Mon D" assertions flip
// between e.g. "May 1" and "May 2" across CI runners — a recurring flaky-test
// churn source. Setting TZ from a setupFile is too late (ICU is already cached).
process.env.TZ = 'UTC';

/** @type {import('jest').Config} */
module.exports = {
  preset: 'jest-expo',
  setupFilesAfterEnv: ['<rootDir>/src/test/setup.ts'],
  testMatch: ['**/*.test.{ts,tsx}'],
  moduleFileExtensions: ['ts', 'tsx', 'js', 'jsx'],
  moduleNameMapper: {
    '^@ppt/api-client$': '<rootDir>/../../packages/api-client/src',
    '^@ppt/shared$': '<rootDir>/../../packages/shared/src',
    // React Native 0.87 removed the standalone `@react-native/assets-registry`
    // package, but jest-expo 56's preset still `jest.mock()`s it and throws
    // "Cannot find module" on boot — which blocked the entire mobile jest suite.
    // Redirect the dead id to a local stub so the preset's mock has a resolvable
    // target. See src/test/assetsRegistryStub.js.
    '^@react-native/assets-registry/registry$': '<rootDir>/src/test/assetsRegistryStub.js',
    // RN 0.87's dev-only DebuggingOverlay imports a Flow-typed codegen spec that
    // jest-expo 56 can't parse, crashing any test that renders Modal (Modal ->
    // AppContainer -> DebuggingOverlay). RN imports the overlay via a relative
    // path, so match the request by its trailing segment (the `Registry` sibling
    // ends differently and is left untouched) and stub it under test.
    'Debugging/DebuggingOverlay$': '<rootDir>/src/test/debuggingOverlayStub.js',
  },
  // Clear transform to use expo's babel preset
  transform: {
    '^.+\\.(js|jsx|ts|tsx)$': 'babel-jest',
  },
  // Match any file within node_modules that needs transformation
  transformIgnorePatterns: [],
};
