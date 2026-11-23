/// <reference types="vitest/globals" />
/**
 * RegistryPage SPA-navigation regression tests.
 *
 * History: the six navigation handlers on RegistryPage — handlePetView,
 * handlePetEdit, handlePetCreate, handleVehicleView, handleVehicleEdit,
 * handleVehicleCreate — used `window.location.href = …`, a full-page reload
 * that drops client state (active tab, filters, scroll, React Query cache) and
 * flashes the app shell. See the sibling RegistryRulesPage and
 * ContentModerationPage for the convention these tests lock in: route via
 * react-router's `useNavigate`.
 *
 * These tests fail on the pre-fix handlers (which never call `navigate`) and
 * pass once each handler routes through the SPA router.
 */
import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// Observe navigation without needing a real <Router>.
const navigateSpy = vi.fn();
vi.mock('react-router-dom', async (importActual) => {
  const actual = await importActual<typeof import('react-router-dom')>();
  return { ...actual, useNavigate: () => navigateSpy };
});

// Registry hooks: stub queries + no-op mutations (not exercised here).
vi.mock('../hooks/useRegistry', () => ({
  useRegistryApi: () => ({
    usePetsList: () => ({ data: undefined, isLoading: false, error: null }),
    useVehiclesList: () => ({ data: undefined, isLoading: false, error: null }),
    useDeletePet: () => ({ mutateAsync: vi.fn(), isPending: false }),
    useReviewPet: () => ({ mutateAsync: vi.fn(), isPending: false }),
    useDeleteVehicle: () => ({ mutateAsync: vi.fn(), isPending: false }),
    useReviewVehicle: () => ({ mutateAsync: vi.fn(), isPending: false }),
  }),
}));

vi.mock('../../../components', () => ({
  useToastOptional: () => ({ showToast: vi.fn(), removeToast: vi.fn() }),
}));

vi.mock('../../../components/useConfirmDialog', () => ({
  useConfirmDialog: () => ({ confirm: () => Promise.resolve(true), confirmDialog: null }),
}));

// List components: render minimal buttons that fire the navigation handlers.
vi.mock('../components/PetRegistrationList', () => ({
  PetRegistrationList: ({
    onView,
    onEdit,
    onCreate,
  }: {
    onView: (id: string) => void;
    onEdit: (id: string) => void;
    onCreate: () => void;
  }) => (
    <div>
      <button type="button" onClick={() => onView('pet-1')}>
        view-pet
      </button>
      <button type="button" onClick={() => onEdit('pet-1')}>
        edit-pet
      </button>
      <button type="button" onClick={() => onCreate()}>
        create-pet
      </button>
    </div>
  ),
}));

vi.mock('../components/VehicleRegistrationList', () => ({
  VehicleRegistrationList: ({
    onView,
    onEdit,
    onCreate,
  }: {
    onView: (id: string) => void;
    onEdit: (id: string) => void;
    onCreate: () => void;
  }) => (
    <div>
      <button type="button" onClick={() => onView('vehicle-1')}>
        view-vehicle
      </button>
      <button type="button" onClick={() => onEdit('vehicle-1')}>
        edit-vehicle
      </button>
      <button type="button" onClick={() => onCreate()}>
        create-vehicle
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
  navigateSpy.mockClear();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe('RegistryPage SPA navigation (not a full-page reload)', () => {
  it('routes to the pet detail view via the SPA router', () => {
    render(<RegistryPage />);
    fireEvent.click(screen.getByText('view-pet'));

    expect(navigateSpy).toHaveBeenCalledTimes(1);
    expect(navigateSpy).toHaveBeenCalledWith('/registry/pets/pet-1');
  });

  it('routes to the pet edit view via the SPA router', () => {
    render(<RegistryPage />);
    fireEvent.click(screen.getByText('edit-pet'));

    expect(navigateSpy).toHaveBeenCalledTimes(1);
    expect(navigateSpy).toHaveBeenCalledWith('/registry/pets/pet-1/edit');
  });

  it('routes to the new-pet form via the SPA router', () => {
    render(<RegistryPage />);
    fireEvent.click(screen.getByText('create-pet'));

    expect(navigateSpy).toHaveBeenCalledTimes(1);
    expect(navigateSpy).toHaveBeenCalledWith('/registry/pets/new');
  });

  it('routes to the vehicle detail view via the SPA router', () => {
    render(<RegistryPage />);
    switchToVehiclesTab();
    fireEvent.click(screen.getByText('view-vehicle'));

    expect(navigateSpy).toHaveBeenCalledTimes(1);
    expect(navigateSpy).toHaveBeenCalledWith('/registry/vehicles/vehicle-1');
  });

  it('routes to the vehicle edit view via the SPA router', () => {
    render(<RegistryPage />);
    switchToVehiclesTab();
    fireEvent.click(screen.getByText('edit-vehicle'));

    expect(navigateSpy).toHaveBeenCalledTimes(1);
    expect(navigateSpy).toHaveBeenCalledWith('/registry/vehicles/vehicle-1/edit');
  });

  it('routes to the new-vehicle form via the SPA router', () => {
    render(<RegistryPage />);
    switchToVehiclesTab();
    fireEvent.click(screen.getByText('create-vehicle'));

    expect(navigateSpy).toHaveBeenCalledTimes(1);
    expect(navigateSpy).toHaveBeenCalledWith('/registry/vehicles/new');
  });
});
