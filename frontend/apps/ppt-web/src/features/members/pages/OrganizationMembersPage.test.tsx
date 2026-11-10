/// <reference types="vitest/globals" />
import { ApiError } from '@ppt/api-client';
import { describe, expect, it } from 'vitest';
import { memberErrorKey } from './OrganizationMembersPage';

describe('memberErrorKey', () => {
  it('maps HTTP statuses from the members API to translated keys', () => {
    expect(memberErrorKey(new ApiError(404, 'User not found'))).toBe('userNotFound');
    expect(memberErrorKey(new ApiError(409, 'Already a member'))).toBe('alreadyMember');
    expect(memberErrorKey(new ApiError(403, 'nope'))).toBe('forbidden');
    expect(memberErrorKey(new ApiError(400, 'bad'))).toBe('invalid');
    expect(memberErrorKey(new Error('boom'))).toBe('generic');
  });
});
