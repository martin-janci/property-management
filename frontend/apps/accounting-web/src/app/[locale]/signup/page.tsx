import type { Metadata } from 'next';
import { useTranslations } from 'next-intl';
import { getTranslations, setRequestLocale } from 'next-intl/server';
import { CheckIcon } from '@/components/marketing/Icons';
import { SiteHeader } from '@/components/marketing/SiteHeader';
import { SignupForm } from '@/components/signup/SignupForm';
import { BRAND_NAME } from '@/i18n/config';
import { Link } from '@/i18n/routing';
import styles from '@/styles/signup.module.css';

type Props = { params: Promise<{ locale: string }> };

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { locale } = await params;
  const t = await getTranslations({ locale, namespace: 'metadata.signup' });
  return {
    title: t('title'),
    description: t('description'),
    alternates: { canonical: locale === 'cs' ? '/signup' : `/${locale}/signup` },
  };
}

export default async function SignupPage({ params }: Props) {
  const { locale } = await params;
  setRequestLocale(locale);
  return <SignupContent />;
}

function SignupContent() {
  const t = useTranslations('signup');
  return (
    <>
      <SiteHeader />
      <div className={styles.wrap}>
        <section className={styles.formCol}>
          <div className={styles.formInner}>
            <h1 className={styles.title}>{t('title')}</h1>
            <p className={styles.subtitle}>{t('subtitle')}</p>
            <SignupForm />
            <p className={styles.altAuth}>
              {t('haveAccount')}{' '}
              {/* TODO(PAP-303 #3): point at /login once the login route lands;
                  currently the only public route is /signup. */}
              <Link href="/signup" className={styles.altAuthLink}>
                {t('login')}
              </Link>
            </p>
          </div>
        </section>

        <aside className={styles.asideCol}>
          {/* Only the decorative brand mark is hidden from assistive tech; the
              benefits heading + list below are real value-prop content and must
              stay announced (#1828 finding-1). */}
          <div className={styles.asideBrand} aria-hidden="true">
            {BRAND_NAME}
          </div>
          <h2 className={styles.asideHeading}>{t('benefitsTitle')}</h2>
          <ul className={styles.benefitList}>
            {[0, 1, 2, 3].map((i) => (
              <li key={i} className={styles.benefit}>
                <span className={styles.benefitCheck}>
                  <CheckIcon />
                </span>
                {t(`benefits.${i}`)}
              </li>
            ))}
          </ul>
        </aside>
      </div>
    </>
  );
}
