/**
 * Tests for the system-announcement form validator (#521).
 */
/// <reference types="vitest/globals" />
import { describe, expect, it } from 'vitest';
import { type AnnFormValues, validateAnnForm } from './schema';

function base(overrides: Partial<AnnFormValues> = {}): AnnFormValues {
  return {
    title: 'Hello',
    message: 'Body',
    severity: 'info',
    start_at: '2026-06-01T09:00',
    end_at: '',
    is_dismissible: true,
    requires_acknowledgment: false,
    ...overrides,
  };
}

describe('validateAnnForm', () => {
  it('accepts a minimal valid form', () => {
    expect(validateAnnForm(base())).toEqual({});
  });

  it('rejects empty title with a friendly message', () => {
    const errs = validateAnnForm(base({ title: '   ' }));
    expect(errs.title).toMatch(/required/i);
  });

  it('rejects empty message', () => {
    const errs = validateAnnForm(base({ message: '' }));
    expect(errs.message).toMatch(/required/i);
  });

  it('rejects too-long title (>200)', () => {
    const errs = validateAnnForm(base({ title: 'x'.repeat(201) }));
    expect(errs.title).toMatch(/200/);
  });

  it('rejects too-long message (>2000)', () => {
    const errs = validateAnnForm(base({ message: 'x'.repeat(2001) }));
    expect(errs.message).toMatch(/2000/);
  });

  it('rejects expire-at on or before publish-at', () => {
    expect(
      validateAnnForm(base({ start_at: '2026-06-01T09:00', end_at: '2026-06-01T09:00' })).end_at
    ).toMatch(/after/i);
    expect(
      validateAnnForm(base({ start_at: '2026-06-01T10:00', end_at: '2026-06-01T09:00' })).end_at
    ).toMatch(/after/i);
  });

  it('accepts a future expire-at', () => {
    expect(
      validateAnnForm(base({ start_at: '2026-06-01T09:00', end_at: '2026-06-01T10:00' }))
    ).toEqual({});
  });

  it('rejects an unknown severity', () => {
    // @ts-expect-error — exercise the runtime guard
    expect(validateAnnForm(base({ severity: 'oops' })).severity).toBeDefined();
  });
});
