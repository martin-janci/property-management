/**
 * 403 Forbidden page for the [locale] segment.
 *
 * Linked from the auth/permission flow when the user is signed in but lacks
 * the role required for a route.
 */

import { StateView } from '@/components/states';
import Link from 'next/link';

export default function ForbiddenPage() {
  return (
    <StateView
      icon="🔒"
      code="403"
      title="Access denied"
      description="You don't have permission to view this page. Contact support if you think this is a mistake."
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
