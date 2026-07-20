import type { BaseSection, LayoutManifest, LayoutRails, TenantOverride } from '@ppt/api-client';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import './TenantSectionEditor.css';

export interface TenantSectionEditorProps {
  baseSections: BaseSection[];
  rails: LayoutRails;
  manifest: LayoutManifest | null;
  override: TenantOverride;
  onChange(next: TenantOverride): void;
}

/** Build the display order: override.order-listed first (in order), then unlisted in base order. */
function resolveOrder(baseSections: BaseSection[], override: TenantOverride): BaseSection[] {
  const orderList = override.order ?? [];
  const listed = orderList
    .map((type) => baseSections.find((s) => s.type === type))
    .filter((s): s is BaseSection => s !== undefined);
  const listedTypes = new Set(orderList);
  const unlisted = baseSections.filter((s) => !listedTypes.has(s.type));
  return [...listed, ...unlisted];
}

/** Prune sections entry if it has no keys; prune override.sections if empty. */
function pruneSectionPatch(
  override: TenantOverride,
  type: string,
  patch: Record<string, unknown>
): TenantOverride {
  const cleanPatch = Object.fromEntries(Object.entries(patch).filter(([, v]) => v !== undefined));
  const hasPatch = Object.keys(cleanPatch).length > 0;

  const sections = { ...(override.sections ?? {}) };
  if (hasPatch) {
    sections[type] = cleanPatch;
  } else {
    delete sections[type];
  }

  const next: TenantOverride = { ...override };
  if (Object.keys(sections).length > 0) {
    next.sections = sections;
  } else {
    delete next.sections;
  }
  return next;
}

/**
 * Coerce a blur-committed input string to the value stored in the override.
 *
 * A string-typed prop must stay a string: `"true"`, `"[]"` and `"123"` are
 * legitimate string content, not a signal to coerce them into a boolean /
 * array / number. We therefore only run JSON coercion when the prop's
 * *declared* type (the base section's value for this prop) is non-string —
 * or unknown/untyped, which preserves the ability to enter numbers, booleans
 * and arrays for whitelisted props that have no string default. JSON that
 * fails to parse always falls back to the raw string, so user intent for the
 * whitelisted prop's value is never lost.
 */
function coercePropValue(raw: string, declaredValue: unknown): unknown {
  if (typeof declaredValue === 'string') {
    return raw;
  }
  try {
    return JSON.parse(raw);
  } catch {
    return raw;
  }
}

interface PropInputProps {
  type: string;
  propName: string;
  currentValue: unknown;
  declaredValue: unknown;
  override: TenantOverride;
  onChange(next: TenantOverride): void;
}

function PropInput({
  type,
  propName,
  currentValue,
  declaredValue,
  override,
  onChange,
}: PropInputProps) {
  const display =
    currentValue === undefined || currentValue === null
      ? ''
      : typeof currentValue === 'string'
        ? currentValue
        : JSON.stringify(currentValue);

  const [localValue, setLocalValue] = useState(display);

  const commit = () => {
    const val = localValue.trim();
    const existingPatch = { ...(override.sections?.[type] ?? {}) };
    const existingProps = { ...(existingPatch.props ?? {}) };

    if (val === '') {
      delete existingProps[propName];
    } else {
      existingProps[propName] = coercePropValue(val, declaredValue);
    }

    const newPatch: Record<string, unknown> = { ...existingPatch };
    if (Object.keys(existingProps).length > 0) {
      newPatch.props = existingProps;
    } else {
      delete newPatch.props;
    }

    onChange(pruneSectionPatch(override, type, newPatch));
  };

  return (
    <input
      className="tenant-editor__input"
      aria-label={propName}
      value={localValue}
      onChange={(e) => setLocalValue(e.target.value)}
      onBlur={commit}
    />
  );
}

