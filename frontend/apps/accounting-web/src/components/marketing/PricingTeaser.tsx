import { useTranslations } from 'next-intl';
import { Link } from '@/i18n/routing';
import styles from '@/styles/marketing.module.css';
import { CheckIcon } from './Icons';

export function PricingTeaser() {
  const t = useTranslations('pricing');
  const plans = [0, 1, 2];
  // Feature-count per plan mirrors the messages arrays (Start 3, Standard 4, Team 4).
  const featureCounts = [3, 4, 4];

  return (
    <section id="pricing" className={`container ${styles.section}`}>
      <div className={styles.sectionHead}>
        <h2 className={styles.sectionTitle}>{t('title')}</h2>
        <p className={styles.sectionSubtitle}>{t('subtitle')}</p>
      </div>

      <div className={styles.pricingGrid}>
        {plans.map((p) => {
          const featured = p === 1;
          const features = Array.from({ length: featureCounts[p] }, (_, k) => k);
          return (
            <div
              key={p}
              className={`${styles.priceCard} ${featured ? styles.priceCardFeatured : ''}`}
            >
              {featured && <span className={styles.priceTag}>{t('mostPopular')}</span>}
              <div className={styles.priceName}>{t(`plans.${p}.name`)}</div>
              <div className={styles.priceTagline}>{t(`plans.${p}.tagline`)}</div>
              <div className={styles.priceAmount}>
                <span className={styles.priceValue}>{t(`plans.${p}.price`)}</span>
                <span className={styles.pricePeriod}>{t('perMonth')}</span>
              </div>
              <ul className={styles.priceFeatures}>
                {features.map((f) => (
                  <li key={f} className={styles.priceFeature}>
                    <CheckIcon className={styles.priceCheck} />
                    {t(`plans.${p}.features.${f}`)}
                  </li>
                ))}
              </ul>
              <Link
                href="/signup"
                className={`btn btn-md ${featured ? 'btn-primary' : 'btn-secondary'}`}
              >
                {t(`plans.${p}.cta`)}
              </Link>
            </div>
          );
        })}
      </div>

      <p className={styles.priceNote}>{t('note')}</p>
    </section>
  );
}
