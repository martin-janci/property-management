/// <reference types="vitest/globals" />
/**
 * usePromptDialog — promise-based replacement for window.prompt() backed by the
 * accessible PromptDialog.
 */
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';
import { usePromptDialog } from './usePromptDialog';

vi.mock('react-i18next', () => ({ useTranslation: () => ({ t: (k: string) => k }) }));

function Harness() {
  const { prompt, promptDialog } = usePromptDialog();
  const [result, setResult] = useState('none');
  return (
    <>
      <button
        type="button"
        onClick={async () =>
          setResult(
            JSON.stringify(
              await prompt({ message: 'Reason?', confirmLabel: 'Submit it', inputLabel: 'Reason' })
            )
          )
        }
      >
        ask
      </button>
      <output>{result}</output>
      {promptDialog}
    </>
  );
}

describe('usePromptDialog', () => {
  it('renders nothing until prompt() is called', () => {
    render(<Harness />);
    expect(screen.queryByText('Reason?')).not.toBeInTheDocument();
  });

  it('resolves the entered value when the user submits', async () => {
    render(<Harness />);
    await userEvent.click(screen.getByRole('button', { name: 'ask' }));
    expect(screen.getByText('Reason?')).toBeInTheDocument();
    await userEvent.type(screen.getByRole('textbox', { name: 'Reason' }), 'too noisy');
    await userEvent.click(screen.getByRole('button', { name: 'Submit it' }));
    expect(await screen.findByText('"too noisy"')).toBeInTheDocument();
    expect(screen.queryByText('Reason?')).not.toBeInTheDocument();
  });

  it('resolves null when the user cancels', async () => {
    render(<Harness />);
    await userEvent.click(screen.getByRole('button', { name: 'ask' }));
    await userEvent.click(screen.getByRole('button', { name: 'common.cancel' }));
    expect(await screen.findByText('null')).toBeInTheDocument();
  });

  it('disables submit while the required field is empty', async () => {
    render(<Harness />);
    await userEvent.click(screen.getByRole('button', { name: 'ask' }));
    expect(screen.getByRole('button', { name: 'Submit it' })).toBeDisabled();
    await userEvent.type(screen.getByRole('textbox', { name: 'Reason' }), 'x');
    expect(screen.getByRole('button', { name: 'Submit it' })).toBeEnabled();
  });

  it('never calls window.prompt', async () => {
    const spy = vi.spyOn(window, 'prompt');
    render(<Harness />);
    await userEvent.click(screen.getByRole('button', { name: 'ask' }));
    expect(spy).not.toHaveBeenCalled();
    spy.mockRestore();
  });
});
