import { useTranslations } from 'next-intl';
import styles from '@/styles/marketing.module.css';

export function Steps() {
  const t = useTranslations('steps');
  const items = [0, 1, 2];

  return (
    <section className={styles.stepsBand}>
      <div className={`container ${styles.section}`}>
        <div className={styles.sectionHead}>
          <h2 className={styles.sectionTitle}>{t('title')}</h2>
        </div>
        <ol className={styles.steps}>
          {items.map((i) => (
            <li key={i} className={styles.step}>
              <span className={styles.stepNum} aria-hidden="true">
                {i + 1}
              </span>
              <h3 className={styles.stepTitle}>{t(`items.${i}.title`)}</h3>
              <p className={styles.stepBody}>{t(`items.${i}.body`)}</p>
            </li>
          ))}
        </ol>
      </div>
    </section>
  );
}
