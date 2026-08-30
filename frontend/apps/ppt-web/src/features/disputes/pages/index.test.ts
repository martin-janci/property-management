/// <reference types="vitest/globals" />
/**
 * Disputes pages barrel guard.
 *
 * Regression for the removed `CreateDisputePage` stub: its `handleSubmit` was a
 * `setTimeout(1000)` no-op that silently discarded every submission. The live
 * `/disputes/new` route uses `FileDisputePageRoute` (which wires the real
 * `useCreateDispute` mutation), so the orphaned stub was deleted. This guard
 * fails if anyone re-exports it from the barrel and a future author mounts a
 * form that loses submissions.
 */

import * as pages from './index';

describe('disputes pages barrel', () => {
  it('does not re-export the removed CreateDisputePage stub', () => {
    expect(pages).not.toHaveProperty('CreateDisputePage');
  });

  it('exports the live FileDisputePage', () => {
    expect(pages).toHaveProperty('FileDisputePage');
  });
});
