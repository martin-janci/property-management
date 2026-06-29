import { useTranslations } from 'next-intl';
import { Link } from '@/i18n/routing';
import styles from '@/styles/marketing.module.css';

export function Hero() {
  const t = useTranslations('hero');

  return (
    <section className={styles.hero}>
      <div className={`container ${styles.heroInner}`}>
        <div>
          <span className={styles.heroBadge}>{t('badge')}</span>
          <h1 className={styles.heroTitle}>{t('title')}</h1>
          <p className={styles.heroSubtitle}>{t('subtitle')}</p>
          <div className={styles.heroActions}>
            <Link href="/signup" className="btn btn-lg btn-primary">
              {t('ctaPrimary')}
            </Link>
            <a href="#features" className="btn btn-lg btn-secondary">
              {t('ctaSecondary')}
            </a>
          </div>
          <p className={styles.heroTrust}>{t('trust')}</p>
        </div>

        {/* Decorative invoice mock — token-built, no image asset. */}
        <div className={styles.heroVisual} aria-hidden="true">
          <div className={styles.invoiceCard}>
            <div className={styles.invoiceTop}>
              <div>
                <div className={styles.invoiceLabel}>Faktúra</div>
                <div className={styles.invoiceNo}>2026-0042</div>
              </div>
              <span className={styles.invoicePaid}>Zaplatené</span>
            </div>
            <div className={styles.invoiceRow}>
              <span>Konzultácia (8 h)</span>
              <span>480,00 €</span>
            </div>
            <div className={styles.invoiceRow}>
              <span>Návrh & vývoj</span>
              <span>1 200,00 €</span>
            </div>
            <div className={styles.invoiceRow}>
              <span>DPH 21 %</span>
              <span>352,80 €</span>
            </div>
            <div className={styles.invoiceTotal}>
              <span>Spolu</span>
              <span>2 032,80 €</span>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
