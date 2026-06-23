import { useTranslations } from 'next-intl';
import { BRAND_NAME } from '@/i18n/config';
import { Link } from '@/i18n/routing';
import styles from '@/styles/marketing.module.css';
import { LocaleSwitcher } from './LocaleSwitcher';

export function SiteHeader() {
  const t = useTranslations('nav');

  return (
    <header className={styles.header}>
      <div className={`container ${styles.headerInner}`}>
        <Link href="/" className={styles.brand} aria-label={BRAND_NAME}>
          <span className={styles.brandMark} aria-hidden="true">
            {BRAND_NAME.charAt(0)}
          </span>
          {BRAND_NAME}
        </Link>

        <nav className={`${styles.nav} ${styles.navLinks}`} aria-label="Primary">
          <a className={styles.navLink} href="#features">
            {t('features')}
          </a>
          <a className={styles.navLink} href="#pricing">
            {t('pricing')}
          </a>
        </nav>

        <div className={styles.navActions}>
          <LocaleSwitcher />
          <Link href="/signup" className={`${styles.navLink} ${styles.navLinks}`}>
            {t('login')}
          </Link>
          <Link href="/signup" className="btn btn-md btn-primary">
            {t('signup')}
          </Link>
        </div>
      </div>
    </header>
  );
}
