import { describe, expect, it } from 'vitest';
import manifest from './manifest.json';
import { dashboardRegistry, registryManifest } from './registry';

describe('layout manifest', () => {
  it('mirrors the dashboard registry exactly', () => {
    expect(manifest).toEqual(registryManifest(dashboardRegistry));
  });
});
