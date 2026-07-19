'use client';

import type { ListingSectionProps } from './registry';
import { ContactForm } from '../ContactForm';

export function AgentContact({ listing }: ListingSectionProps) {
  if (!listing.agent) return null;
  return <ContactForm listingId={listing.id} agent={listing.agent} />;
}
