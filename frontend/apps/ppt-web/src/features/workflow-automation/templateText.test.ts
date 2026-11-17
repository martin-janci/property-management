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

/**
 * Czech (and Slovak) are CLDR `few`/`many` locales: counts 2–4 take the
 * nominative-plural "few" form, 5+ take the genitive-plural "other" form.
 * These automation/moderation count strings previously shipped only `_one`
 * and `_other` in cs.json, so a count of 2–4 silently fell back to the
 * genitive `_other` form ("2 akcí" instead of the correct "2 akce").
 * Guard the full Czech plural ladder so the gap cannot regress.
 */
describe('Czech automation/moderation count plurals', () => {
  it('uses the correct few form for counts 2–4 and other for 5+', async () => {
    const t = await tFor('cs');
    // ruleCard — akce (feminine), podmínka (feminine)
    expect(t('automation.ruleCard.actions', { count: 1 })).toBe('1 akce');
    expect(t('automation.ruleCard.actions', { count: 2 })).toBe('2 akce');
    expect(t('automation.ruleCard.actions', { count: 5 })).toBe('5 akcí');
    expect(t('automation.ruleCard.conditions', { count: 2 })).toBe('2 podmínky');
    expect(t('automation.ruleCard.conditions', { count: 5 })).toBe('5 podmínek');
    // builder summary counts
    expect(t('automation.builder.actionsCount', { count: 2 })).toBe('2 akce');
    expect(t('automation.builder.conditionsCount', { count: 2 })).toBe('2 podmínky');
    // templateCard / templatePreview
    expect(t('automation.templateCard.actions', { count: 2 })).toBe('2 akce');
    expect(t('automation.templateCard.actions', { count: 5 })).toBe('5 akcí');
    expect(t('automation.templatePreview.usedTimes', { count: 2 })).toBe(
      'Použito 2× jinými uživateli'
    );
    // moderation overdue — případ (masculine inanimate)
    expect(t('moderation.stats.overdue', { count: 2 })).toBe('2 případy po termínu (SLA 24 h+)');
    expect(t('moderation.stats.overdue', { count: 5 })).toBe('5 případů po termínu (SLA 24 h+)');
  });
});
