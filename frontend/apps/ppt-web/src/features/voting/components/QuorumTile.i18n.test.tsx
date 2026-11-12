/// <reference types="vitest/globals" />
/**
 * QuorumTile i18n regression test.
 *
 * The quorum/participation progress bar shipped with no accessible, localized
 * label — assistive tech read nothing for the bar, and the only localized copy
 * came from the adjacent text nodes (see code-review-ppt-web-ui-quorumtile-i18n).
 * This guards the fix: the bar exposes `role="progressbar"` with an
 * `aria-label` resolved through react-i18next, so it announces in the active
 * locale (Slovak below) rather than being silent or English-only.
 */
import { render, screen } from '@testing-library/react';
import i18n from 'i18next';
import sk from '../../../../messages/sk.json';
import { QuorumTile } from './QuorumTile';

afterEach(async () => {
  await i18n.changeLanguage('en');
});

describe('QuorumTile i18n', () => {
  it('labels the progress bar with translated copy (English)', () => {
    render(<QuorumTile participation={6} eligible={10} quorumMet={false} />);

    const bar = screen.getByRole('progressbar', { name: 'Quorum participation: 60%' });
    expect(bar).toHaveAttribute('aria-valuenow', '60');
    expect(bar).toHaveAttribute('aria-valuemin', '0');
    expect(bar).toHaveAttribute('aria-valuemax', '100');
  });

  it('localizes the progress-bar label (Slovak)', async () => {
    i18n.addResourceBundle('sk', 'translation', sk, true, true);
    await i18n.changeLanguage('sk');

    render(<QuorumTile participation={6} eligible={10} quorumMet={false} />);

    expect(screen.getByRole('progressbar', { name: 'Účasť na kvóre: 60 %' })).toBeInTheDocument();
  });
});
