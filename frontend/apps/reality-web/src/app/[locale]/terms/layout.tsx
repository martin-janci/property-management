import { pageMetadata } from '@/lib/page-metadata';

export const generateMetadata = pageMetadata('terms');

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
