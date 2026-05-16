import { useTranslation } from 'react-i18next';
import { Link, Outlet } from 'react-router-dom';

import { useAdminAuth } from '../auth/AdminAuthContext';
import { usePrincipalCapabilities } from '../auth/usePrincipalCapabilities';

export function AdminLayout() {
  const { t } = useTranslation();
  const auth = useAdminAuth();
  const { capabilities } = usePrincipalCapabilities();
  const has = (cap: string) => capabilities.includes(cap as never);

  return (
    <div className="admin-shell">
      <aside>
        <h2>{t('admin.title', 'Admin')}</h2>
        <nav>
          <Link to="/">{t('admin.nav.dashboard', 'Dashboard')}</Link>
          {has('agencies_read') && (
            <Link to="/agencies">{t('admin.agencies.title', 'Agencies')}</Link>
          )}
          {has('users_read') && (
            <Link to="/users">{t('admin.users.title', 'Users')}</Link>
          )}
          {has('audit_read') && (
            <Link to="/audit">{t('admin.audit.title', 'Audit')}</Link>
          )}
          {has('feature_flags_write') && (
            <Link to="/feature-flags">{t('admin.featureFlags.title', 'Feature flags')}</Link>
          )}
          {has('site_settings_write') && (
            <Link to="/platform">{t('admin.platform.title', 'Platform')}</Link>
          )}
        </nav>
        <button type="button" onClick={auth.logout}>
          {t('admin.actions.signOut', 'Sign out')}
        </button>
      </aside>
      <main>
        <Outlet />
      </main>
    </div>
  );
}
