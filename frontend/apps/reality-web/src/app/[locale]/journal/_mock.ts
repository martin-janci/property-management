export interface JournalArticle {
  id: string;
  slug: string;
  title: string;
  excerpt: string;
  category: string;
  author: string;
  publishedAt: string;
  readMin: number;
  imageUrl: string;
  featured?: boolean;
}

export const MOCK_ARTICLES: JournalArticle[] = [
  {
    id: '1',
    slug: 'trh-nehnutelnosti-2025',
    title: 'Trh s nehnuteľnosťami v roku 2025: čo nás čaká?',
    excerpt:
      'Analytici predpovedajú mierny rast cien bytov v Bratislave a stagnáciu v regiónoch. Prečítajte si náš exkluzívny prehľad.',
    category: 'Analýza trhu',
    author: 'Martin Kováč',
    publishedAt: '2025-05-01',
    readMin: 8,
    imageUrl: 'https://images.unsplash.com/photo-1560518883-ce09059eeffa?w=800&q=80',
    featured: true,
  },
  {
    id: '2',
    slug: 'ako-pripravit-byt-na-predaj',
    title: 'Ako pripraviť byt na predaj: 10 tipov od expertov',
    excerpt:
      'Prvý dojem je rozhodujúci. Naši odborníci radia, ako jednoducho zvýšiť hodnotu vašej nehnuteľnosti.',
    category: 'Predaj',
    author: 'Jana Horáková',
    publishedAt: '2025-04-22',
    readMin: 6,
    imageUrl: 'https://images.unsplash.com/photo-1556909114-f6e7ad7d3136?w=800&q=80',
  },
  {
    id: '3',
    slug: 'hypoteka-2025-sprievodca',
    title: 'Hypotéka v roku 2025: kompletný sprievodca pre kupujúcich',
    excerpt:
      'Úrokové sadzby, požiadavky bánk a stratégie financovania – všetko, čo potrebujete vedieť pred podaním žiadosti.',
    category: 'Financovanie',
    author: 'Peter Novák',
    publishedAt: '2025-04-15',
    readMin: 10,
    imageUrl: 'https://images.unsplash.com/photo-1554224155-8d04cb21cd6c?w=800&q=80',
  },
  {
    id: '4',
    slug: 'bratislava-nova-stvrt',
    title: 'Nová štvrť Bratislavy: príležitosť alebo riziko?',
    excerpt:
      'Rozvoj Petržalky II a nové rezidenčné projekty menia tvár slovenského hlavného mesta.',
    category: 'Správy',
    author: 'Lucia Šimková',
    publishedAt: '2025-04-08',
    readMin: 5,
    imageUrl: 'https://images.unsplash.com/photo-1486325212027-8081e485255e?w=800&q=80',
  },
  {
    id: '5',
    slug: 'investovanie-do-nehnutelnosti',
    title: 'Investovanie do nehnuteľností pre začiatočníkov',
    excerpt:
      'Pasívny príjem z prenájmu, flipping bytov alebo REITs? Porovnávame stratégie pre malých investorov.',
    category: 'Investície',
    author: 'Martin Kováč',
    publishedAt: '2025-03-30',
    readMin: 12,
    imageUrl: 'https://images.unsplash.com/photo-1579621970795-87facc2f976d?w=800&q=80',
  },
  {
    id: '6',
    slug: 'prenajom-vs-kupa',
    title: 'Prenájom vs. kúpa: kalkulačka a tipy pre rok 2025',
    excerpt: 'Pomáhame vám rozhodnúť sa, čo je v súčasnom trhu výhodnejšie.',
    category: 'Rady',
    author: 'Jana Horáková',
    publishedAt: '2025-03-20',
    readMin: 7,
    imageUrl: 'https://images.unsplash.com/photo-1560520653-9e0e4c89eb11?w=800&q=80',
  },
];

export const MOST_READ = ['1', '3', '5'];
export const EDITORS_PICKS = ['2', '4', '6'];
