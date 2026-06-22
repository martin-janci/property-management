/**
 * GenerateDocumentDialog (Epic 7B, Story 7B.2).
 *
 * Modal that drives the template → document generation flow:
 *   1. loads the template (placeholders + content) via `useTemplate`
 *   2. lets the manager pick a building / unit *context* whose fields pre-fill
 *      matching placeholders (e.g. `{{building_name}}`, `{{unit_number}}`)
 *   3. collects the remaining placeholder values (typed inputs)
 *   4. shows a live preview of the rendered content (client mirror of the
 *      server's `generate_content` substitution)
 *   5. POSTs to `/api/v1/templates/{id}/generate` and navigates to the new doc
 *
 * Endpoints are wired through the hand-written `@ppt/api-client` templates
 * module (`useTemplate`, `useGenerateDocument`).
 */

import { type TemplatePlaceholder, useGenerateDocument, useTemplate } from '@ppt/api-client';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { useFocusTrap } from '../../../hooks/useFocusTrap';
import { useBuildings, useUnits } from '../../buildings/hooks';

/** Document categories accepted by the backend (`document_category::ALL`). */
const DOCUMENT_CATEGORIES = [
  'contracts',
  'invoices',
  'reports',
  'manuals',
  'certificates',
  'other',
] as const;

interface GenerateDocumentDialogProps {
  templateId: string;
  templateName: string;
  onClose: () => void;
}

/** Map a placeholder data-type to a sensible HTML input type. */
function inputTypeFor(type: TemplatePlaceholder['type']): string {
  switch (type) {
    case 'date':
      return 'date';
    case 'number':
    case 'currency':
      return 'number';
    default:
      return 'text';
  }
}

/**
 * Decide whether a building/unit context field should pre-fill a placeholder.
 * Matching is by case-insensitive substring on the placeholder name so a
 * template authored with `{{building_name}}`, `{{buildingName}}` or
 * `{{nazov_budovy}}`-style names still benefits where the alias matches.
 */
function contextValueFor(
  placeholderName: string,
  ctx: {
    buildingName?: string;
    buildingAddress?: string;
    unitDesignation?: string;
    unitFloor?: string;
  }
): string | undefined {
  const n = placeholderName.toLowerCase();
  const has = (...keys: string[]) => keys.some((k) => n.includes(k));

  if (
    ctx.unitDesignation &&
    has('unit_number', 'unit_designation', 'unit', 'apartment', 'byt', 'jednotka')
  ) {
    return ctx.unitDesignation;
  }
  if (ctx.unitFloor && has('floor', 'poschodie')) {
    return ctx.unitFloor;
  }
  if (ctx.buildingAddress && has('address', 'adresa')) {
    return ctx.buildingAddress;
  }
  if (ctx.buildingName && has('building', 'budova', 'dom')) {
    return ctx.buildingName;
  }
  return undefined;
}

/** Render template content by substituting `{{name}}` with values / defaults. */
function renderPreview(
  content: string,
  placeholders: TemplatePlaceholder[],
  values: Record<string, string>
): string {
  let out = content;
  for (const p of placeholders) {
    const value = values[p.name] ?? p.default_value ?? '';
    out = out.split(`{{${p.name}}}`).join(value);
  }
  return out;
}

