/**
 * Unit tests for the mobile document-share validation helper (Story 7A.5).
 *
 * These pin the mobile half of the UX-nit fix tracked as sprint-status issue
 * #485. The web share panel (DocumentSharePanel.tsx) already rejects a
 * malformed user-ID with an inline error before it reaches the network; the
 * mobile DocumentShareSheet previously only checked that the field was
 * non-empty, so free-form text would round-trip to the backend and surface a
 * confusing 4xx. `isValidUserId` closes that gap, and DocumentShareSheet's
 * `handleCreate` now blocks the create mutation on a non-UUID user-ID.
 *
 * The helper is a pure function, so no React Native / native mocking is needed.
 */

import { isValidUserId } from './documentShare';

describe('isValidUserId', () => {
  it.each([
    '11111111-2222-3333-4444-555555555555',
    'abcdefab-1234-5678-9abc-def012345678',
    'ABCDEFAB-1234-5678-9ABC-DEF012345678', // case-insensitive
  ])('accepts a well-formed UUID %s', (uuid) => {
    expect(isValidUserId(uuid)).toBe(true);
  });

  it('trims surrounding whitespace before validating', () => {
    expect(isValidUserId('  11111111-2222-3333-4444-555555555555  ')).toBe(true);
  });

  it.each([
    ['empty string', ''],
    ['whitespace only', '   '],
    ['free-form text', 'not-a-uuid'],
    ['too short', '11111111-2222-3333-4444-5555555555'],
    ['too long', '11111111-2222-3333-4444-5555555555556'],
    ['wrong separators', '11111111_2222_3333_4444_555555555555'],
    ['non-hex characters', 'zzzzzzzz-2222-3333-4444-555555555555'],
    ['missing groups', '11111111-2222-3333-555555555555'],
  ])('rejects %s', (_label, value) => {
    expect(isValidUserId(value)).toBe(false);
  });
});
