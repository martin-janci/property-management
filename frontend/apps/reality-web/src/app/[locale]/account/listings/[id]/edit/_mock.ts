/**
 * Listing-edit wizard static (locale-agnostic) data.
 *
 * Mirrors sell/_mock: human-readable labels for steps, transaction options
 * and property options live in `pages.listingEdit.*` in `messages/<locale>.json`
 * and are read inside `page.tsx` via `useTranslations`. Keep this file free of
 * user-facing strings so the wizard works under every supported locale.
 */

export interface EditStep {
  id: number;
  /** Key under `pages.listingEdit.steps.*` */
  key: 'type' | 'details' | 'photos' | 'price' | 'summary';
}

export const EDIT_STEPS: EditStep[] = [
  { id: 1, key: 'type' },
  { id: 2, key: 'details' },
  { id: 3, key: 'photos' },
  { id: 4, key: 'price' },
  { id: 5, key: 'summary' },
];

export type PropertyType = 'apartment' | 'house' | 'land' | 'commercial';
export type TransactionType = 'sale' | 'rent';

export interface ListingEditFormData {
  transactionType: TransactionType;
  propertyType: PropertyType;
  address: string;
  city: string;
  postalCode: string;
  country: string;
  area: number | '';
  rooms: number | '';
  floor: number | '';
  totalFloors: number | '';
  yearBuilt: number | '';
  description: string;
  existingPhotoUrls: string[];
  newPhotos: File[];
  price: number | '';
  currency: string;
  priceNegotiable: boolean;
  status: 'active' | 'inactive' | 'draft';
}

/** Selectable property types, in display order. Labels come from `pages.listingEdit.propertyOptions.*`. */
export const PROPERTY_TYPES: PropertyType[] = ['apartment', 'house', 'land', 'commercial'];
