/**
 * Homepage
 *
 * Reality Portal homepage with hero search and featured listings (Epic 44, Story 44.1).
 */

'use client';

import {
  CategoryCards,
  FeaturedListings,
  HeroSearch,
  HomeCTA,
  PopularNeighbourhoods,
  ValueStrip,
} from '@/components/home';
import { Footer, Header } from '@/components/ui';

/**
 * Reality Portal homepage.
 *
 * Section order matches the design system reference (`Reality Portal Home -
 * Standalone.html` in the design bundle): hero → categories → featured →
 * neighbourhoods → value strip → selling CTA → footer. Adding/removing a
 * section here should be reflected in the design too — keep the order.
 */
export default function HomePage() {
  return (
    <div className="page-container">
      <Header />
      <main>
        <HeroSearch />
        <CategoryCards />
        <FeaturedListings />
        <PopularNeighbourhoods />
        <ValueStrip />
        <HomeCTA />
      </main>
      <Footer />

      <style jsx>{`
        .page-container {
          min-height: 100vh;
          display: flex;
          flex-direction: column;
        }

        main {
          flex: 1;
        }
      `}</style>
    </div>
  );
}
