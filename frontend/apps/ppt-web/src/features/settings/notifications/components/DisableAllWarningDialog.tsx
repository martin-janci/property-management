/**
 * DisableAllWarningDialog Component (Epic 8A, Story 8A.1)
 *
 * A confirmation dialog shown when the user is about to disable all notification channels.
 * Uses native <dialog> element for better accessibility.
 */

import { useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';

interface DisableAllWarningDialogProps {
  isOpen: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export function DisableAllWarningDialog({
  isOpen,
  onConfirm,
  onCancel,
}: DisableAllWarningDialogProps) {
  const { t } = useTranslation();
  const dialogRef = useRef<HTMLDialogElement>(null);

  // Show/hide dialog using native dialog API
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;

    if (isOpen) {
      dialog.showModal();
    } else {
      dialog.close();
    }
  }, [isOpen]);

  // Handle cancel event from native dialog (Escape key, backdrop click)
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;

    const handleCancel = (e: Event) => {
      e.preventDefault();
      onCancel();
    };

    dialog.addEventListener('cancel', handleCancel);
    return () => dialog.removeEventListener('cancel', handleCancel);
  }, [onCancel]);

  // Handle backdrop click
  const handleBackdropClick = (e: React.MouseEvent<HTMLDialogElement>) => {
    if (e.target === dialogRef.current) {
      onCancel();
    }
  };

  return (
    <dialog
      ref={dialogRef}
      className="backdrop:bg-black backdrop:bg-opacity-25 bg-transparent p-0 max-w-lg w-full"
      aria-labelledby="dialog-title"
      onClick={handleBackdropClick}
      onKeyDown={(e) => {
        // Escape is handled natively by dialog, but we need this for the linter
        if (e.key === 'Escape' || e.key === 'Esc') {
          onCancel();
        }
      }}
    >
      <div className="relative transform overflow-hidden rounded-lg bg-white px-4 pb-4 pt-5 text-left shadow-xl sm:p-6">
        {/* Warning Icon */}
        <div className="sm:flex sm:items-start">
          <div className="mx-auto flex h-12 w-12 flex-shrink-0 items-center justify-center rounded-full bg-amber-100 sm:mx-0 sm:h-10 sm:w-10">
            <svg
              className="h-6 w-6 text-amber-600"
              fill="none"
              viewBox="0 0 24 24"
              strokeWidth="1.5"
              stroke="currentColor"
              aria-hidden="true"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                d="M12 9v3.75m-9.303 3.376c-.866 1.5.217 3.374 1.948 3.374h14.71c1.73 0 2.813-1.874 1.948-3.374L13.949 3.378c-.866-1.5-3.032-1.5-3.898 0L2.697 16.126zM12 15.75h.007v.008H12v-.008z"
              />
            </svg>
          </div>
          <div className="mt-3 text-center sm:ml-4 sm:mt-0 sm:text-left">
            <h3 id="dialog-title" className="text-base font-semibold leading-6 text-gray-900">
              {t('settings.notifications.disableAll.title')}
            </h3>
            <div className="mt-2">
              <p className="text-sm text-gray-500">
                {t('settings.notifications.disableAll.intro')}
              </p>
              <ul className="mt-2 list-disc list-inside text-sm text-gray-500 space-y-1">
                <li>{t('settings.notifications.disableAll.itemAnnouncements')}</li>
                <li>{t('settings.notifications.disableAll.itemFaults')}</li>
                <li>{t('settings.notifications.disableAll.itemVoting')}</li>
                <li>{t('settings.notifications.disableAll.itemMessages')}</li>
              </ul>
              <p className="mt-3 text-sm text-amber-600 font-medium">
                {t('settings.notifications.disableAll.warning')}
              </p>
            </div>
          </div>
        </div>

        {/* Actions */}
        <div className="mt-5 sm:mt-4 sm:flex sm:flex-row-reverse gap-3">
          <button
            type="button"
            onClick={onConfirm}
            className="inline-flex w-full justify-center rounded-md bg-amber-600 px-3 py-2 text-sm font-semibold text-white shadow-sm hover:bg-amber-500 sm:w-auto"
          >
            {t('settings.notifications.disableAll.confirm')}
          </button>
          <button
            type="button"
            onClick={onCancel}
            className="mt-3 inline-flex w-full justify-center rounded-md bg-white px-3 py-2 text-sm font-semibold text-gray-900 shadow-sm ring-1 ring-inset ring-gray-300 hover:bg-gray-50 sm:mt-0 sm:w-auto"
          >
            {t('settings.notifications.disableAll.keep')}
          </button>
        </div>
      </div>
    </dialog>
  );
}
