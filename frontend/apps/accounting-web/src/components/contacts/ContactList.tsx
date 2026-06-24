/**
 * Contact directory list with search + kind filter (UC-ACC-03.1, .6).
 *
 * STORY-ACC-03-001 AC: lists customers/suppliers for the active company;
 * deactivated contacts are visually marked. "New contact" routes to the form.
 */

'use client';

import { Badge, Button, Input, Spinner, Table } from '@ppt/ui-kit';
import { useTranslations } from 'next-intl';
import { useMemo, useState } from 'react';
import { useRouter } from '@/i18n/routing';
// Shared screen styles live in the invoices component dir (both dirs are
// FE-Screens-owned); reuse to avoid duplicating tokens.
import styles from '@/components/invoices/app.module.css';
import { useContacts } from './api';
import type { AccContactExt } from './types';

const KIND_OPTIONS = ['customer', 'supplier', 'both'] as const;

export function ContactList() {
  const t = useTranslations('app.contacts');
  const tc = useTranslations('app.common');
  const router = useRouter();
  const { data, isLoading, isError, refetch } = useContacts();

  const [search, setSearch] = useState('');
  const [kind, setKind] = useState('');

  const filtered = useMemo<AccContactExt[]>(() => {
    const rows = data ?? [];
    const q = search.trim().toLowerCase();
    return rows.filter((c) => {
      if (kind && c.kind !== kind) return false;
      if (!q) return true;
      return (
        c.name.toLowerCase().includes(q) ||
        (c.ico ?? '').toLowerCase().includes(q) ||
        (c.email ?? '').toLowerCase().includes(q)
      );
    });
  }, [data, search, kind]);

  function kindLabel(value: string): string {
    if (value === 'customer' || value === 'supplier' || value === 'both') {
      return t(`kind.${value}`);
    }
    return value;
  }

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h1 className={styles.title}>{t('list.title')}</h1>
        <Button variant="primary" onClick={() => router.push('/contacts/new')}>
          {t('list.newContact')}
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
          value={kind}
          onChange={(e) => setKind(e.target.value)}
          aria-label={t('columns.kind')}
        >
          <option value="">{t('list.filterAll')}</option>
          {KIND_OPTIONS.map((k) => (
            <option key={k} value={k}>
              {t(`kind.${k}`)}
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
              <Table.HeaderCell>{t('columns.name')}</Table.HeaderCell>
              <Table.HeaderCell>{t('columns.kind')}</Table.HeaderCell>
              <Table.HeaderCell>{t('columns.ico')}</Table.HeaderCell>
              <Table.HeaderCell>{t('columns.email')}</Table.HeaderCell>
              <Table.HeaderCell>{t('columns.phone')}</Table.HeaderCell>
              <Table.HeaderCell>{t('columns.active')}</Table.HeaderCell>
            </Table.Row>
          </Table.Head>
          <Table.Body>
            {filtered.map((c) => (
              <Table.Row key={c.id}>
                <Table.Cell>
                  <button
                    type="button"
                    className={styles.rowLink}
                    onClick={() => router.push(`/contacts/${c.id}`)}
                  >
                    {c.name}
                  </button>
                </Table.Cell>
                <Table.Cell>{kindLabel(c.kind)}</Table.Cell>
                <Table.Cell>{c.ico ?? tc('none')}</Table.Cell>
                <Table.Cell>{c.email ?? tc('none')}</Table.Cell>
                <Table.Cell>{c.phone ?? tc('none')}</Table.Cell>
                <Table.Cell>
                  {c.is_active ? (
                    <Badge variant="success" rounded>
                      {t('columns.active')}
                    </Badge>
                  ) : (
                    <Badge variant="secondary" rounded>
                      {t('form.deactivate')}
                    </Badge>
                  )}
                </Table.Cell>
              </Table.Row>
            ))}
          </Table.Body>
        </Table>
      )}
    </div>
  );
}
