'use client';

import { PhotoGallery } from '../PhotoGallery';
import type { ListingSectionProps } from './registry';

export function Gallery({ listing }: ListingSectionProps) {
  return <PhotoGallery photos={listing.photos} title={listing.title} />;
}
