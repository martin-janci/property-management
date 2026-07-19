import type { ListingDetail } from '@ppt/reality-api-client';
import type { ComponentType } from 'react';

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

/**
 * Registry of listing-detail section components.
 * Intentionally empty — Task 6 fills in the actual section implementations.
 */
export const listingRegistry: ListingRegistry = {};
