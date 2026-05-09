export interface SellStep {
  id: number;
  title: string;
  description: string;
}

export const SELL_STEPS: SellStep[] = [
  { id: 1, title: 'Typ a poloha', description: 'Vyberte typ nehnuteľnosti a zadajte adresu' },
  { id: 2, title: 'Detaily', description: 'Plocha, izby, poschodie, rok výstavby' },
  { id: 3, title: 'Fotografie', description: 'Pridajte kvalitné fotografie' },
  { id: 4, title: 'Cena', description: 'Nastavte požadovanú cenu' },
  { id: 5, title: 'Kontakt & súhrn', description: 'Overte údaje a zverejnite ponuku' },
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

export const PROPERTY_TYPE_OPTIONS: { value: PropertyType; label: string }[] = [
  { value: 'apartment', label: 'Byt' },
  { value: 'house', label: 'Dom' },
  { value: 'land', label: 'Pozemok' },
  { value: 'commercial', label: 'Komerčné' },
];
