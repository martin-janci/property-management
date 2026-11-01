/**
 * Listing-edit status mapping — the bridge between the backend lifecycle enum
 * and the wizard's 3-state status control (`active` / `inactive` / `draft`).
 *
 * This logic is the historical churn hotspot on this page (#1762): the backend
 * carries terminal lifecycle states (`sold` / `rented` / `archived`) the control
 * cannot represent, and publishing (`active`) is moderation-gated server-side.
 * Getting either direction wrong silently downgrades a live/terminal listing to
 * `draft`. It lived inline in `page.tsx` and was rewritten on every status /
 * i18n fix, so it is extracted here behind two small, unit-tested functions.
 */

import type { ListingEditFormData } from './_mock';

/** The wizard's 3-state status control value. */
export type FormStatus = ListingEditFormData['status'];

/**
 * Map a backend lifecycle status onto the form's 3-state control.
 *
 * `active` → `active`, `draft` → `draft`, `paused` → `inactive`. Everything
 * else — the terminal states `sold` / `rented` / `archived` plus any unknown
 * value — falls back to `draft` for DISPLAY ONLY. The real state is preserved
 * on save by {@link resolveStatusUpdate}, which omits `status` unless the owner
 * deliberately changed the control.
 */
export function mapListingStatusToForm(backendStatus: string | null | undefined): FormStatus {
  switch (backendStatus) {
    case 'active':
      return 'active';
    case 'paused':
      return 'inactive';
    default:
      // 'draft' and all terminal/unknown values display as 'draft'.
      return 'draft';
  }
}

/**
 * Resolve the `status` value to send on update, given the control's current
 * value and the value it loaded with.
 *
 * - Returns `undefined` when the owner did NOT change the control, so the
 *   backend preserves the listing's real lifecycle state. Without this, editing
 *   any field on a `sold` / `rented` / `archived` listing (all displayed as
 *   `draft`) would COALESCE that `draft` back over the terminal state and
 *   silently downgrade it (#1762).
 * - `inactive` → `paused`, `draft` → `draft` when the owner did change it.
 * - `active` is moderation-gated server-side, so it is NEVER sent directly —
 *   returns `undefined` even on an explicit change (the server rejects a direct
 *   `active` with 400; publishing goes through moderation).
 */
export function resolveStatusUpdate(current: FormStatus, initial: FormStatus): string | undefined {
  if (current === initial) return undefined;
  switch (current) {
    case 'inactive':
      return 'paused';
    case 'draft':
      return 'draft';
    default:
      // 'active' — moderation-gated, never sent directly.
      return undefined;
  }
}
