/**
 * Accessibility Settings Page for Epic 62.
 *
 * Allows users to configure accessibility preferences.
 */

import type { ColorScheme, TextSize } from '@ppt/shared';
import { COLOR_SCHEME_OPTIONS, TEXT_SIZE_OPTIONS } from '@ppt/shared';
import { useAccessibilityContext } from '@ppt/ui-kit';
import type React from 'react';
import '../styles/accessibility.css';
import { useTranslation } from 'react-i18next';

export const AccessibilitySettingsPage: React.FC = () => {
  const { t } = useTranslation();
  const { settings, updateSettings } = useAccessibilityContext();

  const handleColorSchemeChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    updateSettings({ colorScheme: e.target.value as ColorScheme });
  };

  const handleTextSizeChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    updateSettings({ textSize: e.target.value as TextSize });
  };

  const handleReduceMotionChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    updateSettings({ reduceMotion: e.target.checked });
  };

  const handleScreenReaderChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    updateSettings({ screenReaderEnabled: e.target.checked });
  };

  const handleKeyboardNavChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    updateSettings({ keyboardNavigationEnabled: e.target.checked });
  };

  return (
    <div className="accessibility-settings-container">
      <h1>Accessibility Settings</h1>
      <p>Customize the application to meet your accessibility needs.</p>

      {/* Color Scheme Section */}
      <section className="accessibility-settings-section">
        <h2>Visual Preferences</h2>

        <div className="accessibility-settings-field">
          <label htmlFor="color-scheme" className="accessibility-settings-label">
            Color Scheme
          </label>
          <span className="accessibility-settings-description">
            Choose a color scheme that works best for you. High contrast mode improves text
            readability.
          </span>
          <select
            id="color-scheme"
            className="accessibility-settings-select"
            value={settings.colorScheme}
            onChange={handleColorSchemeChange}
            aria-describedby="color-scheme-description"
          >
            {COLOR_SCHEME_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </div>

        <div className="accessibility-settings-field">
          <label htmlFor="text-size" className="accessibility-settings-label">
            Text Size
          </label>
          <span className="accessibility-settings-description">
            Adjust the text size for better readability across the application.
          </span>
          <select
            id="text-size"
            className="accessibility-settings-select"
            value={settings.textSize}
            onChange={handleTextSizeChange}
            aria-describedby="text-size-description"
          >
            {TEXT_SIZE_OPTIONS.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </div>
      </section>

      {/* Motion Preferences Section */}
      <section className="accessibility-settings-section">
        <h2>Motion Preferences</h2>

        <div className="accessibility-settings-field">
          <div className="accessibility-settings-control">
            <input
              type="checkbox"
              id="reduce-motion"
              className="accessibility-settings-checkbox"
              checked={settings.reduceMotion}
              onChange={handleReduceMotionChange}
              aria-describedby="reduce-motion-description"
            />
            <div>
              <label htmlFor="reduce-motion" className="accessibility-settings-label">
                Reduce Motion
              </label>
              <span id="reduce-motion-description" className="accessibility-settings-description">
                Minimize animations and transitions that may cause discomfort or distraction.
              </span>
            </div>
          </div>
        </div>
      </section>

      {/* Navigation Preferences Section */}
      <section className="accessibility-settings-section">
        <h2>Navigation Preferences</h2>

        <div className="accessibility-settings-field">
          <div className="accessibility-settings-control">
            <input
              type="checkbox"
              id="screen-reader"
              className="accessibility-settings-checkbox"
              checked={settings.screenReaderEnabled}
              onChange={handleScreenReaderChange}
              aria-describedby="screen-reader-description"
            />
            <div>
              <label htmlFor="screen-reader" className="accessibility-settings-label">
                Screen Reader Mode
              </label>
              <span id="screen-reader-description" className="accessibility-settings-description">
                Optimize the interface for screen reader users with enhanced ARIA labels and
                descriptions.
              </span>
            </div>
          </div>
        </div>

        <div className="accessibility-settings-field">
          <div className="accessibility-settings-control">
            <input
              type="checkbox"
              id="keyboard-nav"
              className="accessibility-settings-checkbox"
              checked={settings.keyboardNavigationEnabled}
              onChange={handleKeyboardNavChange}
              aria-describedby="keyboard-nav-description"
            />
            <div>
              <label htmlFor="keyboard-nav" className="accessibility-settings-label">
                Enhanced Keyboard Navigation
              </label>
              <span id="keyboard-nav-description" className="accessibility-settings-description">
                Display enhanced focus indicators for keyboard-only navigation.
              </span>
            </div>
          </div>
        </div>
      </section>

      {/* Preview Section */}
      <section className="accessibility-preview" aria-label={t('aria.settingsPreview')}>
        <h3>Preview</h3>
        <p>
          This is a preview of how text will appear with your current settings. The quick brown fox
          jumps over the lazy dog.
        </p>
        <button
          type="button"
          style={{
            padding: '0.5rem 1rem',
            marginTop: '1rem',
            backgroundColor: 'var(--color-primary)',
            color: 'white',
            border: 'none',
            borderRadius: '4px',
            cursor: 'pointer',
          }}
        >
          Sample Button
        </button>
      </section>

      {/* Info Section */}
      <section
        className="accessibility-settings-section"
        aria-label={t('aria.additionalInformation')}
      >
        <h2>Additional Help</h2>
        <p>
          If you experience any accessibility issues or have suggestions for improvement, please
          contact our support team.
        </p>
        <p>
          <strong>Keyboard Shortcuts:</strong>
        </p>
        <ul>
          <li>
            <kbd>Tab</kbd> - Navigate forward through interactive elements
          </li>
          <li>
            <kbd>Shift + Tab</kbd> - Navigate backward through interactive elements
          </li>
          <li>
            <kbd>Enter</kbd> or <kbd>Space</kbd> - Activate buttons and links
          </li>
          <li>
            <kbd>Esc</kbd> - Close dialogs and menus
          </li>
        </ul>
      </section>
    </div>
  );
};

AccessibilitySettingsPage.displayName = 'AccessibilitySettingsPage';
