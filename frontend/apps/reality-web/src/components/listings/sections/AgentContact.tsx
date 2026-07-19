'use client';

import { ContactForm } from '../ContactForm';
import type { ListingSectionProps } from './registry';

export function AgentContact({ listing }: ListingSectionProps) {
  if (!listing.agent) return null;
  return <ContactForm listingId={listing.id} agent={listing.agent} />;
}
