'use client';

import { useLocale } from 'next-intl';
import { locales } from '@/i18n/config';
import { Link, usePathname } from '@/i18n/routing';
import styles from '@/styles/marketing.module.css';

/**
 * cs/sk locale toggle. Keeps the current pathname and swaps only the locale
 * prefix (next-intl Link handles the `as-needed` prefixing).
 */
export function LocaleSwitcher() {
  const active = useLocale();
  const pathname = usePathname();

  return (
    <div className={styles.localeSwitch} role="group" aria-label="Language">
      {locales.map((lc) => (
        <Link
          key={lc}
          href={pathname}
          locale={lc}
          className={`${styles.localeOption} ${lc === active ? styles.localeOptionActive : ''}`}
          aria-current={lc === active ? 'true' : undefined}
        >
          {lc.toUpperCase()}
        </Link>
      ))}
    </div>
  );
}
