/**
 * HeroSearch — gradient hero block for the Reality Portal home page.
 *
 * Renders the brand gradient hero with H1 + subtitle, a tab strip
 * (Predaj / Prenájom / Novostavby), a 4-field search panel (Where,
 * Type, Rooms, Price) and a Search button.
 *
 * Reference: reality-web/home.html `.hero` + `.search-panel`
 */

import React, { useState } from 'react';
import styles from './HeroSearch.module.css';

export type HeroSearchTab = 'sale' | 'rent' | 'new-builds';

export interface HeroSearchValues {
  tab: HeroSearchTab;
  where: string;
  type: string;
  rooms: string;
  price: string;
}

export interface HeroSearchTab_Config {
  key: HeroSearchTab;
  label: string;
}

export interface HeroSearchProps {
  /** Heading text. Defaults to the standard Reality Portal tagline. */
  heading?: string;
  /** Subtitle text. */
  subtitle?: string;
  /** Initial active tab. Default: 'sale'. */
  defaultTab?: HeroSearchTab;
  /** Tab labels (localised). */
  tabs?: HeroSearchTab_Config[];
  /** Placeholder labels for each search field (localised). */
  fieldLabels?: {
    where?: string;
    type?: string;
    rooms?: string;
    price?: string;
    searchBtn?: string;
  };
  /** Called when the user clicks Search with current field values. */
  onSearch: (values: HeroSearchValues) => void;
  className?: string;
}

const DEFAULT_TABS: HeroSearchTab_Config[] = [
  { key: 'sale',       label: 'Predaj' },
  { key: 'rent',       label: 'Prenájom' },
  { key: 'new-builds', label: 'Novostavby' },
];

const SearchIcon = () => (
  <svg
    width="16"
    height="16"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2.5"
    strokeLinecap="round"
    aria-hidden="true"
  >
    <circle cx="11" cy="11" r="8" />
    <line x1="21" y1="21" x2="16.65" y2="16.65" />
  </svg>
);

/** HeroSearch: gradient hero with search panel for Reality Portal home page. */
export const HeroSearch: React.FC<HeroSearchProps> = ({
  heading = 'Find your place in Bratislava, Prague, or Vienna.',
  subtitle = 'Every listing on Reality Portal is verified, with a building passport, a clear price history, and a real agent on the other end.',
  defaultTab = 'sale',
  tabs = DEFAULT_TABS,
  fieldLabels = {},
  onSearch,
  className,
}) => {
  const [activeTab, setActiveTab] = useState<HeroSearchTab>(defaultTab);
  const [where, setWhere] = useState('');
  const [type, setType] = useState('');
  const [rooms, setRooms] = useState('');
  const [price, setPrice] = useState('');

  const labels = {
    where:     fieldLabels.where     ?? 'Where',
    type:      fieldLabels.type      ?? 'Type',
    rooms:     fieldLabels.rooms     ?? 'Rooms',
    price:     fieldLabels.price     ?? 'Price',
    searchBtn: fieldLabels.searchBtn ?? 'Search',
  };

  const handleSearch = () => {
    onSearch({ tab: activeTab, where, type, rooms, price });
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') handleSearch();
  };

  return (
    <section className={[styles.hero, className].filter(Boolean).join(' ')}>
      <div className={styles.inner}>
        <h1 className={styles.heading}>{heading}</h1>
        <p className={styles.subtitle}>{subtitle}</p>

        <div className={styles.tabsWrapper}>
          {/* Tab strip — positioned above the search panel */}
          <div className={styles.tabs} role="tablist" aria-label="Listing type">
            {tabs.map((t) => (
              <button
                key={t.key}
                role="tab"
                aria-selected={activeTab === t.key}
                className={[styles.tab, activeTab === t.key ? styles.tabActive : ''].filter(Boolean).join(' ')}
                onClick={() => setActiveTab(t.key)}
                type="button"
              >
                {t.label}
              </button>
            ))}
          </div>

          {/* Search panel */}
          <div className={styles.searchPanel} role="search" aria-label="Property search">
            <div className={styles.searchField} style={{ flex: '1.4' }}>
              <label className={styles.fieldLabel} htmlFor="hs-where">{labels.where}</label>
              <input
                id="hs-where"
                className={styles.fieldInput}
                type="text"
                placeholder="City or neighbourhood"
                value={where}
                onChange={(e) => setWhere(e.target.value)}
                onKeyDown={handleKeyDown}
                aria-label={labels.where}
              />
            </div>

            <div className={styles.searchField}>
              <label className={styles.fieldLabel} htmlFor="hs-type">{labels.type}</label>
              <input
                id="hs-type"
                className={styles.fieldInput}
                type="text"
                placeholder="Apartment, house…"
                value={type}
                onChange={(e) => setType(e.target.value)}
                onKeyDown={handleKeyDown}
                aria-label={labels.type}
              />
            </div>

            <div className={styles.searchField}>
              <label className={styles.fieldLabel} htmlFor="hs-rooms">{labels.rooms}</label>
              <input
                id="hs-rooms"
                className={styles.fieldInput}
                type="text"
                placeholder="2 – 4 rooms"
                value={rooms}
                onChange={(e) => setRooms(e.target.value)}
                onKeyDown={handleKeyDown}
                aria-label={labels.rooms}
              />
            </div>

            <div className={styles.searchField}>
              <label className={styles.fieldLabel} htmlFor="hs-price">{labels.price}</label>
              <input
                id="hs-price"
                className={styles.fieldInput}
                type="text"
                placeholder="Up to €450k"
                value={price}
                onChange={(e) => setPrice(e.target.value)}
                onKeyDown={handleKeyDown}
                aria-label={labels.price}
              />
            </div>

            <button
              className={styles.searchBtn}
              onClick={handleSearch}
              type="button"
              aria-label={labels.searchBtn}
            >
              <SearchIcon />
              {labels.searchBtn}
            </button>
          </div>
        </div>
      </div>
    </section>
  );
};

export default HeroSearch;
