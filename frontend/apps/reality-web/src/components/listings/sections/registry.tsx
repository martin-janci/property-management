import type { ListingDetail } from '@ppt/reality-api-client';
import type { ComponentType } from 'react';
import { AdditionalInfo } from './AdditionalInfo';
import { AgentContact } from './AgentContact';
import { Description } from './Description';
import { Features } from './Features';
import { Gallery } from './Gallery';
import { KeyDetails } from './KeyDetails';
import { ListingHeader } from './ListingHeader';
import { Resources } from './Resources';

export interface ListingSectionProps {
  listing: ListingDetail;
  mode?: string;
  props?: Record<string, unknown>;
}

export interface ListingSectionDef {
  component: ComponentType<ListingSectionProps>;
  required: boolean;
  supportedModes: string[];
}

export type ListingRegistry = Record<string, ListingSectionDef>;

export const listingRegistry: ListingRegistry = {
  'gallery.v1': { component: Gallery, required: true, supportedModes: [] },
  'listing-header.v1': { component: ListingHeader, required: true, supportedModes: [] },
  'key-details.v1': { component: KeyDetails, required: false, supportedModes: [] },
  'description.v1': { component: Description, required: false, supportedModes: [] },
  'features.v1': { component: Features, required: false, supportedModes: [] },
  'additional-info.v1': { component: AdditionalInfo, required: false, supportedModes: [] },
  'resources.v1': { component: Resources, required: false, supportedModes: [] },
  'agent-contact.v1': { component: AgentContact, required: false, supportedModes: [] },
};

/** The registry manifest for upload to PUT /platform-admin/layout/manifests.
 *  Kept in layout-manifest.json; layout.test.ts asserts it mirrors the registry. */
export function registryManifest(registry: ListingRegistry) {
  return {
    platform: 'web',
    components: Object.fromEntries(
      Object.entries(registry).map(([type, def]) => {
        const supportedModes = def.supportedModes ?? [];
        return [
          type,
          {
            required: def.required,
            ...(supportedModes.length > 0
              ? { supported_modes: supportedModes, default_mode: supportedModes[0] }
              : {}),
          },
        ];
      })
    ),
  };
}
