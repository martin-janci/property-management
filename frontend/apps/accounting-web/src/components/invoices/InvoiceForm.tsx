/**
 * Invoice create/issue editor (UC-ACC-05.1, .2, .19, .20).
 *
 * STORY-ACC-05-001 + 002 AC covered:
 *  - select a contact + set issue/due/supply dates → save as draft
 *  - add line items (description, qty, unit price, VAT rate) with live totals
 *  - net / VAT (grouped per rate) / gross recomputed live on any change
 *  - "Save draft" vs "Issue"; issue blocked with field-level validation when
 *    a contact is missing or there are no lines
 *
 * The form builds a `CreateInvoice` draft, persists via `useCreateInvoice`,
 * then (for "Issue") calls `useIssueInvoice` which the backend handles
 * atomically (allocate gapless number → format → lock).
 */

'use client';

import { Alert, Button, Input, Spinner } from '@ppt/ui-kit';
import { useTranslations } from 'next-intl';
import { useMemo, useState } from 'react';
import { useContacts } from '@/components/contacts/api';
import { useRouter } from '@/i18n/routing';
import { useCreateInvoice, useIssueInvoice } from './api';
import styles from './app.module.css';
import { computeDocumentTotals, computeLine, formatMoney } from './totals';
import type { CreateInvoiceItem } from './types';

interface DraftLine extends CreateInvoiceItem {
  /** stable client-side key for React */
  key: string;
}

const CURRENCIES = ['EUR', 'CZK', 'USD'] as const;
const DEFAULT_VAT = '20';

function today(): string {
  return new Date().toISOString().slice(0, 10);
}
function addDays(iso: string, days: number): string {
  const d = new Date(iso);
  d.setDate(d.getDate() + days);
  return d.toISOString().slice(0, 10);
}
function emptyLine(): DraftLine {
  return {
    key: crypto.randomUUID(),
    description: '',
    qty: '1',
    unit_price: '0',
    vat_rate: DEFAULT_VAT,
  };
}

type FieldErrors = {
  contact?: string;
  lines?: string;
  due?: string;
};

