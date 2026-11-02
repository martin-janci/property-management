/// <reference types="vitest/globals" />

import { describe, expect, it } from 'vitest';
import { mapListingStatusToForm, resolveStatusUpdate } from './_status';

/**
 * Covers the listing-edit status-mapping churn hotspot (#1762).
 *
 * The 3-state form control cannot represent backend terminal states
 * (`sold` / `rented` / `archived`), and publishing (`active`) is
 * moderation-gated server-side. These tests pin both directions of the mapping
 * so future status / i18n fixes can't silently reintroduce the downgrade bug.
 */
describe('mapListingStatusToForm', () => {
  it('maps the directly-representable states', () => {
    expect(mapListingStatusToForm('active')).toBe('active');
    expect(mapListingStatusToForm('draft')).toBe('draft');
    expect(mapListingStatusToForm('paused')).toBe('inactive');
  });

  it('displays terminal states as draft (display only)', () => {
    expect(mapListingStatusToForm('sold')).toBe('draft');
    expect(mapListingStatusToForm('rented')).toBe('draft');
    expect(mapListingStatusToForm('archived')).toBe('draft');
  });

  it('falls back to draft for unknown / missing values', () => {
    expect(mapListingStatusToForm('something-new')).toBe('draft');
    expect(mapListingStatusToForm(undefined)).toBe('draft');
    expect(mapListingStatusToForm(null)).toBe('draft');
  });
});

describe('resolveStatusUpdate', () => {
  it('omits status when the control was not changed', () => {
    expect(resolveStatusUpdate('draft', 'draft')).toBeUndefined();
    expect(resolveStatusUpdate('inactive', 'inactive')).toBeUndefined();
    expect(resolveStatusUpdate('active', 'active')).toBeUndefined();
  });

  it('preserves a terminal listing (loaded as draft) when another field is edited', () => {
    // #1762: sold/rented/archived all load as 'draft'; an edit that does not
    // touch the status control must send no status, so the backend keeps the
    // real terminal state instead of COALESCE-ing 'draft' over it.
    expect(resolveStatusUpdate('draft', 'draft')).toBeUndefined();
  });

  it('maps a deliberate change to the backend enum', () => {
    expect(resolveStatusUpdate('inactive', 'draft')).toBe('paused');
    expect(resolveStatusUpdate('draft', 'inactive')).toBe('draft');
  });

  it('never sends active directly — publishing is moderation-gated', () => {
    expect(resolveStatusUpdate('active', 'draft')).toBeUndefined();
    expect(resolveStatusUpdate('active', 'inactive')).toBeUndefined();
  });
});
