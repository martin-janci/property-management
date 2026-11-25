import { describe, expect, it } from 'vitest';
import { mapPrivacySettingsResponse, toPrivacySettingsBody } from './api';

describe('neighbour privacy settings wire mapping', () => {
  it('sends snake_case, which is what UpdatePrivacySettingsRequest deserializes', () => {
    expect(toPrivacySettingsBody({ profileVisibility: 'hidden', showContactInfo: false })).toEqual({
      profile_visibility: 'hidden',
      show_contact_info: false,
    });
  });

  it('reads the snake_case settings the server returns', () => {
    expect(
      mapPrivacySettingsResponse({
        settings: { profile_visibility: 'contacts_only', show_contact_info: true },
      })
    ).toEqual({ settings: { profileVisibility: 'contacts_only', showContactInfo: true } });
  });

  it('falls back to the server defaults for an empty body', () => {
    expect(mapPrivacySettingsResponse(null)).toEqual({
      settings: { profileVisibility: 'visible', showContactInfo: false },
    });
  });
});
