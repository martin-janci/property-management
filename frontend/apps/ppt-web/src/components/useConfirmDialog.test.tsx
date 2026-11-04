/// <reference types="vitest/globals" />
/**
 * useConfirmDialog — promise-based replacement for window.confirm() backed by
 * the accessible ConfirmationDialog.
 */
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';
import { useConfirmDialog } from './useConfirmDialog';

vi.mock('react-i18next', () => ({ useTranslation: () => ({ t: (k: string) => k }) }));

function Harness() {
  const { confirm, confirmDialog } = useConfirmDialog();
  const [result, setResult] = useState('none');
  return (
    <>
      <button
        type="button"
        onClick={async () =>
          setResult(String(await confirm({ message: 'Really?', confirmLabel: 'Yes, do it' })))
        }
      >
        ask
      </button>
      <output>{result}</output>
      {confirmDialog}
    </>
  );
}

describe('useConfirmDialog', () => {
  it('renders nothing until confirm() is called', () => {
    render(<Harness />);
    expect(screen.queryByText('Really?')).not.toBeInTheDocument();
  });

  it('resolves true when the user confirms', async () => {
    render(<Harness />);
    await userEvent.click(screen.getByRole('button', { name: 'ask' }));
    expect(screen.getByText('Really?')).toBeInTheDocument();
    expect(screen.getByText('common.confirmTitle')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Yes, do it' }));
    expect(await screen.findByText('true')).toBeInTheDocument();
    expect(screen.queryByText('Really?')).not.toBeInTheDocument();
  });

  it('resolves false when the user cancels', async () => {
    render(<Harness />);
    await userEvent.click(screen.getByRole('button', { name: 'ask' }));
    await userEvent.click(screen.getByRole('button', { name: 'common.cancel' }));
    expect(await screen.findByText('false')).toBeInTheDocument();
  });

  it('never calls window.confirm', async () => {
    const spy = vi.spyOn(window, 'confirm');
    render(<Harness />);
    await userEvent.click(screen.getByRole('button', { name: 'ask' }));
    expect(spy).not.toHaveBeenCalled();
    spy.mockRestore();
  });
});
