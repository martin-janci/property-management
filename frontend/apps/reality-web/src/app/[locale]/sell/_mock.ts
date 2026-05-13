/**
 * Static (locale-agnostic) data for the sell-wizard.
 *
 * Localized labels for steps, transaction options and property options
 * live in `pages.sell.*` in `messages/<locale>.json` and are read inside
 * `page.tsx` via `useTranslations`. Keep this file free of human-readable
 * strings so the wizard works correctly under every supported locale.
 */

export interface SellStep {
  id: number;
  /** Key under `pages.sell.steps.*` */
  key: 'type' | 'details' | 'photos' | 'price' | 'contact';
}

export const SELL_STEPS: SellStep[] = [
  { id: 1, key: 'type' },
  { id: 2, key: 'details' },
  { id: 3, key: 'photos' },
  { id: 4, key: 'price' },
  { id: 5, key: 'contact' },
];

export type PropertyType = 'apartment' | 'house' | 'land' | 'commercial';
export type TransactionType = 'sale' | 'rent';

export interface SellFormData {
  transactionType: TransactionType;
  propertyType: PropertyType;
  address: string;
  city: string;
  area: number | '';
  rooms: number | '';
  floor: number | '';
  totalFloors: number | '';
  yearBuilt: number | '';
  description: string;
  photos: File[];
  price: number | '';
  currency: string;
  priceNegotiable: boolean;
  contactName: string;
  contactPhone: string;
  contactEmail: string;
  termsAccepted: boolean;
}

export const INITIAL_FORM_DATA: SellFormData = {
  transactionType: 'sale',
  propertyType: 'apartment',
  address: '',
  city: '',
  area: '',
  rooms: '',
  floor: '',
  totalFloors: '',
  yearBuilt: '',
  description: '',
  photos: [],
  price: '',
  currency: 'EUR',
  priceNegotiable: false,
  contactName: '',
  contactPhone: '',
  contactEmail: '',
  termsAccepted: false,
};

export const PROPERTY_TYPES: PropertyType[] = ['apartment', 'house', 'land', 'commercial'];
