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
      <h1>{t('accessibility.title')}</h1>
      <p>{t('settings.accessibilityPage.intro')}</p>

      {/* Color Scheme Section */}
      <section className="accessibility-settings-section">
        <h2>{t('settings.accessibilityPage.visualTitle')}</h2>

        <div className="accessibility-settings-field">
          <label htmlFor="color-scheme" className="accessibility-settings-label">
            {t('settings.accessibilityPage.colorScheme')}
          </label>
          <span className="accessibility-settings-description">
            {t('settings.accessibilityPage.colorSchemeHelp')}
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
                {t(`settings.accessibilityPage.colorSchemes.${option.value}`, {
                  defaultValue: option.label,
                })}
              </option>
            ))}
          </select>
        </div>

        <div className="accessibility-settings-field">
          <label htmlFor="text-size" className="accessibility-settings-label">
            {t('settings.accessibilityPage.textSize')}
          </label>
          <span className="accessibility-settings-description">
            {t('settings.accessibilityPage.textSizeHelp')}
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
                {t(`settings.accessibilityPage.textSizes.${option.value}`, {
                  defaultValue: option.label,
                })}
              </option>
            ))}
          </select>
        </div>
      </section>

      {/* Motion Preferences Section */}
      <section className="accessibility-settings-section">
        <h2>{t('settings.accessibilityPage.motionTitle')}</h2>

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
                {t('settings.accessibilityPage.reduceMotion')}
              </label>
              <span id="reduce-motion-description" className="accessibility-settings-description">
                {t('settings.accessibilityPage.reduceMotionHelp')}
              </span>
            </div>
          </div>
        </div>
      </section>

      {/* Navigation Preferences Section */}
      <section className="accessibility-settings-section">
        <h2>{t('settings.accessibilityPage.navigationTitle')}</h2>

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
                {t('settings.accessibilityPage.screenReader')}
              </label>
              <span id="screen-reader-description" className="accessibility-settings-description">
                {t('settings.accessibilityPage.screenReaderHelp')}
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
                {t('settings.accessibilityPage.keyboardNav')}
              </label>
              <span id="keyboard-nav-description" className="accessibility-settings-description">
                {t('settings.accessibilityPage.keyboardNavHelp')}
              </span>
            </div>
          </div>
        </div>
      </section>

      {/* Preview Section */}
      <section className="accessibility-preview" aria-label={t('aria.settingsPreview')}>
        <h3>{t('settings.accessibilityPage.previewTitle')}</h3>
        <p>{t('settings.accessibilityPage.previewText')}</p>
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
          {t('settings.accessibilityPage.sampleButton')}
        </button>
      </section>

      {/* Info Section */}
      <section
        className="accessibility-settings-section"
        aria-label={t('aria.additionalInformation')}
      >
        <h2>{t('settings.accessibilityPage.helpTitle')}</h2>
        <p>{t('settings.accessibilityPage.helpText')}</p>
        <p>
          <strong>{t('settings.accessibilityPage.shortcutsTitle')}</strong>
        </p>
        <ul>
          <li>
            <kbd>Tab</kbd> – {t('settings.accessibilityPage.shortcutTab')}
          </li>
          <li>
            <kbd>Shift + Tab</kbd> – {t('settings.accessibilityPage.shortcutShiftTab')}
          </li>
          <li>
            <kbd>Enter</kbd> / <kbd>Space</kbd> – {t('settings.accessibilityPage.shortcutEnter')}
          </li>
          <li>
            <kbd>Esc</kbd> – {t('settings.accessibilityPage.shortcutEsc')}
          </li>
        </ul>
      </section>
    </div>
  );
};

AccessibilitySettingsPage.displayName = 'AccessibilitySettingsPage';
