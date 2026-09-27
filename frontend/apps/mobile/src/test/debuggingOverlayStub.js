/**
 * Stub for `react-native/Libraries/Debugging/DebuggingOverlay`.
 *
 * `DebuggingOverlay` is a dev-only visual overlay that `AppContainer` renders in
 * `__DEV__`. In React Native 0.87 it imports its codegen spec from
 * `src/private/components/debuggingoverlay/specs/DebuggingOverlayNativeComponent`,
 * a Flow-typed file that jest-expo 56's babel transform fails to parse
 * ("':' or '?' expected in property type annotation"). That crashes any test
 * whose tree pulls in `Modal` (Modal -> AppContainer -> DebuggingOverlay), e.g.
 * LanguageSwitcher.
 *
 * `moduleNameMapper` in `jest.config.js` redirects the module to this no-op
 * component so the overlay is skipped under test — it has no behaviour worth
 * asserting and never renders in production tests anyway.
 */
function DebuggingOverlay() {
  return null;
}

module.exports = DebuggingOverlay;
module.exports.default = DebuggingOverlay;
