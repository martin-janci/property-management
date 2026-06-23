import { useTranslations } from 'next-intl';
import styles from '@/styles/marketing.module.css';
import {
  ContactsIcon,
  DashboardIcon,
  ExportIcon,
  InvoiceIcon,
  MatchIcon,
  ShieldIcon,
} from './Icons';

const ICONS = [InvoiceIcon, ContactsIcon, MatchIcon, DashboardIcon, ExportIcon, ShieldIcon];

export function FeatureGrid() {
  const t = useTranslations('features');
  // Six feature slots, keyed by index into the messages array.
  const items = [0, 1, 2, 3, 4, 5];

  return (
    <section id="features" className={`container ${styles.section}`}>
      <div className={styles.sectionHead}>
        <h2 className={styles.sectionTitle}>{t('title')}</h2>
        <p className={styles.sectionSubtitle}>{t('subtitle')}</p>
      </div>

      <div className={styles.featureGrid}>
        {items.map((i) => {
          const Icon = ICONS[i];
          return (
            <article key={i} className={styles.featureCard}>
              <span className={styles.featureIcon}>
                <Icon />
              </span>
              <h3 className={styles.featureTitle}>{t(`items.${i}.title`)}</h3>
              <p className={styles.featureBody}>{t(`items.${i}.body`)}</p>
            </article>
          );
        })}
      </div>
    </section>
  );
}
