/// <reference types="vitest/globals" />
/**
 * `BallotQuestion` i18n regression (code-review-ppt-web-ui-ballotquestion-yesno-i18n).
 *
 * The ballot inputs used to render hardcoded "Yes" / "No" button labels and
 * English reorder aria-labels instead of `t()`, so sk/cs/de/pl/hu voters saw
 * English on the one screen where the choice is the whole interaction. The copy
 * was moved to the shared `common.yes` / `common.no` and `aria.moveUp` /
 * `aria.moveDown` keys and rendered via `useTranslation`.
 *
 * Hardcoded UI copy is invisible to typecheck + lint — it only shows up when the
 * app runs in a non-English locale. This test locks the fix in two ways:
 *   1. render-level — `YesNo` and `Ranked` are rendered under a non-English
 *      locale and must announce the *translated* copy (a hardcoded "Yes"/"No"
 *      or "Move up"/"Move down" fails here);
 *   2. bundle-level — every key the component reads must exist, and be a
 *      non-empty string, in all six shipped locale bundles (a missing key would
 *      silently fall back to English via `fallbackLng`).
 */
import type { VoteQuestion } from '@ppt/api-client';
import { render, screen } from '@testing-library/react';
import i18n from 'i18next';
import cs from '../../messages/cs.json';
import de from '../../messages/de.json';
import en from '../../messages/en.json';
import hu from '../../messages/hu.json';
import pl from '../../messages/pl.json';
import sk from '../../messages/sk.json';
import { BallotQuestion } from '../features/voting/components/BallotQuestion';

type Json = Record<string, unknown>;

const BUNDLES: Record<string, Json> = { en, sk, cs, de, pl, hu };

/** Keys the BallotQuestion component renders through `t()`. */
const COMPONENT_KEYS = ['common.yes', 'common.no', 'aria.moveUp', 'aria.moveDown'] as const;

function getPath(obj: unknown, path: string): unknown {
  return path.split('.').reduce<unknown>((acc, part) => {
    if (acc === null || typeof acc !== 'object') return undefined;
    return (acc as Json)[part];
  }, obj);
}

/** Register a locale's bundle with the test i18n instance (setup only loads en). */
function useLocale(locale: string) {
  i18n.addResourceBundle(locale, 'translation', BUNDLES[locale], true, true);
  return i18n.changeLanguage(locale);
}

const yesNoQuestion: VoteQuestion = {
  id: 'q-yn',
  question_text: 'Approve the budget?',
  question_type: 'yes_no',
  is_required: true,
  description: null,
  options: [],
} as unknown as VoteQuestion;

const rankedQuestion: VoteQuestion = {
  id: 'q-rank',
  question_text: 'Rank the candidates',
  question_type: 'ranked',
  is_required: true,
  description: null,
  options: [
    { id: 'opt-a', text: 'Alice' },
    { id: 'opt-b', text: 'Bob' },
  ],
} as unknown as VoteQuestion;

afterEach(async () => {
  await i18n.changeLanguage('en');
});

describe('BallotQuestion i18n', () => {
  it('renders yes/no labels from the active locale, not hardcoded English', async () => {
    await useLocale('sk');
    render(<BallotQuestion question={yesNoQuestion} value={undefined} onChange={() => {}} />);

    expect(screen.getByRole('button', { name: 'Áno' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Nie' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Yes' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'No' })).not.toBeInTheDocument();
  });

  it('renders ranked reorder aria-labels from the active locale', async () => {
    await useLocale('de');
    render(<BallotQuestion question={rankedQuestion} value={undefined} onChange={() => {}} />);

    expect(screen.getAllByRole('button', { name: 'Nach oben verschieben' }).length).toBeGreaterThan(
      0
    );
    expect(
      screen.getAllByRole('button', { name: 'Nach unten verschieben' }).length
    ).toBeGreaterThan(0);
    expect(screen.queryByRole('button', { name: 'Move up' })).not.toBeInTheDocument();
  });

  it('covers exactly the six shipped locale bundles', () => {
    expect(Object.keys(BUNDLES).sort()).toEqual(['cs', 'de', 'en', 'hu', 'pl', 'sk']);
  });

  for (const [locale, bundle] of Object.entries(BUNDLES)) {
    for (const key of COMPONENT_KEYS) {
      it(`defines a non-empty ${key} in ${locale}.json`, () => {
        const value = getPath(bundle, key);
        expect(value, `${key} missing in ${locale}.json`).toBeTypeOf('string');
        expect((value as string).trim().length).toBeGreaterThan(0);
      });
    }
  }
});
