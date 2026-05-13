import { pageMetadata } from '@/lib/page-metadata';

export const generateMetadata = pageMetadata('priceMap');

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
