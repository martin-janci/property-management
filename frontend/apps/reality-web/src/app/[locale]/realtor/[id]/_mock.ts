export interface AgentProfile {
  id: string;
  name: string;
  title: string;
  phone: string;
  email: string;
  avatarUrl: string;
  coverUrl: string;
  agencyName: string;
  agencyLogoUrl: string;
  licenseNumber: string;
  verified: boolean;
  bio: string;
  specializations: string[];
  languages: string[];
  memberSince: string;
  stats: {
    activeListings: number;
    soldLastYear: number;
    avgDaysOnMarket: number;
    avgRating: number;
    reviewCount: number;
  };
}

export interface AgentListing {
  id: string;
  title: string;
  price: number;
  currency: string;
  area: number;
  rooms: number;
  location: string;
  imageUrl: string;
  type: 'sale' | 'rent';
}

export interface AgentReview {
  id: string;
  reviewer: string;
  rating: number;
  comment: string;
  date: string;
  verified: boolean;
}

export const MOCK_AGENT: AgentProfile = {
  id: 'realtor-1',
  name: 'Ing. Petra Horáčková',
  title: 'Senior realitná maklérka',
  phone: '+421 903 456 789',
  email: 'p.horackova@reality-portal.sk',
  avatarUrl: 'https://i.pravatar.cc/150?img=47',
  coverUrl: 'https://images.unsplash.com/photo-1486325212027-8081e485255e?w=1400&q=70',
  agencyName: 'Premium Reality s.r.o.',
  agencyLogoUrl: 'https://ui-avatars.com/api/?name=PR&background=2563eb&color=fff&size=64',
  licenseNumber: 'SK-REA-2019-4521',
  verified: true,
  bio: 'Viac ako 10 rokov skúseností v oblasti rezidenčných a komerčných nehnuteľností v Bratislave a okolí. Špecializujem sa na prémiové byty a rodinné domy. Moje priority sú transparentnosť, rýchlosť a spokojnosť klientov.',
  specializations: ['Rezidenčné', 'Prémiové', 'Komerčné'],
  languages: ['Slovenčina', 'Angličtina', 'Nemčina'],
  memberSince: 'Apríl 2019',
  stats: {
    activeListings: 18,
    soldLastYear: 34,
    avgDaysOnMarket: 28,
    avgRating: 4.8,
    reviewCount: 41,
  },
};

export const MOCK_AGENT_LISTINGS: AgentListing[] = [
  {
    id: 'al1',
    title: 'Luxusný penthouse Staré Mesto',
    price: 720000,
    currency: '€',
    area: 145,
    rooms: 4,
    location: 'Bratislava I',
    imageUrl: 'https://images.unsplash.com/photo-1600596542815-ffad4c1539a9?w=400&q=70',
    type: 'sale',
  },
  {
    id: 'al2',
    title: '3-izbový byt Ružinov s balkónom',
    price: 295000,
    currency: '€',
    area: 78,
    rooms: 3,
    location: 'Bratislava II',
    imageUrl: 'https://images.unsplash.com/photo-1560518883-ce09059eeffa?w=400&q=70',
    type: 'sale',
  },
  {
    id: 'al3',
    title: 'Moderný 2-izbový byt Nové Mesto',
    price: 1450,
    currency: '€',
    area: 58,
    rooms: 2,
    location: 'Bratislava III',
    imageUrl: 'https://images.unsplash.com/photo-1512917774080-9991f1c4c750?w=400&q=70',
    type: 'rent',
  },
  {
    id: 'al4',
    title: 'Rodinný dom s bazénom Senec',
    price: 485000,
    currency: '€',
    area: 220,
    rooms: 6,
    location: 'Senec',
    imageUrl: 'https://images.unsplash.com/photo-1570129477492-45c003edd2be?w=400&q=70',
    type: 'sale',
  },
];

export const MOCK_AGENT_REVIEWS: AgentReview[] = [
  {
    id: 'ar1',
    reviewer: 'Mgr. Tomáš Benko',
    rating: 5,
    comment:
      'Petra nám pomohla predať byt za 3 týždne za cenu, o akej sme snívali. Profesionálny prístup a vynikajúca komunikácia!',
    date: 'Apríl 2025',
    verified: true,
  },
  {
    id: 'ar2',
    reviewer: 'Katarína Novotná',
    rating: 5,
    comment:
      'Kúpili sme rodinný dom vďaka Petre. Celý proces bol hladký, bez stresu. Vrelo odporúčam.',
    date: 'Marec 2025',
    verified: true,
  },
  {
    id: 'ar3',
    reviewer: 'Marek Sloboda',
    rating: 4,
    comment:
      'Skvelý servis, rýchla komunikácia. Jedna hviezda dolu za mierne predĺženie termínu podpisu.',
    date: 'Február 2025',
    verified: false,
  },
];
