/**
 * 404 page (Next.js convention) for the [locale] segment.
 */

import { StateView } from '@/components/states';
import Link from 'next/link';

export default function NotFound() {
  return (
    <StateView
      icon="🧭"
      code="404"
      title="Page not found"
      description="The page you're looking for doesn't exist or has moved."
    >
      <Link href="/" className="state-action">
        Back to home
      </Link>
      <style jsx global>{`
        .state-action {
          padding: 10px 20px; border-radius: 8px;
          background: #2563eb; color: #fff; font-weight: 500;
          text-decoration: none; display: inline-block; text-align: center;
        }
        .state-action:hover { background: #1d4ed8; }
      `}</style>
    </StateView>
  );
}
