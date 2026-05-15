/**
 * Generic admin CRUD table.
 *
 * Pre-TanStack-Table: a deliberately simple component that the unified
 * super-admin console uses for every resource list (agencies, users, audit
 * rows, etc). Per-row actions are capability-gated — pass `capability` on
 * each action and we hide the button when the current principal lacks it.
 */

import type { Key, ReactNode } from 'react';
import type { Capability } from '../../capabilities';
import { useCapability } from '../../hooks/useCapability';

export interface ResourceTableColumn<T> {
  /** Stable column key. Used as React key + sort discriminator. */
  key: string;
  /** Header label rendered in <th>. */
  header: string;
  /** Cell renderer; receives the row item. */
  render: (item: T) => ReactNode;
  /** Optional CSS class for the cell. */
  className?: string;
}

export interface ResourceTableAction<T> {
  /** Label rendered as the button text. */
  label: string;
  /** Capability required to render this action. UI hides the button when missing. */
  capability: Capability;
  /** Click handler — receives the row item. */
  onClick: (item: T) => void;
  /** Variant; defaults to 'secondary'. 'danger' for destructive actions. */
  variant?: 'primary' | 'secondary' | 'danger';
}

export interface ResourceTableProps<T> {
  /** Column definitions. */
  columns: ReadonlyArray<ResourceTableColumn<T>>;
  /** Async loader. Called on mount + when explicitly refreshed. */
  data: ReadonlyArray<T>;
  /** Stable row key extractor. */
  rowKey: (item: T) => Key;
  /** Per-row actions. Hidden when the current principal lacks the capability. */
  actions?: ReadonlyArray<ResourceTableAction<T>>;
  /** Optional empty-state message. */
  emptyMessage?: string;
  /** Optional table caption (a11y). */
  caption?: string;
}

export function ResourceTable<T>({
  columns,
  data,
  rowKey,
  actions = [],
  emptyMessage = 'No items.',
  caption,
}: ResourceTableProps<T>) {
  return (
    <table className="ppt-admin-resource-table">
      {caption ? <caption>{caption}</caption> : null}
      <thead>
        <tr>
          {columns.map((c) => (
            <th key={c.key} scope="col">
              {c.header}
            </th>
          ))}
          {actions.length > 0 ? <th scope="col">Actions</th> : null}
        </tr>
      </thead>
      <tbody>
        {data.length === 0 ? (
          <tr>
            <td colSpan={columns.length + (actions.length > 0 ? 1 : 0)}>
              <em>{emptyMessage}</em>
            </td>
          </tr>
        ) : (
          data.map((item) => (
            <tr key={rowKey(item)}>
              {columns.map((c) => (
                <td key={c.key} className={c.className}>
                  {c.render(item)}
                </td>
              ))}
              {actions.length > 0 ? (
                <td>
                  {actions.map((a) => (
                    <ActionButton key={a.label} action={a} item={item} />
                  ))}
                </td>
              ) : null}
            </tr>
          ))
        )}
      </tbody>
    </table>
  );
}

/** Capability-gated action button. Returns null (hides UI) when the user
 *  lacks the capability — never renders a disabled button (information
 *  disclosure, leak #21). */
function ActionButton<T>({
  action,
  item,
}: {
  action: ResourceTableAction<T>;
  item: T;
}) {
  const allowed = useCapability(action.capability);
  if (!allowed) return null;
  return (
    <button
      type="button"
      className={`ppt-admin-action ppt-admin-action--${action.variant ?? 'secondary'}`}
      onClick={() => action.onClick(item)}
    >
      {action.label}
    </button>
  );
}
