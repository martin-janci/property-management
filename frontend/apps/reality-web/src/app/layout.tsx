import type { Metadata } from 'next';
import './globals.css';

export const metadata: Metadata = {
  title: 'Reality Portal - Find Your Perfect Property',
  description:
    'Search thousands of property listings across Slovakia, Czech Republic, and beyond. Find apartments, houses, and commercial properties for sale or rent.',
  keywords:
    'real estate, property, apartments, houses, for sale, for rent, Slovakia, Czech Republic',
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return children;
}
