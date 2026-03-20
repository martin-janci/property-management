/**
 * Table component for data display.
 */

import type React from 'react';
import { forwardRef } from 'react';
import './Table.css';

export interface TableProps extends React.TableHTMLAttributes<HTMLTableElement> {
  /** Table variant */
  variant?: 'default' | 'striped' | 'bordered';
  /** Whether rows are hoverable */
  hoverable?: boolean;
  /** Whether the table is compact */
  compact?: boolean;
  /** Make table horizontally scrollable on small screens */
  responsive?: boolean;
}

export interface TableHeadProps extends React.HTMLAttributes<HTMLTableSectionElement> {}
export interface TableBodyProps extends React.HTMLAttributes<HTMLTableSectionElement> {}
export interface TableRowProps extends React.HTMLAttributes<HTMLTableRowElement> {
  /** Whether this row is selected */
  selected?: boolean;
}
export interface TableCellProps extends React.TdHTMLAttributes<HTMLTableCellElement> {
  /** Text alignment */
  align?: 'left' | 'center' | 'right';
}
export interface TableHeaderCellProps extends React.ThHTMLAttributes<HTMLTableCellElement> {
  /** Text alignment */
  align?: 'left' | 'center' | 'right';
  /** Whether this column is sortable */
  sortable?: boolean;
  /** Current sort direction */
  sortDirection?: 'asc' | 'desc' | null;
  /** Callback when sort is clicked */
  onSort?: () => void;
}

/**
 * Table component for displaying tabular data.
 *
 * @example
 * ```tsx
 * <Table striped hoverable>
 *   <Table.Head>
 *     <Table.Row>
 *       <Table.HeaderCell>Name</Table.HeaderCell>
 *       <Table.HeaderCell>Email</Table.HeaderCell>
 *     </Table.Row>
 *   </Table.Head>
 *   <Table.Body>
 *     <Table.Row>
 *       <Table.Cell>John Doe</Table.Cell>
 *       <Table.Cell>john@example.com</Table.Cell>
 *     </Table.Row>
 *   </Table.Body>
 * </Table>
 * ```
 */
export const Table = forwardRef<HTMLTableElement, TableProps>(
  ({ variant = 'default', hoverable = false, compact = false, responsive = true, className = '', children, ...props }, ref) => {
    const tableClasses = [
      'ppt-table',
      `ppt-table--${variant}`,
      hoverable && 'ppt-table--hoverable',
      compact && 'ppt-table--compact',
      className,
    ]
      .filter(Boolean)
      .join(' ');

    const table = (
      <table ref={ref} className={tableClasses} {...props}>
        {children}
      </table>
    );

    if (responsive) {
      return <div className="ppt-table-container">{table}</div>;
    }

    return table;
  }
) as React.ForwardRefExoticComponent<TableProps & React.RefAttributes<HTMLTableElement>> & {
  Head: typeof TableHead;
  Body: typeof TableBody;
  Row: typeof TableRow;
  Cell: typeof TableCell;
  HeaderCell: typeof TableHeaderCell;
};

Table.displayName = 'Table';

const TableHead = forwardRef<HTMLTableSectionElement, TableHeadProps>(
  ({ className = '', children, ...props }, ref) => {
    const classes = ['ppt-table__head', className].filter(Boolean).join(' ');
    return (
      <thead ref={ref} className={classes} {...props}>
        {children}
      </thead>
    );
  }
);

TableHead.displayName = 'Table.Head';

const TableBody = forwardRef<HTMLTableSectionElement, TableBodyProps>(
  ({ className = '', children, ...props }, ref) => {
    const classes = ['ppt-table__body', className].filter(Boolean).join(' ');
    return (
      <tbody ref={ref} className={classes} {...props}>
        {children}
      </tbody>
    );
  }
);

TableBody.displayName = 'Table.Body';

const TableRow = forwardRef<HTMLTableRowElement, TableRowProps>(
  ({ selected = false, className = '', children, ...props }, ref) => {
    const classes = ['ppt-table__row', selected && 'ppt-table__row--selected', className]
      .filter(Boolean)
      .join(' ');
    return (
      <tr ref={ref} className={classes} aria-selected={selected || undefined} {...props}>
        {children}
      </tr>
    );
  }
);

TableRow.displayName = 'Table.Row';

const TableCell = forwardRef<HTMLTableCellElement, TableCellProps>(
  ({ align = 'left', className = '', children, ...props }, ref) => {
    const classes = ['ppt-table__cell', `ppt-table__cell--${align}`, className]
      .filter(Boolean)
      .join(' ');
    return (
      <td ref={ref} className={classes} {...props}>
        {children}
      </td>
    );
  }
);

TableCell.displayName = 'Table.Cell';

const TableHeaderCell = forwardRef<HTMLTableCellElement, TableHeaderCellProps>(
  ({ align = 'left', sortable = false, sortDirection, onSort, className = '', children, ...props }, ref) => {
    const classes = [
      'ppt-table__header-cell',
      `ppt-table__header-cell--${align}`,
      sortable && 'ppt-table__header-cell--sortable',
      className,
    ]
      .filter(Boolean)
      .join(' ');

    const content = sortable ? (
      <button
        type="button"
        className="ppt-table__sort-button"
        onClick={onSort}
        aria-sort={sortDirection === 'asc' ? 'ascending' : sortDirection === 'desc' ? 'descending' : undefined}
      >
        <span>{children}</span>
        <span className="ppt-table__sort-icon" aria-hidden="true">
          {sortDirection === 'asc' && (
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <polyline points="18 15 12 9 6 15" />
            </svg>
          )}
          {sortDirection === 'desc' && (
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <polyline points="6 9 12 15 18 9" />
            </svg>
          )}
          {!sortDirection && (
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" className="ppt-table__sort-icon--inactive">
              <polyline points="18 15 12 9 6 15" />
            </svg>
          )}
        </span>
      </button>
    ) : (
      children
    );

    return (
      <th ref={ref} className={classes} scope="col" {...props}>
        {content}
      </th>
    );
  }
);

TableHeaderCell.displayName = 'Table.HeaderCell';

// Attach sub-components
Table.Head = TableHead;
Table.Body = TableBody;
Table.Row = TableRow;
Table.Cell = TableCell;
Table.HeaderCell = TableHeaderCell;
