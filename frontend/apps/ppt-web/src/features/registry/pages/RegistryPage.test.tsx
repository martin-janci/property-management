/// <reference types="vitest/globals" />
/**
 * RegistryPage destructive-action feedback regression tests.
 *
 * History: the pet/vehicle delete and review handlers caught a failed
 * `mutateAsync` and only called `console.error` — no user-visible side effect.
 * A failed destructive action (delete / approve) therefore looked identical to
 * a successful one, and there was no success confirmation either.
 *
 * These tests lock in the ppt-web convention (useToast), matching the sibling
 * RegistryRulesPage: each handler shows a success Toast on resolve and an
 * error Toast (surfacing the server's message) on reject. They fail on the
 * pre-fix version because no toast ever appears.
 */
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ToastProvider } from '../../../components';
import { RegistryPage } from './RegistryPage';

const deletePet = vi.fn();
const reviewPet = vi.fn();
const deleteVehicle = vi.fn();
const reviewVehicle = vi.fn();

vi.mock('@ppt/api-client', () => ({
  createRegistryApi: vi.fn(() => ({})),
  createRegistryHooks: vi.fn(() => ({
    usePetsList: vi.fn(() => ({
      data: { items: [], total: 0, page: 1, pageSize: 10 },
      isLoading: false,
      error: null,
    })),
    useVehiclesList: vi.fn(() => ({
      data: { items: [], total: 0, page: 1, pageSize: 10 },
      isLoading: false,
      error: null,
    })),
    useDeletePet: vi.fn(() => ({ mutateAsync: deletePet, isPending: false })),
    useReviewPet: vi.fn(() => ({ mutateAsync: reviewPet, isPending: false })),
    useDeleteVehicle: vi.fn(() => ({ mutateAsync: deleteVehicle, isPending: false })),
    useReviewVehicle: vi.fn(() => ({ mutateAsync: reviewVehicle, isPending: false })),
  })),
}));

// Replace the list components with minimal stubs that expose the destructive
// callbacks as plain buttons, so the tests drive the page's handlers directly
// without coupling to the list internals.
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
      <button type="button" onClick={() => onDelete('veh-1')}>
        delete-vehicle
      </button>
      <button type="button" onClick={() => onReview('veh-1')}>
        review-vehicle
      </button>
    </div>
  ),
}));

function renderPage() {
  return render(
    <ToastProvider>
      <RegistryPage />
    </ToastProvider>
  );
}

function alerts() {
  return screen.queryAllByRole('alert');
}

function hasToast(pattern: RegExp) {
  return alerts().some((el) => pattern.test(el.textContent ?? ''));
}

/** Click a list action button, then confirm in the confirmation dialog. */
async function clickAndConfirm(buttonName: RegExp, confirmName: RegExp) {
  fireEvent.click(screen.getByRole('button', { name: buttonName }));
  const confirmBtn = await screen.findByRole('button', { name: confirmName });
  fireEvent.click(confirmBtn);
}

describe('RegistryPage destructive-action feedback', () => {
  beforeEach(() => {
    deletePet.mockReset();
    reviewPet.mockReset();
    deleteVehicle.mockReset();
    reviewVehicle.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('surfaces an error toast (with the server message) when deleting a pet fails', async () => {
    deletePet.mockRejectedValueOnce(new Error('Internal server error'));

    renderPage();
    await clickAndConfirm(/^delete-pet$/, /^delete$/i);

    await waitFor(() => expect(hasToast(/failed to delete pet registration/i)).toBe(true));
    expect(hasToast(/internal server error/i)).toBe(true);
    expect(hasToast(/pet registration deleted/i)).toBe(false);
  });

  it('shows a success toast when deleting a pet succeeds', async () => {
    deletePet.mockResolvedValueOnce(undefined);

    renderPage();
    await clickAndConfirm(/^delete-pet$/, /^delete$/i);

    await waitFor(() => expect(hasToast(/pet registration deleted/i)).toBe(true));
    expect(hasToast(/failed to delete pet registration/i)).toBe(false);
    expect(deletePet).toHaveBeenCalledWith('pet-1');
  });

  it('surfaces an error toast when approving a pet fails', async () => {
    reviewPet.mockRejectedValueOnce(new Error('Approval rejected by server'));

    renderPage();
    await clickAndConfirm(/^review-pet$/, /^approve$/i);

    await waitFor(() => expect(hasToast(/failed to approve pet registration/i)).toBe(true));
    expect(hasToast(/approval rejected by server/i)).toBe(true);
    expect(hasToast(/pet registration approved/i)).toBe(false);
  });

  it('shows a success toast when approving a pet succeeds', async () => {
    reviewPet.mockResolvedValueOnce(undefined);

    renderPage();
    await clickAndConfirm(/^review-pet$/, /^approve$/i);

    await waitFor(() => expect(hasToast(/pet registration approved/i)).toBe(true));
    expect(reviewPet).toHaveBeenCalledWith({ id: 'pet-1', data: { approve: true } });
  });

  it('surfaces an error toast when deleting a vehicle fails', async () => {
    deleteVehicle.mockRejectedValueOnce(new Error('Conflict'));

    renderPage();
    fireEvent.click(screen.getByRole('button', { name: /vehicles/i }));
    await clickAndConfirm(/^delete-vehicle$/, /^delete$/i);

    await waitFor(() => expect(hasToast(/failed to delete vehicle registration/i)).toBe(true));
    expect(hasToast(/conflict/i)).toBe(true);
    expect(hasToast(/vehicle registration deleted/i)).toBe(false);
  });

  it('surfaces an error toast when approving a vehicle fails', async () => {
    reviewVehicle.mockRejectedValueOnce(new Error('Not allowed'));

    renderPage();
    fireEvent.click(screen.getByRole('button', { name: /vehicles/i }));
    await clickAndConfirm(/^review-vehicle$/, /^approve$/i);

    await waitFor(() => expect(hasToast(/failed to approve vehicle registration/i)).toBe(true));
    expect(hasToast(/not allowed/i)).toBe(true);
    expect(hasToast(/vehicle registration approved/i)).toBe(false);
  });
});
