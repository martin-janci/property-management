/**
 * Property comparison page.
 *
 * Epic 51 - Story 51.2: Comparison View
 * Epic 51 - Story 51.3: Share Comparison
 */

import { getTranslations } from 'next-intl/server';

import { ComparisonUrlHandler, ComparisonView } from '../../../components/comparison';
import { Header } from '../../../components/ui';
import { pageMetadata } from '../../../lib/page-metadata';

// Was a hardcoded English `metadata` object (#955). Use the shared helper so the
// title is localized and carries the site " — Reality Portal" suffix.
export const generateMetadata = pageMetadata('compare');

interface ComparePageProps {
  searchParams: Promise<{ ids?: string }>;
}

export default async function ComparePage({ searchParams }: ComparePageProps) {
  const params = await searchParams;
  const sharedIds = params.ids?.split(',').filter(Boolean) ?? [];
  const t = await getTranslations('pages.compare');

  return (
    <>
      <Header />
      <main className="min-h-screen bg-gray-50">
        <div className="mx-auto max-w-[1200px] px-6">
          <div className="border-b border-gray-200 bg-white py-8 -mx-6 px-6 mb-6">
            <h1 className="text-[28px] font-bold text-gray-900 mb-2">{t('h1')}</h1>
            <p className="text-gray-500">{t('subtitle')}</p>
          </div>
          {/* Handle shared URL ids parameter */}
          <ComparisonUrlHandler sharedIds={sharedIds} />
          <ComparisonView />
        </div>
      </main>
    </>
  );
}