export function TenantSectionEditor({
  baseSections,
  rails,
  manifest,
  override,
  onChange,
}: TenantSectionEditorProps) {
  const { t } = useTranslation();

  const orderedSections = resolveOrder(baseSections, override);
  const currentOrder = orderedSections.map((s) => s.type);

  return (
    <div className="tenant-editor">
      {orderedSections.map((base, idx) => {
        const { type } = base;
        const sectionPatch = override.sections?.[type] ?? {};
        const effectiveVisible = sectionPatch.visible ?? base.visible ?? true;
        const effectiveMode = sectionPatch.mode ?? base.mode;

        // Guard the whole manifest path — a malformed envelope may lack
        // `components` entirely, and indexing into undefined would throw.
        const manifestComponent = manifest?.components?.[type];

        const canHide = rails.hideable.includes(type);
        const canReorder = rails.reorderable;
        const canEditMode =
          rails.mode_editable.includes(type) &&
          (manifestComponent?.supported_modes?.length ?? 0) > 0;
        const propWhitelist = rails.prop_whitelist[type] ?? [];
        const hasControls = canHide || canReorder || canEditMode || propWhitelist.length > 0;

        const supportedModes = manifestComponent?.supported_modes ?? [];

        const toggleVisible = () => {
          const existingPatch = { ...(override.sections?.[type] ?? {}) };
          const next: Record<string, unknown> = { ...existingPatch, visible: !effectiveVisible };
          onChange(pruneSectionPatch(override, type, next));
        };

        const moveUp = () => {
          if (idx === 0) return;
          const newOrder = [...currentOrder];
          [newOrder[idx - 1], newOrder[idx]] = [newOrder[idx], newOrder[idx - 1]];
          onChange({ ...override, order: newOrder });
        };

        const moveDown = () => {
          if (idx === currentOrder.length - 1) return;
          const newOrder = [...currentOrder];
          [newOrder[idx], newOrder[idx + 1]] = [newOrder[idx + 1], newOrder[idx]];
          onChange({ ...override, order: newOrder });
        };

        const changeMode = (mode: string) => {
          const existingPatch = { ...(override.sections?.[type] ?? {}) };
          const next: Record<string, unknown> = { ...existingPatch };
          if (mode === '') {
            // Empty option — remove the mode from the patch (back to default)
            delete next.mode;
          } else {
            next.mode = mode;
          }
          onChange(pruneSectionPatch(override, type, next));
        };

        return (
          <div
            key={type}
            className={`tenant-editor__row${!effectiveVisible ? ' tenant-editor__row--hidden' : ''}`}
          >
            <span className="tenant-editor__type">{type}</span>

            {/* State badges */}
            {!effectiveVisible && (
              <span className="tenant-editor__badge tenant-editor__badge--hidden">
                {t('layout.customize.hidden')}
              </span>
            )}
            {effectiveMode && <span className="tenant-editor__badge">{effectiveMode}</span>}

            {hasControls && (
              <div className="tenant-editor__controls">
                {/* Eye toggle */}
                {canHide && (
                  <button
                    type="button"
                    className="tenant-editor__btn"
                    aria-label={
                      effectiveVisible ? t('layout.customize.hide') : t('layout.customize.show')
                    }
                    onClick={toggleVisible}
                  >
                    {effectiveVisible ? '👁' : '👁\u{FE0F}\u{20E3}'}
                  </button>
                )}

                {/* Reorder arrows */}
                {canReorder && (
                  <>
                    <button
                      type="button"
                      className="tenant-editor__btn"
                      aria-label={t('layout.customize.moveUp')}
                      onClick={moveUp}
                      disabled={idx === 0}
                    >
                      ↑
                    </button>
                    <button
                      type="button"
                      className="tenant-editor__btn"
                      aria-label={t('layout.customize.moveDown')}
                      onClick={moveDown}
                      disabled={idx === currentOrder.length - 1}
                    >
                      ↓
                    </button>
                  </>
                )}

                {/* Mode select */}
                {canEditMode && (
                  <select
                    className="tenant-editor__select"
                    aria-label={t('layout.customize.mode')}
                    value={sectionPatch.mode ?? ''}
                    onChange={(e) => changeMode(e.target.value)}
                  >
                    <option value="">{t('layout.customize.modeDefault')}</option>
                    {supportedModes.map((m) => (
                      <option key={m} value={m}>
                        {m}
                      </option>
                    ))}
                  </select>
                )}

                {/* Whitelisted prop inputs.
                    Key includes the serialized current external value so that
                    an external reset (override → {}) causes remount and
                    PropInput reflects the new (empty) value rather than
                    keeping stale internal state from useState. */}
                {propWhitelist.map((propName) => {
                  const currentValue = (override.sections?.[type]?.props ?? base.props ?? {})[
                    propName
                  ];
                  // Declared type signal: the base section's value for this
                  // prop. Drives whether a blur-committed string is coerced.
                  const declaredValue = base.props?.[propName];
                  return (
                    <PropInput
                      key={`${type}:${propName}:${JSON.stringify(currentValue) ?? ''}`}
                      type={type}
                      propName={propName}
                      currentValue={currentValue}
                      declaredValue={declaredValue}
                      override={override}
                      onChange={onChange}
                    />
                  );
                })}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
}
