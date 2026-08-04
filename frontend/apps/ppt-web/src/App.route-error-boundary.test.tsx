/**
 * Route-outlet ErrorBoundary regression coverage.
 *
 * Guards the fix for the "core route error-boundary gap": before it, ppt-web
 * had no boundary between the route outlet and the application shell, so a
 * single stale-chunk `lazy()` rejection (or any uncaught render error in one
 * route) propagated to the top-level `<ErrorBoundary>` in `main.tsx` — which
 * wraps the entire `<App />` — and unmounted the whole shell (nav included).
 *
 * `RouteErrorBoundary` (App.tsx) is rendered as a child of `<main>` and a
 * sibling *below* `<AppNavigation>`. These tests reproduce that arrangement
 * (a shell sibling outside the boundary + a route outlet inside it) and prove:
 *   1. a route that throws during render shows the scoped fallback while the
 *      shell survives;
 *   2. a stale-chunk `lazy()` rejection is caught the same way (the shell
 *      survives, the fallback with a reload affordance renders);
 *   3. navigating to another route after a failure recovers automatically
 *      (the boundary is keyed on pathname) — no full page reload required.
 */
/// <reference types="vitest/globals" />

import { fireEvent, render, screen } from '@testing-library/react';
import { type ComponentType, lazy, Suspense } from 'react';
import { Link, MemoryRouter, Route, Routes } from 'react-router-dom';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { RouteErrorBoundary } from './App';

// React logs caught render errors to console.error; silence it so the
// expected-failure tests don't spam the reporter.
beforeEach(() => {
  vi.spyOn(console, 'error').mockImplementation(() => {});
});
afterEach(() => {
  vi.restoreAllMocks();
});

/** A route component that always throws during render. */
function ThrowingRoute(): never {
  throw new Error('boom: route render failure');
}

const SHELL = 'app-shell-nav';
const FALLBACK_TITLE = 'Something went wrong'; // errors.somethingWentWrong (en)
const RELOAD_LABEL = /reload page/i; // common.reloadPage (en)

describe('RouteErrorBoundary', () => {
  it('renders the scoped fallback for a throwing route while the shell survives', () => {
    render(
      <MemoryRouter initialEntries={['/boom']}>
        {/* Shell sibling — mirrors <AppNavigation> living outside <main>. */}
        <nav>{SHELL}</nav>
        <main>
          <RouteErrorBoundary>
            <Routes>
              <Route path="/boom" element={<ThrowingRoute />} />
            </Routes>
          </RouteErrorBoundary>
        </main>
      </MemoryRouter>
    );

    // Fallback is shown inside the content region…
    expect(screen.getByRole('alert')).toBeInTheDocument();
    expect(screen.getByText(FALLBACK_TITLE)).toBeInTheDocument();
    // …with a reload affordance…
    expect(screen.getByRole('button', { name: RELOAD_LABEL })).toBeInTheDocument();
    // …and the shell is still mounted (not unmounted with the app).
    expect(screen.getByText(SHELL)).toBeInTheDocument();
  });

  it('catches a stale-chunk lazy() rejection without unmounting the shell', async () => {
    // Simulates a deploy that invalidated the old chunk: the dynamic import
    // rejects instead of resolving to a module.
    const StaleChunkRoute = lazy(
      () =>
        Promise.reject(new Error('Failed to fetch dynamically imported module')) as Promise<{
          default: ComponentType;
        }>
    );

    render(
      <MemoryRouter initialEntries={['/stale']}>
        <nav>{SHELL}</nav>
        <main>
          <RouteErrorBoundary>
            <Suspense fallback={<div>loading route…</div>}>
              <Routes>
                <Route path="/stale" element={<StaleChunkRoute />} />
              </Routes>
            </Suspense>
          </RouteErrorBoundary>
        </main>
      </MemoryRouter>
    );

    // The rejection resolves asynchronously; the boundary then renders the
    // fallback rather than letting the error escape to the app root.
    expect(await screen.findByText(FALLBACK_TITLE)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: RELOAD_LABEL })).toBeInTheDocument();
    // Shell survives the lazy failure.
    expect(screen.getByText(SHELL)).toBeInTheDocument();
  });

  it('recovers automatically when the user navigates to another route', () => {
    render(
      <MemoryRouter initialEntries={['/boom']}>
        {/* In-shell nav link — clicking it changes the pathname, which the
            boundary is keyed on, so it resets and renders the new route. */}
        <Link to="/safe">go-safe</Link>
        <main>
          <RouteErrorBoundary>
            <Routes>
              <Route path="/boom" element={<ThrowingRoute />} />
              <Route path="/safe" element={<div>safe-route-content</div>} />
            </Routes>
          </RouteErrorBoundary>
        </main>
      </MemoryRouter>
    );

    // Start on the failing route: fallback visible.
    expect(screen.getByText(FALLBACK_TITLE)).toBeInTheDocument();

    // Navigate away — the pathname-keyed boundary remounts and clears.
    fireEvent.click(screen.getByText('go-safe'));

    expect(screen.getByText('safe-route-content')).toBeInTheDocument();
    expect(screen.queryByText(FALLBACK_TITLE)).not.toBeInTheDocument();
  });
});
