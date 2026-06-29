'use client';

import { Alert, Button, Input } from '@ppt/ui-kit';
import { useTranslations } from 'next-intl';
import { type FormEvent, useState } from 'react';
import styles from '@/styles/signup.module.css';

type Errors = Partial<Record<'company' | 'email' | 'password', string>>;

/**
 * Signup form for the public surface.
 *
 * Copy is fully slot-wired (CMO supplies final strings). The submit handler is
 * intentionally a no-op-with-feedback for now: the real account-creation call
 * depends on the accounting-api-client (PAP-303 task #2) and new signup
 * endpoints, which land in the auth/shell work (#3). Until then we validate
 * inputs client-side and show a "coming soon" notice — no data is sent or
 * stored. Swap the TODO block for the API mutation when #2/#3 land.
 */
export function SignupForm() {
  const t = useTranslations('signup');
  const [errors, setErrors] = useState<Errors>({});
  const [submitted, setSubmitted] = useState(false);

  function onSubmit(e: FormEvent<HTMLFormElement>) {
    e.preventDefault();
    const form = e.currentTarget;
    const data = new FormData(form);
    const company = String(data.get('company') ?? '').trim();
    const email = String(data.get('email') ?? '').trim();
    const password = String(data.get('password') ?? '');

    const next: Errors = {};
    if (!company) next.company = t('validation.companyRequired');
    if (!email) next.email = t('validation.emailRequired');
    else if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email)) next.email = t('validation.emailInvalid');
    if (password.length < 8) next.password = t('validation.passwordShort');

    setErrors(next);
    if (Object.keys(next).length === 0) {
      // TODO(PAP-303 #2/#3): replace with accountingApiClient.signup(...) mutation.
      setSubmitted(true);
    }
  }

  return (
    <form className={styles.form} onSubmit={onSubmit} noValidate>
      <div className={styles.field}>
        <Input
          name="company"
          label={t('fields.companyLabel')}
          placeholder={t('fields.companyPlaceholder')}
          error={errors.company}
          fullWidth
          autoComplete="organization"
        />
      </div>

      <div className={styles.field}>
        <Input
          name="name"
          label={t('fields.nameLabel')}
          placeholder={t('fields.namePlaceholder')}
          fullWidth
          autoComplete="name"
        />
      </div>

      <div className={styles.field}>
        <Input
          name="email"
          type="email"
          label={t('fields.emailLabel')}
          placeholder={t('fields.emailPlaceholder')}
          error={errors.email}
          fullWidth
          autoComplete="email"
        />
      </div>

      <div className={styles.field}>
        <Input
          name="password"
          type="password"
          label={t('fields.passwordLabel')}
          placeholder={t('fields.passwordPlaceholder')}
          error={errors.password}
          helperText={errors.password ? undefined : t('fields.passwordPlaceholder')}
          fullWidth
          autoComplete="new-password"
        />
      </div>

      <p className={styles.consent}>{t('consent')}</p>

      <div className={styles.submitWrap}>
        <Button type="submit" variant="primary" size="lg" fullWidth>
          {t('submit')}
        </Button>
      </div>

      {submitted && (
        <div className={styles.notice}>
          <Alert variant="info">{t('comingSoon')}</Alert>
        </div>
      )}
    </form>
  );
}
