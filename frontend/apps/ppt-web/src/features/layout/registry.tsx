import type { ComponentType } from 'react';
import type { ResolvedScreen } from '@ppt/api-client';

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

/** ppt-web dashboard sections — populated by the dashboard feature (Task 3). */
export const dashboardRegistry: SectionRegistry = {};

/** Rendered when the layout endpoint is unavailable (spec §4: never gate the
 *  page on layout). Task 3 fills the real section list. */
export const DEFAULT_DASHBOARD_LAYOUT: ResolvedScreen = {
  screen: 'ppt/dashboard',
  version: 0,
  sections: [],
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
      ]),
    ),
  };
}
