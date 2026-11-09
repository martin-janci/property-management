/// <reference types="vitest/globals" />
/// <reference types="vite/client" />
/**
 * RegistryPage destructive-action i18n regression guard.
 *
 * The four destructive handlers on `features/registry/pages/RegistryPage.tsx`
 * (handlePetDelete, handlePetReview, handleVehicleDelete, handleVehicleReview)
 * route every confirm prompt and every success/error toast through `t()`:
 *
 *   • `registry.confirm.*` — the confirm-dialog copy (added with the
 *     auth-context / review work, #3354).
 *   • `registry.toast.*`   — the success + failure toast title/body pairs
 *     (added when the handlers started surfacing outcomes via toast, #3359).
 *
 * Those two key groups are *not* covered by `registryPageI18n.test.ts` (which
 * guards only the page chrome — title/subtitle/tabs/loadError) nor by
 * `localeNamespaceParity.test.ts` (which deliberately asserts top-level
 * namespaces only, never leaf keys). So the exact merge-train failure mode that
 * `localeNamespaceParity.test.ts` documents — a mis-resolved conflict silently
 * dropping a locale subtree — would, for these keys, ship a raw key string
 * (e.g. `registry.toast.petDeleteFailed`) into a user-facing toast with nothing
 * catching it.
 *
 * This locks the destructive-action confirm + toast keys into the
 * fully-translated trio (en/sk/cs — the same trio `localeNamespaceParity`
 * keeps in full parity). de/pl/hu are intentionally excluded: they are
 * partially translated and fall back to `en` at the leaf level by design.
 */
import cs from '../../messages/cs.json';
import en from '../../messages/en.json';
import sk from '../../messages/sk.json';

type Json = Record<string, unknown>;

/** Fully-translated trio kept in leaf-key parity (mirrors localeNamespaceParity). */
const FULL_TRIO: Record<string, Json> = { en, sk, cs };

/** Leaf keys the destructive handlers read, relative to `registry.toast`. */
const TOAST_KEYS = [
  'petDeleted',
  'petDeletedBody',
  'petDeleteFailed',
  'petDeleteFailedBody',
  'petReviewed',
  'petReviewedBody',
  'petReviewFailed',
  'petReviewFailedBody',
  'vehicleDeleted',
  'vehicleDeletedBody',
  'vehicleDeleteFailed',
  'vehicleDeleteFailedBody',
  'vehicleReviewed',
  'vehicleReviewedBody',
  'vehicleReviewFailed',
  'vehicleReviewFailedBody',
] as const;

/** Leaf keys the confirm prompts read, relative to `registry.confirm`. */
const CONFIRM_KEYS = [
  'approve',
  'approvePet',
  'approveVehicle',
  'deletePet',
  'deleteVehicle',
] as const;

const leaf = (bundle: Json, namespace: string, key: string): unknown => {
  const registry = bundle.registry as Json | undefined;
  const group = registry?.[namespace] as Json | undefined;
  return group?.[key];
};

const isNonEmptyString = (v: unknown): boolean => typeof v === 'string' && v.trim() !== '';

describe('registry destructive-action i18n (en/sk/cs full trio)', () => {
  for (const [lang, bundle] of Object.entries(FULL_TRIO)) {
    it(`${lang} has every registry.toast key as a non-empty string`, () => {
      const missing = TOAST_KEYS.filter((k) => !isNonEmptyString(leaf(bundle, 'toast', k)));
      expect(missing, `${lang}: missing/empty registry.toast keys`).toEqual([]);
    });

    it(`${lang} has every registry.confirm key as a non-empty string`, () => {
      const missing = CONFIRM_KEYS.filter((k) => !isNonEmptyString(leaf(bundle, 'confirm', k)));
      expect(missing, `${lang}: missing/empty registry.confirm keys`).toEqual([]);
    });
  }

  it('RegistryPage.tsx references every guarded toast + confirm key', () => {
    const sources = import.meta.glob('../features/registry/pages/RegistryPage.tsx', {
      query: '?raw',
      import: 'default',
      eager: true,
    }) as Record<string, string>;
    const src = Object.values(sources)[0];
    expect(src, 'RegistryPage.tsx not found').toBeTruthy();

    for (const k of TOAST_KEYS) {
      expect(src, `RegistryPage.tsx no longer reads registry.toast.${k}`).toContain(
        `registry.toast.${k}`
      );
    }
    for (const k of CONFIRM_KEYS) {
      expect(src, `RegistryPage.tsx no longer reads registry.confirm.${k}`).toContain(
        `registry.confirm.${k}`
      );
    }
  });
});
