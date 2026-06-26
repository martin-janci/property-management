/**
 * Invoice detail — read-only view of an issued/draft invoice with its line
 * items, plus Duplicate (UC-ACC-05.16) and Issue (UC-ACC-05.1) actions.
 *
 * Issued invoices are immutable here (no edit affordance) per
 * STORY-ACC-05-001 AC; corrections happen via a corrective document.
 */

'use client';

import { Button, Spinner, Table } from '@ppt/ui-kit';
import { useTranslations } from 'next-intl';
import { useRouter } from '@/i18n/routing';
import { useDuplicateInvoice, useInvoice, useInvoiceItems, useIssueInvoice } from './api';
import styles from './app.module.css';
import { InvoiceStatusBadge } from './InvoiceStatusBadge';
import { formatMoney } from './totals';

export function InvoiceDetail({ id, locale }: { id: string; locale: string }) {
  const t = useTranslations('app.invoices');
  const tt = useTranslations('app.invoices.totals');
  const tc = useTranslations('app.common');
  const router = useRouter();

  const invoiceQuery = useInvoice(id);
  const itemsQuery = useInvoiceItems(id);
  const duplicate = useDuplicateInvoice();
  const issue = useIssueInvoice();

  if (invoiceQuery.isLoading) {
    return (
      <div className={styles.state}>
        <Spinner />
      </div>
    );
  }
  if (invoiceQuery.isError || !invoiceQuery.data) {
    return (
      <div className={styles.state}>
        <p>{tc('error')}</p>
        <Button variant="secondary" onClick={() => router.push('/invoices')}>
          {t('detail.backToList')}
        </Button>
      </div>
    );
  }

  const inv = invoiceQuery.data;
  const isDraft = inv.status === 'draft';
  const currency = inv.currency;

  async function onDuplicate() {
    const copy = await duplicate.mutateAsync(inv.id);
    router.push(`/invoices/${copy.id}`);
  }
  async function onIssue() {
    await issue.mutateAsync({ id: inv.id, body: {} });
  }

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <div>
          <h1 className={styles.title}>
            {t('detail.title', { number: inv.number || tc('none') })}
          </h1>
          <p className={styles.subtitle}>
            <InvoiceStatusBadge status={inv.status} />
          </p>
        </div>
        <div className={styles.formActions} style={{ margin: 0 }}>
          {isDraft && (
            <Button variant="primary" onClick={onIssue} disabled={issue.isPending}>
              {t('detail.issue')}
            </Button>
          )}
          <Button variant="secondary" onClick={onDuplicate} disabled={duplicate.isPending}>
            {t('detail.duplicate')}
          </Button>
          <Button variant="ghost" onClick={() => router.push('/invoices')}>
            {t('detail.backToList')}
          </Button>
        </div>
      </header>

      <div className={styles.formGrid}>
        <div className={styles.field}>
          <span className={styles.label}>{t('form.issueDate')}</span>
          <span>{inv.issue_date}</span>
        </div>
        <div className={styles.field}>
          <span className={styles.label}>{t('form.dueDate')}</span>
          <span>{inv.due_date}</span>
        </div>
        <div className={styles.field}>
          <span className={styles.label}>{t('form.variableSymbol')}</span>
          <span>{inv.variable_symbol ?? tc('none')}</span>
        </div>
        <div className={styles.field}>
          <span className={styles.label}>{t('form.currency')}</span>
          <span>{inv.currency}</span>
        </div>
      </div>

      <h2 className={styles.sectionTitle} style={{ marginTop: 'var(--ppt-space-6)' }}>
        {t('detail.items')}
      </h2>
      {itemsQuery.isLoading ? (
        <Spinner size="sm" />
      ) : (
        <Table responsive>
          <Table.Head>
            <Table.Row>
              <Table.HeaderCell>{t('form.lineColumns.description')}</Table.HeaderCell>
              <Table.HeaderCell align="right">{t('form.lineColumns.qty')}</Table.HeaderCell>
              <Table.HeaderCell align="right">{t('form.lineColumns.unitPrice')}</Table.HeaderCell>
              <Table.HeaderCell align="right">{t('form.lineColumns.vatRate')}</Table.HeaderCell>
              <Table.HeaderCell align="right">{t('form.lineColumns.lineTotal')}</Table.HeaderCell>
            </Table.Row>
          </Table.Head>
          <Table.Body>
            {(itemsQuery.data ?? []).map((it) => (
              <Table.Row key={it.id}>
                <Table.Cell>{it.description}</Table.Cell>
                <Table.Cell align="right">{it.qty}</Table.Cell>
                <Table.Cell align="right">
                  {formatMoney(Number.parseFloat(it.unit_price) || 0, currency, locale)}
                </Table.Cell>
                <Table.Cell align="right">{it.vat_rate}%</Table.Cell>
                <Table.Cell align="right">
                  {formatMoney(Number.parseFloat(it.total_amount) || 0, currency, locale)}
                </Table.Cell>
              </Table.Row>
            ))}
          </Table.Body>
        </Table>
      )}

      <div className={styles.totals}>
        <div className={styles.totalsRow}>
          <span>{tt('net')}</span>
          <span>{formatMoney(Number.parseFloat(inv.base_amount) || 0, currency, locale)}</span>
        </div>
        <div className={styles.totalsRow}>
          <span>{tt('vat')}</span>
          <span>{formatMoney(Number.parseFloat(inv.vat_amount) || 0, currency, locale)}</span>
        </div>
        <div className={`${styles.totalsRow} ${styles.totalsGrand}`}>
          <span>{tt('gross')}</span>
          <span>{formatMoney(Number.parseFloat(inv.total_amount) || 0, currency, locale)}</span>
        </div>
      </div>
    </div>
  );
}
