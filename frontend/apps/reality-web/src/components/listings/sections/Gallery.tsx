'use client';

import type { ListingSectionProps } from './registry';
import { PhotoGallery } from '../PhotoGallery';

export function Gallery({ listing }: ListingSectionProps) {
  return <PhotoGallery photos={listing.photos} title={listing.title} />;
}
