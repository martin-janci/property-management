/// <reference types="vitest/globals" />
/**
 * MetersPage nav (nav-fixes): `/meters/validations` had no inbound link. The
 * button renders only when the (manager-gated) handler is supplied.
 */
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MetersPage } from './MetersPage';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (k: string) => k, i18n: { language: 'en' } }),
}));

const base = {
  meters: [],
  onNavigateToDetail: vi.fn(),
  onNavigateToSubmitReading: vi.fn(),
};

describe('MetersPage validations link', () => {
  it('shows and wires the validations button when a handler is given', async () => {
    const onValidations = vi.fn();
    render(<MetersPage {...base} onNavigateToValidations={onValidations} />);
    await userEvent.click(screen.getByRole('button', { name: 'meters.nav.validations' }));
    expect(onValidations).toHaveBeenCalledTimes(1);
  });

  it('hides it otherwise (residents)', () => {
    render(<MetersPage {...base} />);
    expect(
      screen.queryByRole('button', { name: 'meters.nav.validations' })
    ).not.toBeInTheDocument();
  });
});
