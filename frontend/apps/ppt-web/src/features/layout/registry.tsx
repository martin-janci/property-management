import type { ResolvedScreen } from '@ppt/api-client';
import type { ComponentType } from 'react';
import { ActionQueue } from '../dashboard/components/ActionQueue';
import { DashboardStats } from '../dashboard/components/DashboardStats';

export interface SectionProps {
  mode?: string;
  props?: Record<string, unknown>;
}

export interface SectionDef {
  component: ComponentType<SectionProps>;
  required: boolean;
  supportedModes: string[];
}

export type SectionRegistry = Record<string, SectionDef>;

/** ppt-web dashboard sections. */
export const dashboardRegistry: SectionRegistry = {
  'dashboard-stats.v1': { component: DashboardStats, required: true, supportedModes: [] },
  'action-queue.v1': {
    component: () => <ActionQueue userRole="manager" />,
    required: true,
    supportedModes: [],
  },
};

/** Rendered when the layout endpoint is unavailable (spec §4: never gate the
 *  page on layout). */
export const DEFAULT_DASHBOARD_LAYOUT: ResolvedScreen = {
  screen: 'ppt/dashboard',
  version: 0,
  sections: [
    { type: 'dashboard-stats.v1', presentation: 'visible' },
    { type: 'action-queue.v1', presentation: 'visible' },
  ],
};

/** The registry manifest for upload to PUT /platform-admin/layout/manifests.
 *  Kept in manifest.json; the manifest.test.ts asserts it mirrors the registry. */
export function registryManifest(registry: SectionRegistry) {
  return {
    platform: 'web',
    components: Object.fromEntries(
      Object.entries(registry).map(([type, def]) => [
        type,
        {
          required: def.required,
          ...(def.supportedModes.length > 0
            ? { supported_modes: def.supportedModes, default_mode: def.supportedModes[0] }
            : {}),
        },
      ])
    ),
  };
}
