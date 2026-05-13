import { pageMetadata } from '@/lib/page-metadata';

export const generateMetadata = pageMetadata('privacy');

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
