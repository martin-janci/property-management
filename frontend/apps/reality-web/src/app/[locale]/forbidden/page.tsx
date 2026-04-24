/**
 * 403 Forbidden page for the [locale] segment.
 *
 * Linked from the auth/permission flow when the user is signed in but lacks
 * the role required for a route. Runs as a server component so it uses
 * inline styles instead of styled-jsx (which is client-only).
 */

import { StateView } from '@/components/states';
import Link from 'next/link';
import type { CSSProperties } from 'react';

const linkStyle: CSSProperties = {
  padding: '10px 20px',
  borderRadius: 8,
  background: '#2563eb',
  color: '#fff',
  fontWeight: 500,
  textDecoration: 'none',
  display: 'inline-block',
  textAlign: 'center',
};

export default function ForbiddenPage() {
  return (
    <StateView
      icon="🔒"
      code="403"
      title="Access denied"
      description="You don't have permission to view this page. Contact support if you think this is a mistake."
    >
      <Link href="/" style={linkStyle}>
        Back to home
      </Link>
    </StateView>
  );
}
