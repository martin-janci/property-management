/**
 * Listing-edit wizard mock data.
 * Mirrors sell/_mock but with pre-populated values from a fetched listing.
 */

export interface EditStep {
  id: number;
  title: string;
  description: string;
}

export const EDIT_STEPS: EditStep[] = [
  { id: 1, title: 'Typ a poloha', description: 'Typ nehnuteľnosti a adresa' },
  { id: 2, title: 'Detaily', description: 'Plocha, izby, poschodie' },
  { id: 3, title: 'Fotografie', description: 'Spravovať fotografie' },
  { id: 4, title: 'Cena', description: 'Aktualizovať požadovanú cenu' },
  { id: 5, title: 'Súhrn', description: 'Skontrolovať a uložiť' },
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

export const PROPERTY_TYPE_OPTIONS: { value: PropertyType; label: string }[] = [
  { value: 'apartment', label: 'Byt' },
  { value: 'house', label: 'Dom' },
  { value: 'land', label: 'Pozemok' },
  { value: 'commercial', label: 'Komerčné' },
];

/** Simulated pre-populated listing fetched from API. */
export const MOCK_LISTING_DATA: ListingEditFormData = {
  transactionType: 'sale',
  propertyType: 'apartment',
  address: 'Pribinova 25',
  city: 'Bratislava',
  postalCode: '81109',
  country: 'SK',
  area: 72,
  rooms: 3,
  floor: 4,
  totalFloors: 8,
  yearBuilt: 2005,
  description:
    'Slnečný 3-izbový byt v tichej časti Petržalky s balkónom a pivnicou. Kompletná rekonštrukcia v roku 2022. Kuchyňa s jedálňou, šatník, 2 kúpeľne.',
  existingPhotoUrls: [
    'https://images.unsplash.com/photo-1560518883-ce09059eeffa?w=400&q=70',
    'https://images.unsplash.com/photo-1512917774080-9991f1c4c750?w=400&q=70',
    'https://images.unsplash.com/photo-1556909114-f6e7ad7d3136?w=400&q=70',
  ],
  newPhotos: [],
  price: 245000,
  currency: 'EUR',
  priceNegotiable: false,
  status: 'active',
};
