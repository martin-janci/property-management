import { useTranslation } from 'react-i18next';

import { usePrincipalCapabilities } from '../auth/usePrincipalCapabilities';

export function Dashboard() {
  const { t } = useTranslation();
  const { capabilities, isPlatformPrincipal } = usePrincipalCapabilities();
  return (
    <section aria-label="dashboard">
      <h1>{t('admin.title', 'Admin dashboard')}</h1>
      <p>
        {t('admin.platformPrincipal', 'Platform principal:')} {String(isPlatformPrincipal)}
      </p>
      <p>
        {t('admin.capabilitiesCount', 'Capabilities:')} {capabilities.length}
      </p>
      <ul>
        {capabilities.map((c) => (
          <li key={c}>{c}</li>
        ))}
      </ul>
    </section>
  );
}
