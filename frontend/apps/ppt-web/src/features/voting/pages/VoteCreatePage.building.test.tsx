/// <reference types="vitest/globals" />
/**
 * VoteCreatePage building scope regression test: the vote must be created
 * for the building chosen via `?building=` / the picker, not `buildings[0]`.
 */
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { ToastProvider } from '../../../components';
import { VoteCreatePage } from './VoteCreatePage';

const h = vi.hoisted(() => ({
  createVote: vi.fn(async (_req: Record<string, unknown>) => ({ id: 'v-1' })),
}));

vi.mock('@ppt/api-client', () => ({
  useBuildings: () => ({
    data: {
      items: [
        { id: 'b-1', name: 'Hlavná 1' },
        { id: 'b-2', name: 'Dlhá 2' },
      ],
    },
    isLoading: false,
  }),
  useMyUnits: () => ({ data: [], isLoading: false }),
  useCreateVote: () => ({ mutateAsync: h.createVote, isPending: false }),
  useDeleteVote: () => ({ mutateAsync: vi.fn(async () => undefined), isPending: false }),
  addQuestion: vi.fn(async () => ({})),
  publishVote: vi.fn(async () => ({})),
}));

function fillAndSave(container: HTMLElement) {
  const [title] = screen.getAllByRole('textbox');
  fireEvent.change(title, { target: { value: 'Roof repair' } });
  const question = container.querySelector('input[placeholder="Question text"]') as HTMLElement;
  fireEvent.change(question, { target: { value: 'Approve?' } });
  const endAt = container.querySelector('input[type="datetime-local"]') as HTMLElement;
  fireEvent.change(endAt, { target: { value: '2030-01-01T10:00' } });
  fireEvent.click(screen.getByRole('button', { name: 'Save draft' }));
}

function renderAt(url: string, onCreated = vi.fn()) {
  const utils = render(
    <MemoryRouter initialEntries={[url]}>
      <ToastProvider>
        <VoteCreatePage onCreated={onCreated} onCancel={() => {}} />
      </ToastProvider>
    </MemoryRouter>
  );
  return { ...utils, onCreated };
}

beforeEach(() => h.createVote.mockClear());

describe('VoteCreatePage building selection', () => {
  it('creates the vote for the building in ?building=', async () => {
    const { container, onCreated } = renderAt('/voting/new?building=b-2');
    expect(screen.getByLabelText('Building')).toHaveValue('b-2');
    fillAndSave(container);
    await waitFor(() => expect(onCreated).toHaveBeenCalledWith('v-1'));
    expect(h.createVote.mock.calls[0][0]).toMatchObject({ building_id: 'b-2' });
  });

  it('creates the vote for the building picked in the selector', async () => {
    const { container, onCreated } = renderAt('/voting/new');
    fireEvent.change(screen.getByLabelText('Building'), { target: { value: 'b-2' } });
    fillAndSave(container);
    await waitFor(() => expect(onCreated).toHaveBeenCalled());
    expect(h.createVote.mock.calls[0][0]).toMatchObject({ building_id: 'b-2' });
  });
});
