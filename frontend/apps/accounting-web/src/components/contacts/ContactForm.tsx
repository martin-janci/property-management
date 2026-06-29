/**
 * Contact create/edit form (UC-ACC-03.1, .4, .6, .9).
 *
 * STORY-ACC-03-001 + 003 + 004 AC covered:
 *  - create/edit a contact with identifiers (name, IČO, DIČ, IBAN) and a
 *    customer/supplier/both kind
 *  - billing address fields (flattened into the base `address` line for the
 *    MVP CRUD surface — multi-address rows are a follow-up endpoint)
 *  - per-contact defaults (currency, price level, payment terms, language)
 *    that prefill documents (UC-ACC-03.9 → feeds EPIC-ACC-05)
 *  - tags entry (UC-ACC-03.6) captured in the UI; persisted via the contact
 *    tags endpoint when it lands (see contract-notes/fe-screens.md)
 *
 * Persists via `useCreateContact` / `useUpdateContact` (ContactRequest DTO).
 */

'use client';

import { Alert, Button, Input, Spinner } from '@ppt/ui-kit';
import { useTranslations } from 'next-intl';
import { type FormEvent, useEffect, useState } from 'react';
import styles from '@/components/invoices/app.module.css';
import { useRouter } from '@/i18n/routing';
import { useContact, useCreateContact, useUpdateContact } from './api';
import type { ContactKind, ContactRequest } from './types';

const KINDS: ContactKind[] = ['customer', 'supplier', 'both'];
const CURRENCIES = ['EUR', 'CZK', 'USD'] as const;
const LANGUAGES = ['sk', 'cs', 'en', 'de'] as const;

type FormState = {
  name: string;
  kind: ContactKind;
  ico: string;
  dic: string;
  email: string;
  phone: string;
  address: string;
  iban: string;
  default_currency: string;
  default_price_level: string;
  default_payment_terms_days: string;
  default_language: string;
};

const EMPTY: FormState = {
  name: '',
  kind: 'customer',
  ico: '',
  dic: '',
  email: '',
  phone: '',
  address: '',
  iban: '',
  default_currency: '',
  default_price_level: '',
  default_payment_terms_days: '',
  default_language: '',
};

type Errors = Partial<Record<'name' | 'email', string>>;

