import { useTranslations } from 'next-intl';
import { BRAND_NAME } from '@/i18n/config';
import { Link } from '@/i18n/routing';
import styles from '@/styles/marketing.module.css';
import { LocaleSwitcher } from './LocaleSwitcher';

export function SiteFooter() {
  const t = useTranslations('footer');
  const year = 2026; // Static build year — avoids Date() in RSC render path.

  return (
    <footer className={styles.footer}>
      <div className="container">
        <div className={styles.footerGrid}>
          <div>
            <div className={styles.footerBrand}>
              <span className={styles.brandMark} aria-hidden="true">
                {BRAND_NAME.charAt(0)}
              </span>
              {BRAND_NAME}
            </div>
            <p className={styles.footerTagline}>{t('tagline')}</p>
          </div>

          <div>
            <div className={styles.footerHeading}>{t('productHeading')}</div>
            <ul className={styles.footerLinks}>
              <li>
                <a className={styles.footerLink} href="#features">
                  {t('links.features')}
                </a>
              </li>
              <li>
                <a className={styles.footerLink} href="#pricing">
                  {t('links.pricing')}
                </a>
              </li>
              <li>
                <Link className={styles.footerLink} href="/signup">
                  {t('links.signup')}
                </Link>
              </li>
            </ul>
          </div>

          <div>
            <div className={styles.footerHeading}>{t('companyHeading')}</div>
            <ul className={styles.footerLinks}>
              <li>
                <Link className={styles.footerLink} href="/signup">
                  {t('links.login')}
                </Link>
              </li>
            </ul>
          </div>

          <div>
            <div className={styles.footerHeading}>{t('legalHeading')}</div>
            <ul className={styles.footerLinks}>
              <li>
                <span className={styles.footerLink}>{t('links.terms')}</span>
              </li>
              <li>
                <span className={styles.footerLink}>{t('links.privacy')}</span>
              </li>
            </ul>
          </div>
        </div>

        <div className={styles.footerBottom}>
          <span>
            © {year} {BRAND_NAME}. {t('rights')} — {t('placeholderNote')}
          </span>
          <LocaleSwitcher />
        </div>
      </div>
    </footer>
  );
}
