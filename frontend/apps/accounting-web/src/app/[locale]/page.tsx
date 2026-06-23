import { useTranslations } from 'next-intl';
import { setRequestLocale } from 'next-intl/server';
import { CtaBanner } from '@/components/marketing/CtaBanner';
import { FeatureGrid } from '@/components/marketing/FeatureGrid';
import { Hero } from '@/components/marketing/Hero';
import { PricingTeaser } from '@/components/marketing/PricingTeaser';
import { SiteFooter } from '@/components/marketing/SiteFooter';
import { SiteHeader } from '@/components/marketing/SiteHeader';
import { Steps } from '@/components/marketing/Steps';

type Props = { params: Promise<{ locale: string }> };

export default async function LandingPage({ params }: Props) {
  const { locale } = await params;
  setRequestLocale(locale);

  return <LandingContent />;
}

function LandingContent() {
  const t = useTranslations('nav');
  return (
    <>
      <a className="skip-link" href="#main">
        {t('skipToContent')}
      </a>
      <SiteHeader />
      <main id="main">
        <Hero />
        <FeatureGrid />
        <Steps />
        <PricingTeaser />
        <CtaBanner />
      </main>
      <SiteFooter />
    </>
  );
}
