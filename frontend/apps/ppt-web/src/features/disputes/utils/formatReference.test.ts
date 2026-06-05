/// <reference types="vitest/globals" />
/**
 * Unit tests for formatDisputeReference — derives a short, human-friendly
 * reference code from a dispute UUID instead of uppercasing the full UUID.
 */

import { describe, expect, it } from 'vitest';
import { formatDisputeReference } from './formatReference';

describe('formatDisputeReference', () => {
  it('renders a short DSP code from the first 8 hex chars of the UUID', () => {
    expect(formatDisputeReference('3f2a1b9c-4d5e-6f70-8910-abcdef123456')).toBe('DSP-3F2A1B9C');
  });

  it('does not return the full uppercased UUID', () => {
    const id = '3f2a1b9c-4d5e-6f70-8910-abcdef123456';
    const ref = formatDisputeReference(id);
    expect(ref).not.toBe(`DSP-${id.toUpperCase()}`);
    expect(ref.length).toBeLessThanOrEqual(12);
  });

  it('strips hyphens so the code is always 8 hex digits', () => {
    // First UUID segment is shorter than 8 chars here; hyphen stripping
    // ensures we still take 8 contiguous hex characters.
    expect(formatDisputeReference('abc-de-1234567890')).toBe('DSP-ABCDE123');
  });

  it('uppercases the code', () => {
    expect(formatDisputeReference('deadbeefcafe')).toBe('DSP-DEADBEEF');
  });
});
