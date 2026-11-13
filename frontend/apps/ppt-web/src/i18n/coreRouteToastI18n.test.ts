/// <reference types="vitest/globals" />
/**
 * Core route-group action-toast i18n parity regression.
 *
 * Sibling of `routeGroupToastI18n.test.ts`. That guard de-hardcoded the first
 * wave of route-group toasts (community / faults / iot / leases / meters /
 * person-months). A follow-up audit (code-review-ppt-web-core-route-toasts-i18n)
 * found a second wave of route-level `showToast({ title, message })` copy that
 * was routed through `t(...)` but whose keys were NOT present in every shipped
 * locale bundle — so the translation silently degraded to English:
 *
 *  - `predictive.acknowledgeFailed` / `sentiment.acknowledgeFailed`
 *    (routes/groups/ai-dashboards.tsx) existed in NO bundle at all; the toast
 *    only ever rendered its inline English `defaultValue`, for every locale
 *    including en.
 *  - the faults withdraw / photo-upload toasts, the messaging archive/delete
 *    toasts and the outages lifecycle toasts were defined in en/sk/cs but
 *    missing from de/pl/hu, so a manager on those locales saw the English
 *    `fallbackLng` string even though a translation convention exists.
 *
 * Toast copy is short, high-visibility status text that a user reads in the
 * moment an action succeeds or fails; unlike deep page body copy it is expected
 * to be translated in ALL six shipped locales (the same contract
 * `routeGroupToastI18n.test.ts` enforces), not left to the en fallback. This
 * test locks that: every core-route toast key below must resolve to a
 * non-empty string in all six bundles. It fails on the pre-fix tree (keys
 * missing from de/pl/hu, and the two `acknowledgeFailed` keys missing
 * everywhere) and passes once each key is defined at parity.
 */

import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';

type Json = Record<string, unknown>;

const BUNDLES: Record<string, Json> = { en, sk, cs, de, pl, hu };

function getPath(obj: unknown, path: string): unknown {
  return path.split('.').reduce<unknown>((acc, part) => {
    if (acc === null || typeof acc !== 'object') return undefined;
    return (acc as Json)[part];
  }, obj);
}

/**
 * Toast keys raised from the core route groups' mutation handlers that must be
 * translated at parity across every shipped locale.
 */
const REQUIRED_TOAST_KEYS: string[] = [
  // ai-dashboards.tsx — were absent from every bundle (en included).
  'predictive.acknowledgeFailed',
  'sentiment.acknowledgeFailed',
  // faults.tsx — withdraw + photo-upload toasts (missing from de/pl/hu).
  'faults.photoUploadFailed',
  'faults.photosNotAttached',
  'faults.withdrawFailed',
  'faults.withdrawn',
  // faults.tsx — detail-page action-mutation error toasts
  // (code-review-ppt-web-core-fault-detail-mutations-swallow-errors): the
  // triage/resolve/confirm/reopen/comment/delete-attachment mutations used to
  // swallow errors; these keys back the new error toasts and must be at parity.
  'faults.triageFailed',
  'faults.resolveFailed',
  'faults.confirmFailed',
  'faults.reopenFailed',
  'faults.commentFailed',
  'faults.attachmentDeleteFailed',
  // messaging.tsx — bulk archive/delete toasts (missing from de/pl/hu).
  'messaging.archiveSuccess',
  'messaging.archiveFailed',
  'messaging.deleteSuccess',
  'messaging.deleteFailed',
  // outages.tsx — outage lifecycle toasts (missing from de/pl/hu).
  'outages.createdSuccessfully',
  'outages.outageCreatedMsg',
  'outages.failedToCreate',
  'outages.updatedSuccessfully',
  'outages.outageUpdatedMsg',
  'outages.failedToUpdate',
  'outages.started',
  'outages.resolved',
  'outages.cancelled',
];

describe('core route-group action toast i18n parity', () => {
  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    describe(`locale: ${locale}`, () => {
      for (const path of REQUIRED_TOAST_KEYS) {
        it(`defines a non-empty ${path}`, () => {
          const value = getPath(bundle, path);
          expect(value, `${path} missing in ${locale}.json`).toBeTypeOf('string');
          expect((value as string).trim().length).toBeGreaterThan(0);
        });
      }
    });
  }
});
