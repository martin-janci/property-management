/**
 * Modal displaying available keyboard shortcuts for the action queue.
 * Triggered by pressing '?' on the dashboard.
 *
 * @module features/dashboard/components/KeyboardShortcutsHelp
 */

import { useCallback, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { useFocusTrap } from '../../../hooks/useFocusTrap';
import './KeyboardShortcutsHelp.css';

interface KeyboardShortcutsHelpProps {
  isOpen: boolean;
  onClose: () => void;
}

interface ShortcutItem {
  keys: string[];
  descriptionKey: string;
}

const shortcuts: ShortcutItem[] = [
  { keys: ['j', '↓'], descriptionKey: 'dashboard.shortcuts.nextItem' },
  { keys: ['k', '↑'], descriptionKey: 'dashboard.shortcuts.prevItem' },
  { keys: ['Enter'], descriptionKey: 'dashboard.shortcuts.openItem' },
  { keys: ['a'], descriptionKey: 'dashboard.shortcuts.approve' },
  { keys: ['r'], descriptionKey: 'dashboard.shortcuts.reject' },
  { keys: ['Esc'], descriptionKey: 'dashboard.shortcuts.closeFilters' },
  { keys: ['?'], descriptionKey: 'dashboard.shortcuts.showHelp' },
];

export function KeyboardShortcutsHelp({ isOpen, onClose }: KeyboardShortcutsHelpProps) {
  const { t } = useTranslation();

  if (!isOpen) return null;

  return <KeyboardShortcutsHelpDialog t={t} onClose={onClose} />;
}

interface KeyboardShortcutsHelpDialogProps {
  t: ReturnType<typeof useTranslation>['t'];
  onClose: () => void;
}

// Inner component so useFocusTrap only mounts when the dialog is actually open,
// guaranteeing focus capture + restore on each open/close cycle.
function KeyboardShortcutsHelpDialog({ t, onClose }: KeyboardShortcutsHelpDialogProps) {
  const dialogRef = useRef<HTMLDivElement>(null);
  useFocusTrap(dialogRef, onClose);

  // Handle backdrop click
  const handleBackdropClick = useCallback(
    (e: React.MouseEvent) => {
      if (e.target === e.currentTarget) {
        onClose();
      }
    },
    [onClose]
  );

  return (
    <div className="kbd-help-overlay" onClick={handleBackdropClick} role="presentation">
      <div
        ref={dialogRef}
        className="kbd-help-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="shortcuts-title"
        tabIndex={-1}
      >
        <div className="kbd-help-dialog__header">
          <h2 id="shortcuts-title" className="kbd-help-dialog__title">
            {t('dashboard.shortcuts.title')}
          </h2>
          <button
            type="button"
            onClick={onClose}
            className="kbd-help-dialog__close"
            aria-label={t('common.close')}
          >
            <span aria-hidden="true">✕</span>
          </button>
        </div>

        <div className="kbd-help-dialog__list">
          {shortcuts.map((shortcut, index) => (
            <div key={index} className="kbd-help-dialog__row">
              <span className="kbd-help-dialog__desc">{t(shortcut.descriptionKey)}</span>
              <div className="kbd-help-dialog__keys">
                {shortcut.keys.map((key, keyIndex) => (
                  <span key={keyIndex} className="kbd-help-dialog__keys">
                    {keyIndex > 0 && <span className="kbd-help-dialog__sep">/</span>}
                    <kbd className="kbd-help-dialog__key">{key}</kbd>
                  </span>
                ))}
              </div>
            </div>
          ))}
        </div>

        <div className="kbd-help-dialog__footer">
          <p className="kbd-help-dialog__footer-note">{t('dashboard.shortcuts.hint')}</p>
        </div>
      </div>
    </div>
  );
}
