/**
 * HelpTooltip — inline "?" icon that shows a small tooltip on hover/focus.
 * Can be placed next to any label or heading to provide brief context.
 *
 * Usage:
 *   <HelpTooltip text="Maintenance mode suspends all non-admin API requests." />
 */

import { useId, useState } from 'react';
import { useTranslation } from 'react-i18next';

// ---------------------------------------------------------------------------
// Styles (injected once)
// ---------------------------------------------------------------------------

const STYLE_ID = 'ppt-help-tooltip-styles';

function ensureStyles() {
  if (typeof document === 'undefined') return;
  if (document.getElementById(STYLE_ID)) return;
  const el = document.createElement('style');
  el.id = STYLE_ID;
  el.textContent = `
    .ppt-help-tip {
      position: relative;
      display: inline-flex;
      align-items: center;
    }

    .ppt-help-tip__btn {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      width: 16px;
      height: 16px;
      border-radius: 50%;
      border: 1px solid var(--ppt-border-default, #e5e7eb);
      background: var(--ppt-bg-subtle, #f3f4f6);
      color: var(--ppt-fg-muted, #6b7280);
      font-size: 10px;
      font-weight: 700;
      cursor: default;
      line-height: 1;
      padding: 0;
      flex-shrink: 0;
    }
    .ppt-help-tip__btn:hover,
    .ppt-help-tip__btn:focus {
      background: var(--ppt-brand-100, #dbeafe);
      border-color: var(--ppt-brand-300, #93c5fd);
      color: var(--ppt-brand-700, #1d4ed8);
      outline: none;
    }

    .ppt-help-tip__bubble {
      position: absolute;
      bottom: calc(100% + 6px);
      left: 50%;
      transform: translateX(-50%);
      background: var(--ppt-fg-primary, #111827);
      color: #fff;
      font-size: 12px;
      line-height: 1.5;
      padding: 7px 10px;
      border-radius: var(--ppt-radius-md, 6px);
      width: 220px;
      white-space: normal;
      pointer-events: none;
      z-index: 300;
      box-shadow: 0 4px 12px rgba(0,0,0,0.2);
    }

    /* Tail */
    .ppt-help-tip__bubble::after {
      content: '';
      position: absolute;
      top: 100%;
      left: 50%;
      transform: translateX(-50%);
      border: 5px solid transparent;
      border-top-color: var(--ppt-fg-primary, #111827);
    }
  `;
  document.head.appendChild(el);
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

interface HelpTooltipProps {
  /** Short text shown in the tooltip bubble. */
  text: string;
}

export function HelpTooltip({ text }: HelpTooltipProps) {
  ensureStyles();
  const { t } = useTranslation();
  const [visible, setVisible] = useState(false);
  const tooltipId = useId();

  return (
    <span className="ppt-help-tip">
      <button
        type="button"
        className="ppt-help-tip__btn"
        aria-label={t('admin.help.tooltip', 'Help')}
        aria-describedby={tooltipId}
        onMouseEnter={() => setVisible(true)}
        onMouseLeave={() => setVisible(false)}
        onFocus={() => setVisible(true)}
        onBlur={() => setVisible(false)}
      >
        ?
      </button>
      <span
        id={tooltipId}
        className="ppt-help-tip__bubble"
        role="tooltip"
        style={{ visibility: visible ? 'visible' : 'hidden' }}
      >
        {text}
      </span>
    </span>
  );
}
