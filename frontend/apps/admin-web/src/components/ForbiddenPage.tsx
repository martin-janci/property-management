import { Link } from 'react-router-dom';

interface ForbiddenPageProps {
  requiredCapability: string | string[];
}

/**
 * 403 page shown when a user navigates directly to a route they lack
 * the required capability for. Lists the missing capability code in
 * monospace and links to the Capabilities admin page.
 */
export function ForbiddenPage({ requiredCapability }: ForbiddenPageProps) {
  const caps = Array.isArray(requiredCapability) ? requiredCapability : [requiredCapability];

  return (
    <section
      role="alert"
      style={{
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        minHeight: '60vh',
        padding: 'var(--ppt-space-xl, 24px)',
        background: 'var(--ppt-bg-app, #f9fafb)',
        textAlign: 'center',
      }}
    >
      <div
        style={{
          maxWidth: '480px',
          background: 'var(--ppt-bg-surface, #ffffff)',
          border: '1px solid var(--ppt-border-default, #e5e7eb)',
          borderRadius: 'var(--ppt-radius-lg, 12px)',
          padding: 'var(--ppt-space-2xl, 32px)',
          boxShadow: 'var(--ppt-shadow-card, 0 1px 3px rgba(0,0,0,0.1))',
        }}
      >
        <h1
          style={{
            fontSize: 'var(--ppt-text-h1, 1.5rem)',
            fontWeight: 700,
            color: 'var(--ppt-fg-primary, #111827)',
            margin: '0 0 12px',
          }}
        >
          403 — Missing capability
        </h1>
        <p
          style={{
            fontSize: 'var(--ppt-text-body, 0.875rem)',
            color: 'var(--ppt-fg-secondary, #374151)',
            margin: '0 0 16px',
          }}
        >
          Your account does not hold the required{' '}
          {caps.length === 1 ? 'capability' : 'capabilities'} to view this page:
        </p>
        <ul
          style={{
            listStyle: 'none',
            padding: 0,
            margin: '0 0 20px',
            display: 'flex',
            flexDirection: 'column',
            gap: '6px',
            alignItems: 'center',
          }}
        >
          {caps.map((cap) => (
            <li key={cap}>
              <code
                style={{
                  fontFamily: 'var(--ppt-font-mono, monospace)',
                  fontSize: '0.875em',
                  background: 'var(--ppt-bg-subtle, #f3f4f6)',
                  padding: '2px 8px',
                  borderRadius: 'var(--ppt-radius-sm, 4px)',
                  color: 'var(--ppt-fg-primary, #111827)',
                }}
              >
                {cap}
              </code>
            </li>
          ))}
        </ul>
        <Link
          to="/identity/capabilities"
          style={{
            display: 'inline-block',
            fontSize: 'var(--ppt-text-body, 0.875rem)',
            color: 'var(--ppt-fg-link, #2563eb)',
            textDecoration: 'underline',
          }}
        >
          Request access here
        </Link>
      </div>
    </section>
  );
}