export function InvoiceForm({ locale }: { locale: string }) {
  const t = useTranslations('app.invoices.form');
  const tt = useTranslations('app.invoices.totals');
  const tc = useTranslations('app.common');
  const router = useRouter();

  const contactsQuery = useContacts();
  const createInvoice = useCreateInvoice();
  const issueInvoice = useIssueInvoice();

  const [contactId, setContactId] = useState('');
  const [currency, setCurrency] = useState<string>('EUR');
  const [issueDate, setIssueDate] = useState(today());
  const [dueDate, setDueDate] = useState(addDays(today(), 14));
  const [supplyDate, setSupplyDate] = useState<string>(today());
  const [variableSymbol, setVariableSymbol] = useState('');
  const [lines, setLines] = useState<DraftLine[]>([emptyLine()]);
  const [errors, setErrors] = useState<FieldErrors>({});
  const [submitError, setSubmitError] = useState<string | null>(null);

  const totals = useMemo(() => computeDocumentTotals(lines), [lines]);
  const busy = createInvoice.isPending || issueInvoice.isPending;

  function updateLine(key: string, patch: Partial<CreateInvoiceItem>) {
    setLines((prev) => prev.map((l) => (l.key === key ? { ...l, ...patch } : l)));
  }
  function addLine() {
    setLines((prev) => [...prev, emptyLine()]);
  }
  function removeLine(key: string) {
    setLines((prev) => (prev.length > 1 ? prev.filter((l) => l.key !== key) : prev));
  }

  /** Validate. `forIssue` enforces the stricter issue gate (UC-ACC-05.1). */
  function validate(forIssue: boolean): FieldErrors {
    const next: FieldErrors = {};
    if (!contactId) next.contact = t('validation.contactRequired');
    const meaningful = lines.filter(
      (l) => l.description.trim() !== '' || Number.parseFloat(l.qty) > 0
    );
    if (forIssue && meaningful.length === 0) next.lines = t('validation.lineRequired');
    if (dueDate && issueDate && dueDate < issueDate) next.due = t('validation.dueAfterIssue');
    return next;
  }

  function buildCreatePayload() {
    const items: CreateInvoiceItem[] = lines
      .filter((l) => l.description.trim() !== '' || Number.parseFloat(l.qty) > 0)
      .map((l) => ({
        description: l.description,
        qty: l.qty,
        unit_price: l.unit_price,
        vat_rate: l.vat_rate,
      }));
    return {
      // tenant_id is enforced server-side from the JWT/RLS context; the field
      // exists on the DTO but the server overrides it to the active company.
      tenant_id: '00000000-0000-0000-0000-000000000000',
      contact_id: contactId,
      // Draft has no number; the backend assigns one only on issue.
      number: '',
      issue_date: issueDate,
      due_date: dueDate,
      taxable_supply_date: supplyDate || null,
      currency,
      variable_symbol: variableSymbol || null,
      status: 'draft' as const,
      items,
      country: locale === 'cs' ? 'CZ' : 'SK',
    };
  }

  async function onSaveDraft() {
    setSubmitError(null);
    const next = validate(false);
    setErrors(next);
    if (Object.keys(next).length > 0) return;
    try {
      const created = await createInvoice.mutateAsync(buildCreatePayload());
      router.push(`/invoices/${created.id}`);
    } catch {
      setSubmitError(tc('error'));
    }
  }

  async function onIssue() {
    setSubmitError(null);
    const next = validate(true);
    setErrors(next);
    if (Object.keys(next).length > 0) return;
    try {
      const created = await createInvoice.mutateAsync(buildCreatePayload());
      await issueInvoice.mutateAsync({ id: created.id, body: {} });
      router.push(`/invoices/${created.id}`);
    } catch {
      setSubmitError(tc('error'));
    }
  }

  const contacts = contactsQuery.data ?? [];

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h1 className={styles.title}>{t('newTitle')}</h1>
      </header>

      {submitError && <Alert variant="danger">{submitError}</Alert>}

      {/* Header fields */}
      <div className={styles.section}>
        <div className={styles.formGrid}>
          <div className={styles.field}>
            <label className={styles.label} htmlFor="acc-inv-contact">
              {t('contact')}
            </label>
            {contactsQuery.isLoading ? (
              <Spinner size="sm" />
            ) : contacts.length === 0 ? (
              <Alert variant="info">{t('noContacts')}</Alert>
            ) : (
              <select
                id="acc-inv-contact"
                className={styles.select}
                value={contactId}
                onChange={(e) => setContactId(e.target.value)}
              >
                <option value="">{t('contactPlaceholder')}</option>
                {contacts.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.name}
                    {c.ico ? ` · ${c.ico}` : ''}
                  </option>
                ))}
              </select>
            )}
            {errors.contact && <span className={styles.fieldError}>{errors.contact}</span>}
          </div>

          <div className={styles.field}>
            <label className={styles.label} htmlFor="acc-inv-currency">
              {t('currency')}
            </label>
            <select
              id="acc-inv-currency"
              className={styles.select}
              value={currency}
              onChange={(e) => setCurrency(e.target.value)}
            >
              {CURRENCIES.map((c) => (
                <option key={c} value={c}>
                  {c}
                </option>
              ))}
            </select>
          </div>

          <div className={styles.field}>
            <Input
              type="date"
              label={t('issueDate')}
              value={issueDate}
              onChange={(e) => setIssueDate(e.target.value)}
              fullWidth
            />
          </div>
          <div className={styles.field}>
            <Input
              type="date"
              label={t('dueDate')}
              value={dueDate}
              onChange={(e) => setDueDate(e.target.value)}
              error={errors.due}
              fullWidth
            />
          </div>
          <div className={styles.field}>
            <Input
              type="date"
              label={t('supplyDate')}
              value={supplyDate}
              onChange={(e) => setSupplyDate(e.target.value)}
              fullWidth
            />
          </div>
          <div className={styles.field}>
            <Input
              label={t('variableSymbol')}
              value={variableSymbol}
              onChange={(e) => setVariableSymbol(e.target.value)}
              inputMode="numeric"
              fullWidth
            />
          </div>
        </div>
      </div>

      {/* Line editor */}
      <div className={styles.section}>
        <h2 className={styles.sectionTitle}>{t('lines')}</h2>
        {errors.lines && <span className={styles.fieldError}>{errors.lines}</span>}
        <table className={styles.lineTable}>
          <thead>
            <tr>
              <th>{t('lineColumns.description')}</th>
              <th>{t('lineColumns.qty')}</th>
              <th>{t('lineColumns.unitPrice')}</th>
              <th>{t('lineColumns.vatRate')}</th>
              <th>{t('lineColumns.lineTotal')}</th>
              <th aria-label={tc('actions')} />
            </tr>
          </thead>
          <tbody>
            {lines.map((line) => {
              const lt = computeLine(line);
              return (
                <tr key={line.key}>
                  <td>
                    <input
                      className={styles.lineInput}
                      placeholder={t('linePlaceholder')}
                      value={line.description}
                      onChange={(e) => updateLine(line.key, { description: e.target.value })}
                    />
                  </td>
                  <td>
                    <input
                      className={`${styles.lineInput} ${styles.lineNum}`}
                      type="number"
                      min="0"
                      step="0.0001"
                      value={line.qty}
                      onChange={(e) => updateLine(line.key, { qty: e.target.value })}
                    />
                  </td>
                  <td>
                    <input
                      className={`${styles.lineInput} ${styles.lineNum}`}
                      type="number"
                      min="0"
                      step="0.0001"
                      value={line.unit_price}
                      onChange={(e) => updateLine(line.key, { unit_price: e.target.value })}
                    />
                  </td>
                  <td>
                    <input
                      className={`${styles.lineInput} ${styles.lineNum}`}
                      type="number"
                      min="0"
                      step="1"
                      value={line.vat_rate}
                      onChange={(e) => updateLine(line.key, { vat_rate: e.target.value })}
                    />
                  </td>
                  <td className={styles.lineNum}>{formatMoney(lt.gross, currency, locale)}</td>
                  <td>
                    <button
                      type="button"
                      className={styles.lineRemove}
                      onClick={() => removeLine(line.key)}
                      aria-label={t('removeLine')}
                      disabled={lines.length <= 1}
                    >
                      ×
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
        <div style={{ marginTop: 'var(--ppt-space-3)' }}>
          <Button variant="secondary" size="sm" onClick={addLine}>
            + {t('addLine')}
          </Button>
        </div>

        {/* Live totals + VAT breakdown by rate */}
        <div className={styles.totals}>
          <div className={styles.totalsRow}>
            <span>{tt('net')}</span>
            <span>{formatMoney(totals.base, currency, locale)}</span>
          </div>
          <div className={styles.totalsRow}>
            <span>{tt('vat')}</span>
            <span>{formatMoney(totals.vat, currency, locale)}</span>
          </div>
          <div className={`${styles.totalsRow} ${styles.totalsGrand}`}>
            <span>{tt('gross')}</span>
            <span>{formatMoney(totals.gross, currency, locale)}</span>
          </div>
          {totals.groups.length > 0 && (
            <div className={styles.vatBreakdown}>
              <strong>{tt('vatBreakdown')}</strong>
              {totals.groups.map((g) => (
                <div className={styles.totalsRow} key={g.rate}>
                  <span>
                    {tt('rate')} {g.rate}% · {formatMoney(g.base, currency, locale)}
                  </span>
                  <span>{formatMoney(g.vat, currency, locale)}</span>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

      <div className={styles.formActions}>
        <Button variant="ghost" onClick={() => router.push('/invoices')} disabled={busy}>
          {tc('cancel')}
        </Button>
        <Button variant="secondary" onClick={onSaveDraft} disabled={busy}>
          {t('saveDraft')}
        </Button>
        <Button variant="primary" onClick={onIssue} disabled={busy}>
          {t('issue')}
        </Button>
      </div>
    </div>
  );
}
