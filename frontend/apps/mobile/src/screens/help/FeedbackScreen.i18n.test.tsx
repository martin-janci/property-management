/**
 * FeedbackScreen — device-info i18n regression test
 * (code-review-mobile-rn-feedback-device-info-i18n).
 *
 * The device-info preview rendered its three strings as hardcoded English
 * literals even though react-i18next was already wired for the rest of the
 * screen:
 *   - "Platform: <platform> <osVersion>"
 *   - "App: v<appVersion> (Build <buildNumber>)"
 *   - the "Diagnostic Report" alert title
 * This suite locks the contract: every device-info string is now sourced from
 * an i18n key via `t()`, not a literal.
 *
 * Mirrors OutagesScreen.i18n.test.tsx:
 *  1. Render layer — `src/test/setup.ts` mocks react-i18next so `t(key) => key`.
 *     We assert the screen renders the i18n KEYS and that the OLD hardcoded
 *     English literals are gone.
 *  2. Resource layer — the locale JSON files carry the keys (with their
 *     interpolation placeholders) and locale-distinct values.
 */

import { render, screen } from '@testing-library/react-native';
import cs from '../../locales/cs.json';
import de from '../../locales/de.json';
import en from '../../locales/en.json';
import hu from '../../locales/hu.json';
import pl from '../../locales/pl.json';
import sk from '../../locales/sk.json';
import { FeedbackScreen } from './FeedbackScreen';

const DEVICE_INFO = {
  platform: 'ios' as const,
  osVersion: '17.4',
  appVersion: '1.2.3',
  buildNumber: '456',
};

const FEEDBACK_TYPES = ['bug', 'feature', 'improvement', 'question', 'other'] as const;

jest.mock('../../onboarding', () => ({
  feedbackManager: {
    updateContext: jest.fn(),
    // Feedback-type labels are i18n KEYS resolved by the screen via `t()`,
    // never hardcoded English literals (code-review-mobile-rn-feedback-types-i18n).
    getFeedbackTypes: jest.fn(() => [
      { type: 'bug', labelKey: 'feedback.types.bug', icon: '🐛' },
      { type: 'feature', labelKey: 'feedback.types.feature', icon: '💡' },
      { type: 'improvement', labelKey: 'feedback.types.improvement', icon: '✨' },
      { type: 'question', labelKey: 'feedback.types.question', icon: '❓' },
      { type: 'other', labelKey: 'feedback.types.other', icon: '📝' },
    ]),
    getDeviceInfo: jest.fn(() => DEVICE_INFO),
    generateDiagnosticReport: jest.fn(() => 'diagnostic-report'),
    createFeedback: jest.fn(),
    submitFeedback: jest.fn(),
    saveDraft: jest.fn(),
  },
}));

describe('FeedbackScreen device-info i18n', () => {
  it('renders device-info keys, not hardcoded literals', async () => {
    await render(<FeedbackScreen onNavigate={jest.fn()} />);
    // includeDeviceInfo defaults to true, so the preview is rendered on mount.
    expect(screen.getByText('feedback.devicePlatform')).toBeTruthy();
    expect(screen.getByText('feedback.deviceAppVersion')).toBeTruthy();
  });

  it('no longer renders the old hardcoded English device-info literals', async () => {
    await render(<FeedbackScreen onNavigate={jest.fn()} />);
    expect(screen.queryByText(/^Platform:/)).toBeNull();
    expect(screen.queryByText(/^App: v/)).toBeNull();
  });
});

describe('feedback device-info keys resolve to real, locale-distinct translations', () => {
  it('en carries the expected English copy with interpolation placeholders', () => {
    expect(en.feedback.devicePlatform).toBe('Platform: {{platform}} {{osVersion}}');
    expect(en.feedback.deviceAppVersion).toBe('App: v{{version}} (Build {{build}})');
    expect(en.feedback.diagnosticReportTitle).toBe('Diagnostic Report');
  });

  it('every locale defines the keys with their interpolation placeholders', () => {
    for (const locale of [en, sk, cs, de, pl, hu]) {
      expect(locale.feedback.devicePlatform).toContain('{{platform}}');
      expect(locale.feedback.devicePlatform).toContain('{{osVersion}}');
      expect(locale.feedback.deviceAppVersion).toContain('{{version}}');
      expect(locale.feedback.deviceAppVersion).toContain('{{build}}');
      expect(locale.feedback.diagnosticReportTitle.length).toBeGreaterThan(0);
    }
  });

  it('translates the diagnostic-report title away from English in every other locale', () => {
    for (const locale of [sk, cs, de, pl, hu]) {
      expect(locale.feedback.diagnosticReportTitle).not.toBe(en.feedback.diagnosticReportTitle);
    }
  });
});

describe('FeedbackScreen feedback-type i18n', () => {
  it('renders feedback-type label KEYS, not hardcoded English literals', async () => {
    await render(<FeedbackScreen onNavigate={jest.fn()} />);
    // The i18next mock resolves `t(key) => key`, so a localized label surfaces
    // as its key. If the screen still rendered the manager's old English
    // `label` field, these key lookups would fail.
    for (const type of FEEDBACK_TYPES) {
      expect(screen.getByText(`feedback.types.${type}`)).toBeTruthy();
    }
  });

  it('no longer renders the old hardcoded English feedback-type literals', async () => {
    await render(<FeedbackScreen onNavigate={jest.fn()} />);
    expect(screen.queryByText('Bug Report')).toBeNull();
    expect(screen.queryByText('Feature Request')).toBeNull();
  });
});

describe('feedback-type keys resolve to real, locale-distinct translations', () => {
  it('en carries the expected English copy', () => {
    expect(en.feedback.types.bug).toBe('Bug Report');
    expect(en.feedback.types.feature).toBe('Feature Request');
    expect(en.feedback.types.improvement).toBe('Improvement');
    expect(en.feedback.types.question).toBe('Question');
    expect(en.feedback.types.other).toBe('Other');
  });

  it('every locale defines a non-empty label for each feedback type', () => {
    for (const locale of [en, sk, cs, de, pl, hu]) {
      for (const type of FEEDBACK_TYPES) {
        expect(locale.feedback.types[type].length).toBeGreaterThan(0);
      }
    }
  });

  it('translates each feedback-type label away from English in every other locale', () => {
    for (const locale of [sk, cs, de, pl, hu]) {
      for (const type of FEEDBACK_TYPES) {
        expect(locale.feedback.types[type]).not.toBe(en.feedback.types[type]);
      }
    }
  });
});