export function GenerateDocumentDialog({
  templateId,
  templateName,
  onClose,
}: GenerateDocumentDialogProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const dialogRef = useRef<HTMLDivElement | null>(null);
  useFocusTrap(dialogRef, onClose);

  const { data, isLoading, isError, refetch } = useTemplate(templateId);
  const generate = useGenerateDocument(templateId);

  const template = data?.template;
  const placeholders = useMemo(() => template?.placeholders ?? [], [template]);

  // Form state
  const [values, setValues] = useState<Record<string, string>>({});
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [category, setCategory] = useState<string>('other');
  const [buildingId, setBuildingId] = useState('');
  const [unitId, setUnitId] = useState('');
  const [submitAttempted, setSubmitAttempted] = useState(false);

  // Context data
  const { data: buildingsData } = useBuildings();
  const buildings = buildingsData?.items ?? [];
  const { data: unitsData } = useUnits(buildingId, undefined, !!buildingId);
  const units = unitsData?.units ?? [];

  // Seed default values once the template loads.
  useEffect(() => {
    if (!template) return;
    setValues((prev) => {
      const next = { ...prev };
      for (const p of placeholders) {
        if (next[p.name] === undefined && p.default_value) {
          next[p.name] = p.default_value;
        }
      }
      return next;
    });
    if (!title) setTitle(template.name);
  }, [template, placeholders, title]);

  // Pre-fill placeholders from the chosen building / unit context.
  useEffect(() => {
    const building = buildings.find((b) => b.id === buildingId);
    const unit = units.find((u) => u.id === unitId);
    if (!building && !unit) return;

    const ctx = {
      buildingName: building?.name,
      buildingAddress: building
        ? [building.address?.street, building.address?.city].filter(Boolean).join(', ')
        : undefined,
      unitDesignation: unit?.designation,
      unitFloor: unit ? String(unit.floor) : undefined,
    };

    setValues((prev) => {
      const next = { ...prev };
      let changed = false;
      for (const p of placeholders) {
        const ctxValue = contextValueFor(p.name, ctx);
        if (ctxValue && !next[p.name]) {
          next[p.name] = ctxValue;
          changed = true;
        }
      }
      return changed ? next : prev;
    });
  }, [buildingId, unitId, buildings, units, placeholders]);

  const missingRequired = useMemo(
    () =>
      placeholders.filter((p) => p.required && !(values[p.name] ?? '').trim()).map((p) => p.name),
    [placeholders, values]
  );

  const canSubmit = title.trim().length > 0 && missingRequired.length === 0;

  const handleGenerate = () => {
    setSubmitAttempted(true);
    if (!canSubmit) return;

    // Only send non-empty values; the server applies placeholder defaults.
    const payloadValues: Record<string, string> = {};
    for (const [k, v] of Object.entries(values)) {
      if (v.trim()) payloadValues[k] = v;
    }

    generate.mutate(
      {
        values: payloadValues,
        title: title.trim(),
        description: description.trim() || undefined,
        category,
      },
      {
        onSuccess: (res) => {
          onClose();
          navigate(`/documents/${res.document_id}`);
        },
      }
    );
  };

  return (
    <div className="gdt-overlay" role="presentation" onClick={onClose}>
      <div
        ref={dialogRef}
        className="gdt-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="gdt-title"
        tabIndex={-1}
        onClick={(e) => e.stopPropagation()}
      >
        <header className="gdt-header">
          <h2 id="gdt-title" className="gdt-title">
            {t('documentTemplates.generate.title', { name: templateName })}
          </h2>
          <button
            type="button"
            className="gdt-close"
            onClick={onClose}
            aria-label={t('common.close')}
          >
            ×
          </button>
        </header>

        {isLoading && <div className="gdt-state">{t('common.loading')}</div>}

        {isError && (
          <div className="gdt-state gdt-state--error">
            <p>{t('documentTemplates.generate.loadError')}</p>
            <button type="button" className="gdt-btn" onClick={() => refetch()}>
              {t('common.retry')}
            </button>
          </div>
        )}

        {template && (
          <div className="gdt-body">
            {/* Context selector */}
            <fieldset className="gdt-section">
              <legend className="gdt-legend">
                {t('documentTemplates.generate.contextLegend')}
              </legend>
              <div className="gdt-grid">
                <label className="gdt-field">
                  <span className="gdt-label">{t('documentTemplates.generate.building')}</span>
                  <select
                    className="gdt-input"
                    value={buildingId}
                    onChange={(e) => {
                      setBuildingId(e.target.value);
                      setUnitId('');
                    }}
                  >
                    <option value="">{t('documentTemplates.generate.noContext')}</option>
                    {buildings.map((b) => (
                      <option key={b.id} value={b.id}>
                        {b.name}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="gdt-field">
                  <span className="gdt-label">{t('documentTemplates.generate.unit')}</span>
                  <select
                    className="gdt-input"
                    value={unitId}
                    onChange={(e) => setUnitId(e.target.value)}
                    disabled={!buildingId}
                  >
                    <option value="">{t('documentTemplates.generate.noUnit')}</option>
                    {units.map((u) => (
                      <option key={u.id} value={u.id}>
                        {u.designation}
                      </option>
                    ))}
                  </select>
                </label>
              </div>
              <p className="gdt-hint">{t('documentTemplates.generate.contextHint')}</p>
            </fieldset>

            {/* Placeholder inputs */}
            <fieldset className="gdt-section">
              <legend className="gdt-legend">
                {t('documentTemplates.generate.placeholdersLegend')}
              </legend>
              {placeholders.length === 0 ? (
                <p className="gdt-hint">{t('documentTemplates.generate.noPlaceholders')}</p>
              ) : (
                <div className="gdt-grid">
                  {placeholders.map((p) => {
                    const isMissing =
                      submitAttempted && p.required && !(values[p.name] ?? '').trim();
                    return (
                      <label className="gdt-field" key={p.name}>
                        <span className="gdt-label">
                          {p.name}
                          {p.required && (
                            <span className="gdt-req" aria-hidden="true">
                              {' '}
                              *
                            </span>
                          )}
                        </span>
                        <input
                          className={`gdt-input${isMissing ? ' gdt-input--error' : ''}`}
                          type={inputTypeFor(p.type)}
                          step={p.type === 'currency' ? '0.01' : undefined}
                          value={values[p.name] ?? ''}
                          placeholder={p.description ?? undefined}
                          aria-invalid={isMissing || undefined}
                          onChange={(e) =>
                            setValues((prev) => ({ ...prev, [p.name]: e.target.value }))
                          }
                        />
                        {p.description && <span className="gdt-desc">{p.description}</span>}
                      </label>
                    );
                  })}
                </div>
              )}
            </fieldset>

            {/* Document metadata */}
            <fieldset className="gdt-section">
              <legend className="gdt-legend">
                {t('documentTemplates.generate.documentLegend')}
              </legend>
              <div className="gdt-grid">
                <label className="gdt-field">
                  <span className="gdt-label">
                    {t('documentTemplates.generate.docTitle')}
                    <span className="gdt-req" aria-hidden="true">
                      {' '}
                      *
                    </span>
                  </span>
                  <input
                    className={`gdt-input${submitAttempted && !title.trim() ? ' gdt-input--error' : ''}`}
                    type="text"
                    value={title}
                    onChange={(e) => setTitle(e.target.value)}
                  />
                </label>
                <label className="gdt-field">
                  <span className="gdt-label">{t('documentTemplates.generate.category')}</span>
                  <select
                    className="gdt-input"
                    value={category}
                    onChange={(e) => setCategory(e.target.value)}
                  >
                    {DOCUMENT_CATEGORIES.map((c) => (
                      <option key={c} value={c}>
                        {t(`documentTemplates.categories.${c}`)}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="gdt-field gdt-field--full">
                  <span className="gdt-label">{t('documentTemplates.generate.description')}</span>
                  <input
                    className="gdt-input"
                    type="text"
                    value={description}
                    onChange={(e) => setDescription(e.target.value)}
                  />
                </label>
              </div>
            </fieldset>

            {/* Live preview */}
            <fieldset className="gdt-section">
              <legend className="gdt-legend">
                {t('documentTemplates.generate.previewLegend')}
              </legend>
              <pre className="gdt-preview" aria-live="polite">
                {renderPreview(template.content, placeholders, values)}
              </pre>
            </fieldset>

            {submitAttempted && missingRequired.length > 0 && (
              <p className="gdt-error" role="alert">
                {t('documentTemplates.generate.missingPlaceholders', {
                  names: missingRequired.join(', '),
                })}
              </p>
            )}
            {generate.isError && (
              <p className="gdt-error" role="alert">
                {t('documentTemplates.generate.generateError', {
                  message: (generate.error as Error)?.message ?? '',
                })}
              </p>
            )}
          </div>
        )}

        <footer className="gdt-footer">
          <button type="button" className="gdt-btn" onClick={onClose}>
            {t('common.cancel')}
          </button>
          <button
            type="button"
            className="gdt-btn gdt-btn--primary"
            onClick={handleGenerate}
            disabled={!template || generate.isPending}
          >
            {generate.isPending
              ? t('documentTemplates.generate.generating')
              : t('documentTemplates.generate.submit')}
          </button>
        </footer>
      </div>
    </div>
  );
}
