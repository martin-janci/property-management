/**
 * Shared chrome for the Community section: page title, the Feed / Groups /
 * Events / Marketplace tab bar, and the building selector. Children receive
 * the active building id; the layout itself renders the loading, error and
 * "no building" states so every page handles them the same way.
 */

import type { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { NavLink } from 'react-router-dom';
import { Spinner } from '../../../components';
import { type CommunityBuildingContext, withBuilding } from '../hooks/useCommunityBuilding';
import { EmptyState, ErrorState } from './StateViews';

export type CommunityTab = 'feed' | 'groups' | 'events' | 'marketplace';

const TABS: { id: CommunityTab; path: string; key: string }[] = [
  { id: 'feed', path: '/community', key: 'community.hub.tabs.feed' },
  { id: 'groups', path: '/community/groups', key: 'community.hub.tabs.groups' },
  { id: 'events', path: '/community/events', key: 'community.hub.tabs.events' },
  { id: 'marketplace', path: '/community/marketplace', key: 'community.hub.tabs.marketplace' },
];

interface CommunityLayoutProps {
  active: CommunityTab;
  title: string;
  subtitle?: string;
  actions?: ReactNode;
  building: CommunityBuildingContext;
  children: (buildingId: string) => ReactNode;
}

export function CommunityLayout({
  active,
  title,
  subtitle,
  actions,
  building,
  children,
}: CommunityLayoutProps) {
  const { t } = useTranslation();
  const { buildings, buildingId, setBuildingId, isLoading, error } = building;

  return (
    <div className="max-w-6xl mx-auto px-4 py-8">
      <div className="flex flex-wrap items-start justify-between gap-4 mb-6">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">{title}</h1>
          {subtitle && <p className="mt-1 text-sm text-gray-500">{subtitle}</p>}
        </div>
        <div className="flex flex-wrap items-center gap-3">
          {buildings.length > 1 && (
            <label className="flex items-center gap-2 text-sm text-gray-600">
              <span>{t('community.hub.building')}</span>
              <select
                value={buildingId ?? ''}
                onChange={(e) => setBuildingId(e.target.value)}
                className="px-3 py-2 text-sm border border-gray-300 rounded-md bg-white focus:ring-blue-500 focus:border-blue-500"
              >
                {buildings.map((b) => (
                  <option key={b.id} value={b.id}>
                    {b.name}
                  </option>
                ))}
              </select>
            </label>
          )}
          {buildingId && actions}
        </div>
      </div>

      <nav aria-label={t('community.hub.tabsLabel')} className="mb-6 border-b border-gray-200">
        <ul className="flex gap-1 overflow-x-auto">
          {TABS.map((tab) => (
            <li key={tab.id}>
              <NavLink
                to={withBuilding(tab.path, buildingId)}
                end
                aria-current={tab.id === active ? 'page' : undefined}
                className={`inline-block px-4 py-2 text-sm font-medium border-b-2 -mb-px whitespace-nowrap ${
                  tab.id === active
                    ? 'border-blue-600 text-blue-600'
                    : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'
                }`}
              >
                {t(tab.key)}
              </NavLink>
            </li>
          ))}
        </ul>
      </nav>

      {isLoading ? (
        <div className="flex justify-center py-16">
          <Spinner size="lg" label={t('community.common.loading')} />
        </div>
      ) : error ? (
        <ErrorState message={t('community.hub.buildingsError')} />
      ) : !buildingId ? (
        <EmptyState
          title={t('community.hub.noBuildingTitle')}
          body={t('community.hub.noBuildingBody')}
        />
      ) : (
        children(buildingId)
      )}
    </div>
  );
}
