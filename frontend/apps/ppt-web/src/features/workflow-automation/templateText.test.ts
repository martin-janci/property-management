/// <reference types="vitest/globals" />
/**
 * Gap audit #32: built-in automation templates showed their English server
 * name ("Weekly Maintenance Report") in every locale. The server now sends a
 * stable `slug`; ppt-web translates by it and falls back to the server text
 * for custom templates.
 */

import i18next from 'i18next';
import cs from '../../../messages/cs.json';
import de from '../../../messages/de.json';
import en from '../../../messages/en.json';
import hu from '../../../messages/hu.json';
import pl from '../../../messages/pl.json';
import sk from '../../../messages/sk.json';
import { templateActionLabel, templateDescription, templateName } from './templateText';

const BUNDLES = { en, sk, cs, de, pl, hu } as const;
const SLUGS = [
  'weekly_maintenance_report',
  'overdue_payment_reminder',
  'high_priority_fault_escalation',
  'monthly_statement_generation',
];
const ACTION_TYPES = ['generate_report', 'send_email', 'send_notification', 'assign_fault'];

async function tFor(lng: keyof typeof BUNDLES) {
  const instance = i18next.createInstance();
  await instance.init({
    lng,
    fallbackLng: 'en',
    resources: { [lng]: { translation: BUNDLES[lng] }, en: { translation: en } },
    interpolation: { escapeValue: false },
  });
  return instance.t;
}

describe('automation template copy', () => {
  it.each(Object.keys(BUNDLES))(
    'has a name and description for every built-in template in %s',
    (lng) => {
      const templates = (
        BUNDLES[lng as keyof typeof BUNDLES] as {
          automation: { templates: Record<string, Record<string, string>> };
        }
      ).automation.templates;
      for (const slug of SLUGS) {
        expect(templates[slug]?.name, `${lng}:${slug}.name`).toBeTruthy();
        expect(templates[slug]?.description, `${lng}:${slug}.description`).toBeTruthy();
      }
      for (const action of ACTION_TYPES) {
        expect(templates.actionTypes?.[action], `${lng}:actionTypes.${action}`).toBeTruthy();
      }
    }
  );

  it('renders the Slovak name and description for a built-in template', async () => {
    const t = await tFor('sk');
    const template = {
      slug: 'weekly_maintenance_report',
      name: 'Weekly Maintenance Report',
      description: 'Send weekly summary of open maintenance requests to managers',
    };
    expect(templateName(t, template)).toBe('Týždenný prehľad údržby');
    expect(templateDescription(t, template)).not.toBe(template.description);
    expect(templateActionLabel(t, 'send_email')).toBe('Odoslať e-mail');
    expect(t('automation.templateCard.actions', { count: 2 })).toBe('2 akcie');
  });

  it('falls back to the server text for custom templates and unknown actions', async () => {
    const t = await tFor('sk');
    const custom = { name: 'Moja šablóna', description: 'Vlastná' };
    expect(templateName(t, custom)).toBe('Moja šablóna');
    expect(templateDescription(t, custom)).toBe('Vlastná');
    expect(templateName(t, { ...custom, slug: 'not_a_template' })).toBe('Moja šablóna');
    expect(templateActionLabel(t, 'custom_action')).toBe('custom_action');
  });
});
