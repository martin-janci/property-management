/**
 * RecipientManager - Component for managing report recipients.
 *
 * Story 81.1 - Report Schedule Editing
 * Supports searching for users and adding external email addresses.
 */

import { useCallback, useState } from 'react';
import { useTranslation } from 'react-i18next';

interface RecipientManagerProps {
  recipients: string[];
  onChange: (recipients: string[]) => void;
  error?: string;
  placeholder?: string;
}

// Email validation regex - RFC 5322 compliant pattern supporting:
// - International characters in local part (Unicode)
// - Newer TLDs (4+ characters)
// - International domain names
// Note: For maximum compatibility, consider using a dedicated validation library
// like validator.js or email-validator in production.
const EMAIL_REGEX =
  /^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$/;

export function RecipientManager({
  recipients,
  onChange,
  error,
  placeholder = 'Enter email address',
}: RecipientManagerProps) {
  const { t } = useTranslation();
  const [inputValue, setInputValue] = useState('');
  const [inputError, setInputError] = useState<string | null>(null);

  const validateEmail = useCallback((email: string): boolean => {
    return EMAIL_REGEX.test(email.trim());
  }, []);

  const addRecipient = useCallback(
    (email: string) => {
      const trimmedEmail = email.trim().toLowerCase();

      if (!trimmedEmail) {
        return;
      }

      if (!validateEmail(trimmedEmail)) {
        setInputError(t('reports.schedule.editErrors.invalidEmail'));
        return;
      }

      if (recipients.includes(trimmedEmail)) {
        setInputError(t('reports.schedule.editErrors.duplicateEmail'));
        return;
      }

      onChange([...recipients, trimmedEmail]);
      setInputValue('');
      setInputError(null);
    },
    [recipients, onChange, validateEmail, t]
  );

  const removeRecipient = useCallback(
    (email: string) => {
      onChange(recipients.filter((r) => r !== email));
    },
    [recipients, onChange]
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        addRecipient(inputValue);
      } else if (e.key === 'Backspace' && inputValue === '' && recipients.length > 0) {
        // Remove last recipient when pressing backspace on empty input
        removeRecipient(recipients[recipients.length - 1]);
      }
    },
    [inputValue, addRecipient, recipients, removeRecipient]
  );

  const handleInputChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    setInputValue(e.target.value);
    setInputError(null);
  }, []);

  const handleAddClick = useCallback(() => {
    addRecipient(inputValue);
  }, [inputValue, addRecipient]);

  const handlePaste = useCallback(
    (e: React.ClipboardEvent<HTMLInputElement>) => {
      e.preventDefault();
      const pastedText = e.clipboardData.getData('text');
      // Split by common separators (comma, semicolon, newline, space)
      const emails = pastedText.split(/[,;\s\n]+/).filter(Boolean);

      const validEmails: string[] = [];
      const invalidEmails: string[] = [];

      for (const email of emails) {
        const trimmed = email.trim().toLowerCase();
        if (trimmed) {
          if (validateEmail(trimmed) && !recipients.includes(trimmed)) {
            validEmails.push(trimmed);
          } else if (!validateEmail(trimmed)) {
            invalidEmails.push(trimmed);
          }
        }
      }

      if (validEmails.length > 0) {
        onChange([...recipients, ...validEmails]);
      }

      if (invalidEmails.length > 0) {
        setInputError(
          t('reports.schedule.editErrors.invalidEmailsList', {
            emails: `${invalidEmails.slice(0, 3).join(', ')}${invalidEmails.length > 3 ? '...' : ''}`,
          })
        );
      }

      setInputValue('');
    },
    [recipients, onChange, validateEmail, t]
  );

  return (
    <div className="space-y-2">
      {/* Recipient chips */}
      {recipients.length > 0 && (
        <div className="flex flex-wrap gap-2">
          {recipients.map((email) => (
            <span
              key={email}
              className="inline-flex items-center gap-1 px-3 py-1 bg-blue-100 text-blue-800 text-sm rounded-full"
            >
              <svg
                className="w-4 h-4 text-blue-600"
                fill="none"
                stroke="currentColor"
                viewBox="0 0 24 24"
                aria-hidden="true"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M3 8l7.89 5.26a2 2 0 002.22 0L21 8M5 19h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"
                />
              </svg>
              <span className="max-w-[200px] truncate">{email}</span>
              <button
                type="button"
                onClick={() => removeRecipient(email)}
                className="ml-1 text-blue-600 hover:text-blue-800 focus:outline-none"
                aria-label={`Remove ${email}`}
              >
                <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                  <path
                    fillRule="evenodd"
                    d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
                    clipRule="evenodd"
                  />
                </svg>
              </button>
            </span>
          ))}
        </div>
      )}

      {/* Input field */}
      <div className="flex gap-2">
        <div className="flex-1">
          <input
            type="email"
            value={inputValue}
            onChange={handleInputChange}
            onKeyDown={handleKeyDown}
            onPaste={handlePaste}
            placeholder={placeholder}
            className={`block w-full px-3 py-2 border rounded-md shadow-sm focus:outline-none focus:ring-blue-500 focus:border-blue-500 ${
              inputError || error ? 'border-red-300' : 'border-gray-300'
            }`}
            aria-invalid={!!(inputError || error)}
            aria-describedby={inputError || error ? 'recipient-error' : undefined}
          />
        </div>
        <button
          type="button"
          onClick={handleAddClick}
          disabled={!inputValue.trim()}
          className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-md hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          Add
        </button>
      </div>

      {/* Error message */}
      {(inputError || error) && (
        <p id="recipient-error" className="text-sm text-red-600">
          {inputError || error}
        </p>
      )}

      {/* Help text */}
      <p className="text-xs text-gray-500">
        Press Enter to add. You can paste multiple emails separated by commas.
      </p>
    </div>
  );
}
