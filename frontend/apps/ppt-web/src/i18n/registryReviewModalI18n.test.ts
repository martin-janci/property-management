/// <reference types="vitest/globals" />
/// <reference types="vite/client" />
/**
 * ReviewRegistrationModal i18n regression guard.
 *
 * `features/registry/components/ReviewRegistrationModal.tsx` (the approve/reject
 * modal for pet + vehicle registrations, Epic 57) originally hard-coded every
 * user-facing string — the dialog title, the "Choose action" prompt, the
 * approve/reject button + SVG labels, the approval-confirmation copy, the
 * rejection-reason field label/placeholder/hint, and the submit buttons'
 * idle/pending captions. They are now routed through `react-i18next`:
 *
 *   • `registry.review.*` — the modal's own copy (added with this change).
 *   • `common.back` / `common.cancel` — the shared Back + Cancel buttons.
 *
 * This test locks those keys into every shipped locale and asserts the
 * component still reads each one, so a dropped locale subtree (the merge-train
 * failure mode documented in `localeNamespaceParity.test.ts`) or a regression
 * back to a hard-coded literal fails CI instead of shipping a raw key string
 * (or English copy) into the modal.
 *
 * Unlike the older `registryActionsI18n.test.ts` (en/sk/cs trio only), this
 * guards all six bundles: the `registry` namespace is carried by every locale
 * and `localeLeafParity.test.ts`'s NO-GAP invariant already requires every en
 * `registry.review.*` leaf to be present in each of them.
 */
import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';

type Json = Record<string, unknown>;

const BUNDLES: Record<string, Json> = { en, sk, cs, de, pl, hu };

/** Leaf keys the modal reads, relative to `registry.review`. */
const REVIEW_KEYS = [
  'titlePet',
  'titleVehicle',
  'chooseAction',
  'approve',
  'reject',
  'infoIconLabel',
  'approveConfirm',
  'confirmApproval',
  'approving',
  'rejectionReasonLabel',
  'rejectionPlaceholder',
  'rejectionHint',
  'confirmRejection',
  'rejecting',
] as const;

/** Shared keys the modal reuses for its Back + Cancel buttons. */
const COMMON_KEYS = ['back', 'cancel'] as const;

const reviewLeaf = (bundle: Json, key: string): unknown => {
  const registry = bundle.registry as Json | undefined;
  const review = registry?.review as Json | undefined;
  return review?.[key];
};

const commonLeaf = (bundle: Json, key: string): unknown => {
  const common = bundle.common as Json | undefined;
  return common?.[key];
};

const isNonEmptyString = (v: unknown): boolean => typeof v === 'string' && v.trim() !== '';

describe('ReviewRegistrationModal i18n (all six locales)', () => {
  for (const [lang, bundle] of Object.entries(BUNDLES)) {
    it(`${lang} has every registry.review key as a non-empty string`, () => {
      const missing = REVIEW_KEYS.filter((k) => !isNonEmptyString(reviewLeaf(bundle, k)));
      expect(missing, `${lang}: missing/empty registry.review keys`).toEqual([]);
    });

    it(`${lang} has the common Back/Cancel keys the modal reuses`, () => {
      const missing = COMMON_KEYS.filter((k) => !isNonEmptyString(commonLeaf(bundle, k)));
      expect(missing, `${lang}: missing/empty common keys`).toEqual([]);
    });
  }

  it('ReviewRegistrationModal.tsx references every guarded key', () => {
    const sources = import.meta.glob(
      '../features/registry/components/ReviewRegistrationModal.tsx',
      {
        query: '?raw',
        import: 'default',
        eager: true,
      }
    ) as Record<string, string>;
    const src = Object.values(sources)[0];
    expect(src, 'ReviewRegistrationModal.tsx not found').toBeTruthy();

    for (const k of REVIEW_KEYS) {
      expect(src, `ReviewRegistrationModal.tsx no longer reads registry.review.${k}`).toContain(
        `registry.review.${k}`
      );
    }
    for (const k of COMMON_KEYS) {
      expect(src, `ReviewRegistrationModal.tsx no longer reads common.${k}`).toContain(
        `common.${k}`
      );
    }
  });

  it('ReviewRegistrationModal.tsx carries no leftover hard-coded modal copy', () => {
    const sources = import.meta.glob(
      '../features/registry/components/ReviewRegistrationModal.tsx',
      {
        query: '?raw',
        import: 'default',
        eager: true,
      }
    ) as Record<string, string>;
    const src = Object.values(sources)[0];
    // Representative English literals that must not reappear outside a key path.
    for (const literal of ['Confirm Approval', 'Confirm Rejection', 'Choose action:']) {
      expect(src, `ReviewRegistrationModal.tsx reintroduced hard-coded "${literal}"`).not.toContain(
        literal
      );
    }
  });
});
