/**
 * Onboarding and help system module.
 *
 * Epic 50 - Stories 50.1-50.4: Onboarding & Help
 */

// Feedback Manager
export { FeedbackManager, feedbackManager } from './FeedbackManager';
// Help Center
export { HelpCenter, helpCenter } from './HelpCenter';
// Tour Manager
export { TourManager, tourManager } from './TourManager';
// Types
export type {
  AppContext,
  DeviceInfo,
  FAQCategory,
  FAQItem,
  FeedbackSubmission,
  FeedbackType,
  HelpCenterState,
  HelpContent,
  OnboardingState,
  TourConfig,
  TourProgress,
  TourStep,
  Tutorial,
  TutorialStep,
} from './types';
