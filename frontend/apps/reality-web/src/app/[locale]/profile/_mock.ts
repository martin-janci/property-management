export interface UserProfile {
  id: string;
  name: string;
  email: string;
  phone: string;
  avatarUrl: string;
  coverUrl: string;
  memberSince: string;
  verified: boolean;
  bio: string;
  stats: {
    activeListings: number;
    completedDeals: number;
    reviews: number;
    avgRating: number;
  };
}

export interface ProfileListing {
  id: string;
  title: string;
  price: number;
  currency: string;
  area: number;
  rooms: number;
  location: string;
  status: 'active' | 'sold' | 'rented';
  imageUrl: string;
}

export interface ActivityEntry {
  id: string;
  type: 'listing_viewed' | 'inquiry_sent' | 'favorite_added' | 'search_saved';
  description: string;
  timestamp: string;
}

export interface Review {
  id: string;
  reviewer: string;
  rating: number;
  comment: string;
  date: string;
}

export const MOCK_PROFILE: UserProfile = {
  id: 'u1',
  name: 'Tomáš Novotný',
  email: 't.novotny@email.sk',
  phone: '+421 911 123 456',
  avatarUrl: 'https://i.pravatar.cc/150?img=11',
  coverUrl: 'https://images.unsplash.com/photo-1486325212027-8081e485255e?w=1200&q=70',
  memberSince: 'Január 2022',
  verified: true,
  bio: 'Skúsený maklér s 8-ročnou praxou v oblasti rezidenčných nehnuteľností v Bratislave a okolí.',
  stats: { activeListings: 12, completedDeals: 47, reviews: 23, avgRating: 4.7 },
};

export const MOCK_LISTINGS: ProfileListing[] = [
  { id: 'l1', title: '3-izbový byt Petržalka', price: 245000, currency: '€', area: 72, rooms: 3, location: 'Bratislava – Petržalka', status: 'active', imageUrl: 'https://images.unsplash.com/photo-1560518883-ce09059eeffa?w=400&q=70' },
  { id: 'l2', title: '2-izbový byt Ružinov', price: 1150, currency: '€', area: 55, rooms: 2, location: 'Bratislava – Ružinov', status: 'active', imageUrl: 'https://images.unsplash.com/photo-1556909114-f6e7ad7d3136?w=400&q=70' },
  { id: 'l3', title: 'Rodinný dom Senec', price: 389000, currency: '€', area: 186, rooms: 5, location: 'Senec', status: 'sold', imageUrl: 'https://images.unsplash.com/photo-1570129477492-45c003edd2be?w=400&q=70' },
];

export const MOCK_ACTIVITY: ActivityEntry[] = [
  { id: 'a1', type: 'listing_viewed', description: 'Prezerali ste: 3-izbový byt, Petržalka', timestamp: 'Pred 2 hodinami' },
  { id: 'a2', type: 'inquiry_sent', description: 'Odoslali ste dopyt pre 2-izbový byt, Ružinov', timestamp: 'Včera, 14:32' },
  { id: 'a3', type: 'favorite_added', description: 'Pridali ste do obľúbených: Penthouse Staré Mesto', timestamp: 'Pred 3 dňami' },
  { id: 'a4', type: 'search_saved', description: 'Uložili ste vyhľadávanie: Byty, Bratislava I, do 300 000 €', timestamp: 'Pred týždňom' },
];

export const MOCK_REVIEWS: Review[] = [
  { id: 'r1', reviewer: 'Eva Malíková', rating: 5, comment: 'Skvelý maklér! Veľmi profesionálny a vždy k dispozícii.', date: 'Apríl 2025' },
  { id: 'r2', reviewer: 'Ján Horváth', rating: 4, comment: 'Dobrá komunikácia a rýchly predaj. Odporúčam.', date: 'Marec 2025' },
  { id: 'r3', reviewer: 'Marta Bieliková', rating: 5, comment: 'Pomohol mi nájsť presne to, čo som hľadala. Ďakujem!', date: 'Február 2025' },
];

export type ProfileTab = 'listings' | 'activity' | 'reviews' | 'settings';
