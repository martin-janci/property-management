import { BRAND_NAME } from '@/i18n/config';
import { Link } from '@/i18n/routing';

export default function NotFound() {
  return (
    <main
      style={{
        minHeight: '70vh',
        display: 'grid',
        placeItems: 'center',
        textAlign: 'center',
        padding: 'var(--ppt-space-8)',
      }}
    >
      <div>
        <p
          style={{
            fontSize: '3rem',
            fontWeight: 'var(--ppt-font-weight-bold)',
            color: 'var(--ppt-color-primary)',
          }}
        >
          404
        </p>
        <Link href="/" className="btn btn-md btn-primary" style={{ marginTop: '1rem' }}>
          ← {BRAND_NAME}
        </Link>
      </div>
    </main>
  );
}
