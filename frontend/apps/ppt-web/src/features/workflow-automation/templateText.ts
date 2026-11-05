/**
 * Localised copy for automation templates.
 *
 * Built-in templates come from the server with English `name`/`description`
 * plus a stable `slug`; their translated copy lives under
 * `automation.templates.<slug>.*`. Custom templates (no slug) and unknown
 * slugs fall back to the server text.
 */

import type { AutomationTemplate } from '@ppt/api-client';
import type { TFunction } from 'i18next';

type TemplateText = Pick<AutomationTemplate, 'slug' | 'name' | 'description'>;

export function templateName(t: TFunction, template: TemplateText): string {
  if (!template.slug) return template.name;
  return t(`automation.templates.${template.slug}.name`, { defaultValue: template.name });
}

export function templateDescription(t: TFunction, template: TemplateText): string {
  if (!template.slug) return template.description;
  return t(`automation.templates.${template.slug}.description`, {
    defaultValue: template.description,
  });
}

/** Label for an action type shown in template previews (e.g. `send_email`). */
export function templateActionLabel(t: TFunction, actionType: string): string {
  return t(`automation.templates.actionTypes.${actionType}`, { defaultValue: actionType });
}
