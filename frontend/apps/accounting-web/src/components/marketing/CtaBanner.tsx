import { useTranslations } from 'next-intl';
import { Link } from '@/i18n/routing';
import styles from '@/styles/marketing.module.css';

export function CtaBanner() {
  const t = useTranslations('cta');

  return (
    <section className={styles.ctaBand}>
      <div className={`container ${styles.ctaInner}`}>
        <h2 className={styles.ctaTitle}>{t('title')}</h2>
        <p className={styles.ctaSubtitle}>{t('subtitle')}</p>
        <div className={styles.ctaActions}>
          <Link href="/signup" className="btn btn-lg btn-white">
            {t('button')}
          </Link>
        </div>
      </div>
    </section>
  );
}
