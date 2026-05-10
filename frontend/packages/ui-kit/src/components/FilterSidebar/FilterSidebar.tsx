/**
 * FilterSidebar — 220px sticky vertical sidebar with collapsible filter groups.
 *
 * Each group has a title and a list of checkbox items with optional counts.
 * A "Clear" button resets all selections.
 *
 * Reference: ppt-documents.html `.fbar`
 */

import type React from 'react';
import { useId, useState } from 'react';
import styles from './FilterSidebar.module.css';

export interface FilterItem {
  /** Unique key for this item within its group */
  key: string;
  label: string;
  count?: number;
  checked: boolean;
  onToggle: (key: string, checked: boolean) => void;
}

export interface FilterGroup {
  /** Unique key for this group */
  key: string;
  title: string;
  items: FilterItem[];
  /** Whether the group starts expanded. Default: true. */
  defaultOpen?: boolean;
}

export interface FilterSidebarProps {
  groups: FilterGroup[];
  onClear: () => void;
  /** Sidebar heading. Default: "Filters". */
  title?: string;
  /** Number of active filters shown next to title. */
  activeCount?: number;
  className?: string;
}

const ChevronDownIcon = () => (
  <svg
    width="12"
    height="12"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2.5"
    strokeLinecap="round"
    strokeLinejoin="round"
    aria-hidden="true"
  >
    <polyline points="6 9 12 15 18 9" />
  </svg>
);

/** FilterSidebar: collapsible filter panel used in list/document views. */
export const FilterSidebar: React.FC<FilterSidebarProps> = ({
  groups,
  onClear,
  title = 'Filters',
  activeCount,
  className,
}) => {
  // Stable per-instance prefix — group.key is arbitrary (may contain spaces /
  // special chars unsafe for HTML id) and may collide across groups. Pair this
  // prefix with the group's array index for the actual id.
  const uid = useId();
  const groupId = (idx: number) => `${uid}group-${idx}`;

  const [openGroups, setOpenGroups] = useState<Record<string, boolean>>(() => {
    const initial: Record<string, boolean> = {};
    for (const g of groups) {
      initial[g.key] = g.defaultOpen !== false;
    }
    return initial;
  });

  const toggleGroup = (key: string) => {
    setOpenGroups((prev) => ({ ...prev, [key]: !prev[key] }));
  };

  return (
    <aside className={[styles.sidebar, className].filter(Boolean).join(' ')} aria-label={title}>
      {/* Header */}
      <div className={styles.head}>
        <h4 className={styles.headTitle}>
          {title}
          {activeCount != null && activeCount > 0 && (
            <span aria-label={`${activeCount} active`}> · {activeCount} active</span>
          )}
        </h4>
        <button
          className={styles.clearBtn}
          onClick={onClear}
          type="button"
          aria-label="Clear all filters"
        >
          Clear
        </button>
      </div>

      {/* Groups */}
      {groups.map((group, idx) => {
        const isOpen = openGroups[group.key] ?? true;
        const regionId = groupId(idx);
        return (
          <div key={group.key} className={styles.group}>
            <button
              className={styles.groupHeader}
              onClick={() => toggleGroup(group.key)}
              aria-expanded={isOpen}
              aria-controls={regionId}
              type="button"
            >
              {/* span (not heading): button content must be phrasing content;
                  headings are not allowed inside <button>. Visual size kept
                  via styles.groupTitle. */}
              <span className={styles.groupTitle}>{group.title}</span>
              <span
                className={[styles.chevron, isOpen ? styles.chevronOpen : '']
                  .filter(Boolean)
                  .join(' ')}
              >
                <ChevronDownIcon />
              </span>
            </button>

            {/* Always render the controlled region so aria-controls always
                points at an existing id; toggle visibility with `hidden`. */}
            <div
              id={regionId}
              className={styles.groupItems}
              role="group"
              aria-label={group.title}
              hidden={!isOpen}
            >
              {group.items.map((item) => (
                <label key={item.key} className={styles.item}>
                  <input
                    type="checkbox"
                    className={styles.itemCheck}
                    checked={item.checked}
                    onChange={(e) => item.onToggle(item.key, e.target.checked)}
                    aria-label={item.label}
                  />
                  <span className={styles.itemLabel}>{item.label}</span>
                  {item.count != null && <span className={styles.itemCount}>{item.count}</span>}
                </label>
              ))}
            </div>
          </div>
        );
      })}
    </aside>
  );
};

export default FilterSidebar;
