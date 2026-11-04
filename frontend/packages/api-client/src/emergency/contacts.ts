/**
 * Emergency Contacts API client for Epic 62.
 */

import { authenticatedFetch } from '../lib/fetch';
import type {
  CreateEmergencyContact,
  EmergencyContact,
  EmergencyContactQuery,
  UpdateEmergencyContact,
} from './types';

const API_BASE = '/api/v1/emergency';

/**
 * List emergency contacts with optional filters.
 */
export async function listEmergencyContacts(
  query: EmergencyContactQuery
): Promise<EmergencyContact[]> {
  const params = new URLSearchParams();
  params.append('organization_id', query.organization_id);

  if (query.building_id) params.append('building_id', query.building_id);
  if (query.contact_type) params.append('contact_type', query.contact_type);
  if (query.is_active !== undefined) params.append('is_active', String(query.is_active));
  if (query.limit) params.append('limit', String(query.limit));
  if (query.offset) params.append('offset', String(query.offset));

  const response = await authenticatedFetch(`${API_BASE}/contacts?${params.toString()}`);

  if (!response.ok) {
    throw new Error('Unable to load emergency contacts. Please try again later.');
  }

  return response.json();
}

/**
 * Get a single emergency contact by ID.
 */
export async function getEmergencyContact(
  id: string,
  organizationId: string
): Promise<EmergencyContact> {
  const params = new URLSearchParams();
  params.append('organization_id', organizationId);

  const response = await authenticatedFetch(`${API_BASE}/contacts/${id}?${params.toString()}`);

  if (!response.ok) {
    if (response.status === 404) {
      throw new Error('Emergency contact not found');
    }
    throw new Error('Unable to load emergency contact. Please try again later.');
  }

  return response.json();
}

/**
 * Create a new emergency contact.
 */
export async function createEmergencyContact(
  organizationId: string,
  data: CreateEmergencyContact
): Promise<EmergencyContact> {
  const response = await authenticatedFetch(`${API_BASE}/contacts`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      organization_id: organizationId,
      ...data,
    }),
  });

  if (!response.ok) {
    throw new Error('Unable to create emergency contact. Please check your input and try again.');
  }

  return response.json();
}

/**
 * Update an existing emergency contact.
 */
export async function updateEmergencyContact(
  id: string,
  organizationId: string,
  data: UpdateEmergencyContact
): Promise<EmergencyContact> {
  const response = await authenticatedFetch(`${API_BASE}/contacts/${id}`, {
    method: 'PUT',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      organization_id: organizationId,
      ...data,
    }),
  });

  if (!response.ok) {
    if (response.status === 404) {
      throw new Error('Emergency contact not found');
    }
    throw new Error('Unable to update emergency contact. Please check your input and try again.');
  }

  return response.json();
}

/**
 * Delete an emergency contact.
 */
export async function deleteEmergencyContact(id: string, organizationId: string): Promise<void> {
  const params = new URLSearchParams();
  params.append('organization_id', organizationId);

  const response = await authenticatedFetch(`${API_BASE}/contacts/${id}?${params.toString()}`, {
    method: 'DELETE',
  });

  if (!response.ok) {
    if (response.status === 404) {
      throw new Error('Emergency contact not found');
    }
    throw new Error('Unable to delete emergency contact. Please try again later.');
  }
}
