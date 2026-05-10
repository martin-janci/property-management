export interface District {
  id: string;
  name: string;
  avgPricePerSqm: number;
  change12m: number; // percent
  medianTotal: number;
  listings: number;
  color: string; // choropleth fill
  svgPath: string; // simplified SVG path
}

export interface PriceMapFilter {
  propertyType: 'all' | 'apartment' | 'house' | 'land';
  transactionType: 'sale' | 'rent';
}

export const MOCK_DISTRICTS: District[] = [
  {
    id: 'ba-i',
    name: 'Bratislava I – Staré Mesto',
    avgPricePerSqm: 5800,
    change12m: 4.2,
    medianTotal: 380000,
    listings: 142,
    color: '#dc2626',
    svgPath: 'M200,120 L240,100 L270,130 L260,160 L220,165 Z',
  },
  {
    id: 'ba-ii',
    name: 'Bratislava II – Ružinov',
    avgPricePerSqm: 4200,
    change12m: 3.1,
    medianTotal: 260000,
    listings: 218,
    color: '#ea580c',
    svgPath: 'M260,160 L310,140 L330,180 L300,200 L260,195 Z',
  },
  {
    id: 'ba-iii',
    name: 'Bratislava III – Nové Mesto',
    avgPricePerSqm: 4600,
    change12m: 5.0,
    medianTotal: 305000,
    listings: 97,
    color: '#d97706',
    svgPath: 'M220,100 L240,80 L280,90 L270,130 L240,100 Z',
  },
  {
    id: 'ba-iv',
    name: 'Bratislava IV – Karlova Ves',
    avgPricePerSqm: 4900,
    change12m: 2.8,
    medianTotal: 320000,
    listings: 85,
    color: '#d97706',
    svgPath: 'M140,130 L200,120 L220,165 L180,175 L140,160 Z',
  },
  {
    id: 'ba-v',
    name: 'Bratislava V – Petržalka',
    avgPricePerSqm: 3400,
    change12m: 1.5,
    medianTotal: 210000,
    listings: 334,
    color: '#65a30d',
    svgPath: 'M180,175 L220,165 L260,195 L260,240 L200,245 L160,220 Z',
  },
];

export const MOCK_INSIGHTS = [
  { label: 'Priemerná cena / m²', value: '4 580 €', trend: '+3.2 % (12m)' },
  { label: 'Najdrahšia štvrť', value: 'Staré Mesto', trend: '5 800 € / m²' },
  { label: 'Najväčší rast', value: 'Nové Mesto', trend: '+5.0 % YoY' },
  { label: 'Aktívne inzeráty', value: '876', trend: '+12 za posledný týždeň' },
];
