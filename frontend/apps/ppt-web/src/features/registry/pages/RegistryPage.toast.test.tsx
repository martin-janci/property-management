/// <reference types="vitest/globals" />
/**
 * RegistryPage destructive-action toast regression tests.
 *
 * History: the four destructive handlers on RegistryPage — handlePetDelete,
 * handlePetReview, handleVehicleDelete, handleVehicleReview — swallowed
 * mutation failures with only `console.error(...)`. A rejected delete/review
 * therefore looked identical to a success: no feedback at all, and certainly
 * no surfaced server message. See the sibling RegistryRulesPage.handleSubmit
 * for the convention these tests lock in: a success toast on resolve and a
 * persistent error toast carrying the server message on reject.
 *
 * These tests fail on the pre-fix handlers (which never call `showToast`) and
 * pass once each handler surfaces its outcome via `useToast()`.
 *
 * Kept deliberately separate from RegistryPage.test.tsx (the auth-context
 * suite) so each file owns its own self-contained module mocks.
 */
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// --- Controllable mutation spies -------------------------------------------
const mutations = vi.hoisted(() => ({
  deletePet: vi.fn<[string], Promise<unknown>>(() => Promise.resolve({})),
  reviewPet: vi.fn<[unknown], Promise<unknown>>(() => Promise.resolve({})),
  deleteVehicle: vi.fn<[string], Promise<unknown>>(() => Promise.resolve({})),
  reviewVehicle: vi.fn<[unknown], Promise<unknown>>(() => Promise.resolve({})),
}));

const showToast = vi.hoisted(() => vi.fn());

// Registry hooks: stub queries + mutateAsync wired to the controllable spies.
vi.mock('../hooks/useRegistry', () => ({
  useRegistryApi: () => ({
    usePetsList: () => ({ data: undefined, isLoading: false, error: null }),
    useVehiclesList: () => ({ data: undefined, isLoading: false, error: null }),
    useDeletePet: () => ({ mutateAsync: mutations.deletePet, isPending: false }),
    useReviewPet: () => ({ mutateAsync: mutations.reviewPet, isPending: false }),
    useDeleteVehicle: () => ({ mutateAsync: mutations.deleteVehicle, isPending: false }),
    useReviewVehicle: () => ({ mutateAsync: mutations.reviewVehicle, isPending: false }),
  }),
}));

// Toast: capture the calls. RegistryPage uses the provider-tolerant
// `useToastOptional` (so it also renders under the auth-context suite, which
// mounts it without a ToastProvider); the real app-level provider supplies it
// in production.
vi.mock('../../../components', () => ({
  useToastOptional: () => ({ showToast, removeToast: vi.fn() }),
}));

// Confirm dialog: auto-confirm every destructive action.
vi.mock('../../../components/useConfirmDialog', () => ({
  useConfirmDialog: () => ({
    confirm: () => Promise.resolve(true),
    confirmDialog: null,
  }),
}));

// List components: render minimal buttons that fire the page handlers.
vi.mock('../components/PetRegistrationList', () => ({
  PetRegistrationList: ({
    onDelete,
    onReview,
  }: {
    onDelete: (id: string) => void;
    onReview: (id: string) => void;
  }) => (
    <div>
      <button type="button" onClick={() => onDelete('pet-1')}>
        delete-pet
      </button>
      <button type="button" onClick={() => onReview('pet-1')}>
        review-pet
      </button>
    </div>
  ),
}));

vi.mock('../components/VehicleRegistrationList', () => ({
  VehicleRegistrationList: ({
    onDelete,
    onReview,
  }: {
    onDelete: (id: string) => void;
    onReview: (id: string) => void;
  }) => (
    <div>
      <button type="button" onClick={() => onDelete('vehicle-1')}>
        delete-vehicle
      </button>
      <button type="button" onClick={() => onReview('vehicle-1')}>
        review-vehicle
      </button>
    </div>
  ),
}));

import { RegistryPage } from './RegistryPage';

function switchToVehiclesTab() {
  // The second tab button toggles the vehicles list into view.
  fireEvent.click(screen.getByRole('button', { name: /vehicles/i }));
}

beforeEach(() => {
  mutations.deletePet.mockResolvedValue({});
  mutations.reviewPet.mockResolvedValue({});
  mutations.deleteVehicle.mockResolvedValue({});
  mutations.reviewVehicle.mockResolvedValue({});
});

afterEach(() => {
  vi.clearAllMocks();
});

describe('RegistryPage destructive-action toasts', () => {
  it('shows a success toast when a pet delete resolves', async () => {
    render(<RegistryPage />);
    fireEvent.click(screen.getByText('delete-pet'));

    await waitFor(() => expect(showToast).toHaveBeenCalled());
    expect(showToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'success' }));
  });

  it('shows an error toast carrying the server message when a pet delete rejects', async () => {
    mutations.deletePet.mockRejectedValue(new Error('server said no (pet delete)'));
    render(<RegistryPage />);
    fireEvent.click(screen.getByText('delete-pet'));

    await waitFor(() => expect(showToast).toHaveBeenCalled());
    expect(showToast).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'error', message: 'server said no (pet delete)' })
    );
  });

  it('shows a success toast when a pet review resolves', async () => {
    render(<RegistryPage />);
    fireEvent.click(screen.getByText('review-pet'));

    await waitFor(() => expect(showToast).toHaveBeenCalled());
    expect(showToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'success' }));
  });

  it('shows an error toast carrying the server message when a pet review rejects', async () => {
    mutations.reviewPet.mockRejectedValue(new Error('server said no (pet review)'));
    render(<RegistryPage />);
    fireEvent.click(screen.getByText('review-pet'));

    await waitFor(() => expect(showToast).toHaveBeenCalled());
    expect(showToast).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'error', message: 'server said no (pet review)' })
    );
  });

  it('shows a success toast when a vehicle delete resolves', async () => {
    render(<RegistryPage />);
    switchToVehiclesTab();
    fireEvent.click(screen.getByText('delete-vehicle'));

    await waitFor(() => expect(showToast).toHaveBeenCalled());
    expect(showToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'success' }));
  });

  it('shows an error toast carrying the server message when a vehicle delete rejects', async () => {
    mutations.deleteVehicle.mockRejectedValue(new Error('server said no (vehicle delete)'));
    render(<RegistryPage />);
    switchToVehiclesTab();
    fireEvent.click(screen.getByText('delete-vehicle'));

    await waitFor(() => expect(showToast).toHaveBeenCalled());
    expect(showToast).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'error', message: 'server said no (vehicle delete)' })
    );
  });

  it('shows a success toast when a vehicle review resolves', async () => {
    render(<RegistryPage />);
    switchToVehiclesTab();
    fireEvent.click(screen.getByText('review-vehicle'));

    await waitFor(() => expect(showToast).toHaveBeenCalled());
    expect(showToast).toHaveBeenCalledWith(expect.objectContaining({ type: 'success' }));
  });

  it('shows an error toast carrying the server message when a vehicle review rejects', async () => {
    mutations.reviewVehicle.mockRejectedValue(new Error('server said no (vehicle review)'));
    render(<RegistryPage />);
    switchToVehiclesTab();
    fireEvent.click(screen.getByText('review-vehicle'));

    await waitFor(() => expect(showToast).toHaveBeenCalled());
    expect(showToast).toHaveBeenCalledWith(
      expect.objectContaining({ type: 'error', message: 'server said no (vehicle review)' })
    );
  });
});