/** `contactId` undefined → create mode; otherwise edit mode. */
export function ContactForm({ contactId }: { contactId?: string }) {
  const t = useTranslations('app.contacts.form');
  const tk = useTranslations('app.contacts.kind');
  const tc = useTranslations('app.common');
  const router = useRouter();

  const isEdit = Boolean(contactId);
  const existing = useContact(contactId);
  const createContact = useCreateContact();
  const updateContact = useUpdateContact(contactId ?? '');

  const [form, setForm] = useState<FormState>(EMPTY);
  const [tags, setTags] = useState<string[]>([]);
  const [tagDraft, setTagDraft] = useState('');
  const [errors, setErrors] = useState<Errors>({});
  const [submitError, setSubmitError] = useState<string | null>(null);

  // Hydrate the form once the existing contact loads (edit mode).
  useEffect(() => {
    const c = existing.data;
    if (!c) return;
    setForm({
      name: c.name,
      kind: (c.kind as ContactKind) ?? 'customer',
      ico: c.ico ?? '',
      dic: c.dic ?? '',
      email: c.email ?? '',
      phone: c.phone ?? '',
      address: c.address ?? '',
      iban: c.iban ?? '',
      default_currency: c.default_currency ?? '',
      default_price_level: c.default_price_level ?? '',
      default_payment_terms_days:
        c.default_payment_terms_days != null ? String(c.default_payment_terms_days) : '',
      default_language: c.default_language ?? '',
    });
  }, [existing.data]);

  function set<K extends keyof FormState>(key: K, value: FormState[K]) {
    setForm((prev) => ({ ...prev, [key]: value }));
  }

  function addTag() {
    const v = tagDraft.trim();
    if (v && !tags.includes(v)) setTags((prev) => [...prev, v]);
    setTagDraft('');
  }
  function removeTag(tag: string) {
    setTags((prev) => prev.filter((x) => x !== tag));
  }

  function validate(): Errors {
    const next: Errors = {};
    if (!form.name.trim()) next.name = t('validation.nameRequired');
    if (form.email && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(form.email))
      next.email = t('validation.emailInvalid');
    return next;
  }

  function toRequest(): ContactRequest {
    const termsRaw = form.default_payment_terms_days.trim();
    const terms = termsRaw === '' ? null : Number.parseInt(termsRaw, 10);
    return {
      name: form.name.trim(),
      kind: form.kind,
      ico: form.ico || null,
      dic: form.dic || null,
      email: form.email || null,
      phone: form.phone || null,
      address: form.address || null,
      iban: form.iban || null,
      default_currency: form.default_currency || null,
      default_price_level: form.default_price_level || null,
      default_payment_terms_days: Number.isFinite(terms as number) ? (terms as number) : null,
      default_language: form.default_language || null,
    };
  }

  async function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setSubmitError(null);
    const next = validate();
    setErrors(next);
    if (Object.keys(next).length > 0) return;
    try {
      if (isEdit && contactId) {
        await updateContact.mutateAsync(toRequest());
        router.push(`/contacts/${contactId}`);
      } else {
        const created = await createContact.mutateAsync(toRequest());
        router.push(`/contacts/${created.id}`);
      }
    } catch {
      setSubmitError(tc('error'));
    }
  }

  if (isEdit && existing.isLoading) {
    return (
      <div className={styles.state}>
        <Spinner />
      </div>
    );
  }

  const busy = createContact.isPending || updateContact.isPending;

  return (
    <div className={styles.page}>
      <header className={styles.header}>
        <h1 className={styles.title}>{isEdit ? t('editTitle') : t('newTitle')}</h1>
      </header>

      {submitError && <Alert variant="danger">{submitError}</Alert>}

      <form onSubmit={onSubmit} noValidate>
        {/* Identity */}
        <div className={styles.section}>
          <h2 className={styles.sectionTitle}>{t('sectionIdentity')}</h2>
          <div className={styles.formGrid}>
            <div className={`${styles.field} ${styles.formGridFull}`}>
              <Input
                label={t('name')}
                value={form.name}
                onChange={(e) => set('name', e.target.value)}
                error={errors.name}
                required
                fullWidth
              />
            </div>
            <div className={styles.field}>
              <label className={styles.label} htmlFor="acc-contact-kind">
                {t('kind')}
              </label>
              <select
                id="acc-contact-kind"
                className={styles.select}
                value={form.kind}
                onChange={(e) => set('kind', e.target.value as ContactKind)}
              >
                {KINDS.map((k) => (
                  <option key={k} value={k}>
                    {tk(k)}
                  </option>
                ))}
              </select>
            </div>
            <div className={styles.field}>
              <Input
                label={t('ico')}
                value={form.ico}
                onChange={(e) => set('ico', e.target.value)}
                fullWidth
              />
            </div>
            <div className={styles.field}>
              <Input
                label={t('dic')}
                value={form.dic}
                onChange={(e) => set('dic', e.target.value)}
                fullWidth
              />
            </div>
            <div className={styles.field}>
              <Input
                label={t('iban')}
                value={form.iban}
                onChange={(e) => set('iban', e.target.value)}
                fullWidth
              />
            </div>
            <div className={styles.field}>
              <Input
                type="email"
                label={t('email')}
                value={form.email}
                onChange={(e) => set('email', e.target.value)}
                error={errors.email}
                fullWidth
              />
            </div>
            <div className={styles.field}>
              <Input
                type="tel"
                label={t('phone')}
                value={form.phone}
                onChange={(e) => set('phone', e.target.value)}
                fullWidth
              />
            </div>
          </div>
        </div>

        {/* Address */}
        <div className={styles.section}>
          <h2 className={styles.sectionTitle}>{t('sectionAddress')}</h2>
          <div className={styles.formGrid}>
            <div className={`${styles.field} ${styles.formGridFull}`}>
              <Input
                label={t('address')}
                value={form.address}
                onChange={(e) => set('address', e.target.value)}
                fullWidth
              />
            </div>
          </div>
        </div>

        {/* Per-contact defaults (UC-ACC-03.9) */}
        <div className={styles.section}>
          <h2 className={styles.sectionTitle}>{t('sectionDefaults')}</h2>
          <div className={styles.formGrid}>
            <div className={styles.field}>
              <label className={styles.label} htmlFor="acc-contact-currency">
                {t('defaultCurrency')}
              </label>
              <select
                id="acc-contact-currency"
                className={styles.select}
                value={form.default_currency}
                onChange={(e) => set('default_currency', e.target.value)}
              >
                <option value="">{tc('none')}</option>
                {CURRENCIES.map((c) => (
                  <option key={c} value={c}>
                    {c}
                  </option>
                ))}
              </select>
            </div>
            <div className={styles.field}>
              <Input
                label={t('defaultPriceLevel')}
                value={form.default_price_level}
                onChange={(e) => set('default_price_level', e.target.value)}
                fullWidth
              />
            </div>
            <div className={styles.field}>
              <Input
                type="number"
                min="0"
                label={t('defaultPaymentTermsDays')}
                value={form.default_payment_terms_days}
                onChange={(e) => set('default_payment_terms_days', e.target.value)}
                fullWidth
              />
            </div>
            <div className={styles.field}>
              <label className={styles.label} htmlFor="acc-contact-lang">
                {t('defaultLanguage')}
              </label>
              <select
                id="acc-contact-lang"
                className={styles.select}
                value={form.default_language}
                onChange={(e) => set('default_language', e.target.value)}
              >
                <option value="">{tc('none')}</option>
                {LANGUAGES.map((l) => (
                  <option key={l} value={l}>
                    {l.toUpperCase()}
                  </option>
                ))}
              </select>
            </div>
          </div>
        </div>

        {/* Tags (UC-ACC-03.6) */}
        <div className={styles.section}>
          <h2 className={styles.sectionTitle}>{t('sectionTags')}</h2>
          <Input
            label={t('tags')}
            placeholder={t('tagsPlaceholder')}
            value={tagDraft}
            onChange={(e) => setTagDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') {
                e.preventDefault();
                addTag();
              }
            }}
            fullWidth
          />
          {tags.length > 0 && (
            <div className={styles.tagRow}>
              {tags.map((tag) => (
                <span key={tag} className={styles.tagChip}>
                  {tag}
                  <button
                    type="button"
                    className={styles.tagChipRemove}
                    onClick={() => removeTag(tag)}
                    aria-label={`${tc('delete')} ${tag}`}
                  >
                    ×
                  </button>
                </span>
              ))}
            </div>
          )}
        </div>

        <div className={styles.formActions}>
          <Button
            variant="ghost"
            type="button"
            onClick={() => router.push('/contacts')}
            disabled={busy}
          >
            {tc('cancel')}
          </Button>
          <Button variant="primary" type="submit" disabled={busy}>
            {tc('save')}
          </Button>
        </div>
      </form>
    </div>
  );
}
