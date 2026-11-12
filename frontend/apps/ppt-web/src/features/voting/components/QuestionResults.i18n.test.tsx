/// <reference types="vitest/globals" />
/**
 * QuestionResults i18n regression test.
 *
 * The per-question results view marked the winning option with a bare `✓`
 * glyph and no accessible, translatable label — assistive tech announced only
 * "check mark" (or nothing), identically for every locale, so the *meaning*
 * (this is the winning option) was hardcoded out of the i18n system
 * (code-review-ppt-web-ui-questionresults-i18n). The winner marker now carries
 * `role="img"` + `aria-label={t('voting.results.winnerLabel')}`.
 *
 * This test locks two things:
 *   1. every shipped locale bundle defines a non-empty
 *      `voting.results.winnerLabel` (parity — a dropped locale falls back to
 *      English silently otherwise), and
 *   2. the rendered winner marker exposes the *translated* label (Slovak),
 *      which the pre-fix bare-glyph implementation could not.
 */
import type { QuestionResult } from '@ppt/api-client';
import { render, screen } from '@testing-library/react';
import i18n from 'i18next';
import cs from '../../../../messages/cs.json';
import de from '../../../../messages/de.json';
import en from '../../../../messages/en.json';
import hu from '../../../../messages/hu.json';
import pl from '../../../../messages/pl.json';
import sk from '../../../../messages/sk.json';
import { QuestionResults } from './QuestionResults';

type Json = Record<string, unknown>;
const BUNDLES: Record<string, Json> = { en, sk, cs, de, pl, hu };

const RESULT: QuestionResult = {
  question_id: 'q1',
  question_text: 'Approve the roof repair?',
  total_responses: 3,
  winner: 'opt-yes',
  options: [
    { option_id: 'opt-yes', option_text: 'Yes', count: 2, percentage: 66.6 },
    { option_id: 'opt-no', option_text: 'No', count: 1, percentage: 33.3 },
  ],
} as unknown as QuestionResult;

afterEach(async () => {
  await i18n.changeLanguage('en');
});

describe('QuestionResults i18n', () => {
  it('defines voting.results.winnerLabel in every shipped locale', () => {
    for (const [locale, bundle] of Object.entries(BUNDLES)) {
      const voting = bundle.voting as Json | undefined;
      const results = voting?.results as Json | undefined;
      const value = results?.winnerLabel;
      expect(value, `voting.results.winnerLabel missing in ${locale}.json`).toBeTypeOf('string');
      expect((value as string).trim().length).toBeGreaterThan(0);
    }
  });

  it('labels the winning option with translated copy (Slovak)', async () => {
    i18n.addResourceBundle('sk', 'translation', sk, true, true);
    await i18n.changeLanguage('sk');

    render(<QuestionResults result={RESULT} />);

    expect(screen.getByRole('img', { name: 'Víťazná možnosť' })).toBeInTheDocument();
  });
});
