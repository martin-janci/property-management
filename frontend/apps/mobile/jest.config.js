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
  },
  // Clear transform to use expo's babel preset
  transform: {
    '^.+\\.(js|jsx|ts|tsx)$': 'babel-jest',
  },
  // Match any file within node_modules that needs transformation
  transformIgnorePatterns: [],
};
