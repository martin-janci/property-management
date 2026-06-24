/**
 * Invoice list with status badges + search/status filters (UC-ACC-05.1).
 *
 * STORY-ACC-05-001 AC: lists invoices for the active company with their status;
 * "New invoice" routes to the create editor. Filtering is client-side over the
 * fetched page (the MVP list endpoint returns the company's invoices).
 */

'use client';

import { Button, Input, Spinner, Table } from '@ppt/ui-kit';
import { useTranslations } from 'next-intl';
import { useMemo, useState } from 'react';
import { useRouter } from '@/i18n/routing';
import { useInvoices } from './api';
import { InvoiceStatusBadge } from './InvoiceStatusBadge';
import { formatMoney } from './totals';
import type { AccInvoiceExt } from './types';
import styles from './app.module.css';

const STATUS_OPTIONS = [
  'draft',
  'issued',
  'sent',
  'paid',
  'partially_paid',
  'overdue',
  'cancelled',
] as const;

export function InvoiceList({ locale }: { locale: string }) {
  const t = useTranslations('app.invoices');
  const tc = useTranslations('app.common');
  const router = useRouter();
  const { data, isLoading, isError, refetch } = useInvoices();

  const [search, setSearch] = useState('');
  const [status, setStatus] = useState<string>('');

  const filtered = useMemo<AccInvoiceExt[]>(() => {
    const rows = data ?? [];
    const q = search.trim().toLowerCase();
    return rows.filter((inv) => {
      if (status && inv.status !== status) return false;
      if (!q) return true;
      return (
        inv.number.toLowerCase().includes(q) ||
        (inv.variable_symbol ?? '').toLowerCase().includes(q) ||
        inv.contact_id.toLowerCase().includes(q)
      );
    });
  }, [data, search, status]);

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <div>
          <h1 className={styles.title}>{t('list.title')}</h1>
          {data && (
            <p className={styles.subtitle}>{t('list.count', { count: data.length })}</p>
          )}
        </div>
        <Button variant="primary" onClick={() => router.push('/invoices/new')}>
          {t('list.newInvoice')}
        </Button>
      </header>

      <div className={styles.toolbar}>
        <div className={styles.toolbarGrow}>
          <Input
            type="search"
            placeholder={t('list.searchPlaceholder')}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            fullWidth
            aria-label={tc('search')}
          />
        </div>
        <select
          className={styles.select}
          value={status}
          onChange={(e) => setStatus(e.target.value)}
          aria-label={t('columns.status')}
        >
          <option value="">{t('list.filterAll')}</option>
          {STATUS_OPTIONS.map((s) => (
            <option key={s} value={s}>
              {t(`status.${s}`)}
            </option>
          ))}
        </select>
      </div>

      {isLoading && (
        <div className={styles.state}>
          <Spinner />
        </div>
      )}

      {isError && (
        <div className={styles.state}>
          <p>{tc('error')}</p>
          <Button variant="secondary" onClick={() => refetch()}>
            {tc('retry')}
          </Button>
        </div>
      )}

      {!isLoading && !isError && filtered.length === 0 && (
        <div className={styles.empty}>
          <p className={styles.emptyTitle}>{t('list.empty')}</p>
          <p>{t('list.emptyHint')}</p>
        </div>
      )}

      {!isLoading && !isError && filtered.length > 0 && (
        <Table hoverable responsive>
          <Table.Head>
            <Table.Row>
              <Table.HeaderCell>{t('columns.number')}</Table.HeaderCell>
              <Table.HeaderCell>{t('columns.contact')}</Table.HeaderCell>
              <Table.HeaderCell>{t('columns.issueDate')}</Table.HeaderCell>
              <Table.HeaderCell>{t('columns.dueDate')}</Table.HeaderCell>
              <Table.HeaderCell align="right">{t('columns.total')}</Table.HeaderCell>
              <Table.HeaderCell>{t('columns.status')}</Table.HeaderCell>
            </Table.Row>
          </Table.Head>
          <Table.Body>
            {filtered.map((inv) => (
              <Table.Row key={inv.id}>
                <Table.Cell className={styles.numberCell}>
                  <button
                    type="button"
                    className={styles.rowLink}
                    onClick={() => router.push(`/invoices/${inv.id}`)}
                  >
                    {inv.number || '—'}
                  </button>
                </Table.Cell>
                <Table.Cell>{inv.contact_id}</Table.Cell>
                <Table.Cell>{inv.issue_date}</Table.Cell>
                <Table.Cell>{inv.due_date}</Table.Cell>
                <Table.Cell align="right" className={styles.amountCell}>
                  {formatMoney(Number.parseFloat(inv.total_amount) || 0, inv.currency, locale)}
                </Table.Cell>
                <Table.Cell>
                  <InvoiceStatusBadge status={inv.status} />
                </Table.Cell>
              </Table.Row>
            ))}
          </Table.Body>
        </Table>
      )}
    </div>
  );
}
