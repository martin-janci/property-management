/**
 * Stub for `@react-native/assets-registry/registry`.
 *
 * jest-expo 56's `src/preset/setup.js` unconditionally calls
 * `jest.mock('@react-native/assets-registry/registry', …)`. React Native 0.87
 * dropped that standalone package (its asset helpers moved into
 * `@react-native/asset-utils` and `react-native/Libraries/Image/*`), so the
 * module id no longer resolves and jest-expo's preset throws
 * `Cannot find module '@react-native/assets-registry/registry'` before any
 * mobile test can start.
 *
 * `moduleNameMapper` in `jest.config.js` redirects that id here so the preset's
 * `jest.mock(...)` has a resolvable target. jest-expo supplies its own mock
 * implementation via the factory, so this shim is only ever the resolution
 * target; the functional exports below just keep any direct import safe.
 */
module.exports = {
  registerAsset: () => 1,
  getAssetByID: () => undefined,
};
