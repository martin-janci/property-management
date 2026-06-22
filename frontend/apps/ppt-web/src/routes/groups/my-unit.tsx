/**
 * My Unit route group (Epic 3, Story 3.6).
 *
 * Owns the resident-facing `/my-unit` route. The page is self-contained — it
 * resolves the caller's own units from `GET /api/v1/users/me/units` via
 * `useMyUnits()` — so the wrapper only has to mount the lazy component.
 */
import { Route } from 'react-router-dom';
import { MyUnitPage } from '../lazyRoutes';

/** Resident "My Unit" routes (Story 3.6). */
export function myUnitRoutes() {
  return <Route path="/my-unit" element={<MyUnitPage />} />;
}
